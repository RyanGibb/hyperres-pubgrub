
use core::ops::RangeFull;

use hyperres_pubgrub_multiversion_features::index::Index;
use hyperres_pubgrub_multiversion_features::multiversion_optional_deps::Package;
use std::collections::HashMap;
use pubgrub::{error::PubGrubError, report::{DefaultStringReporter, Reporter}, solver::{Dependencies, DependencyProvider}, type_aliases::SelectedDependencies};
use pubgrub::version::SemanticVersion as SemVer;
use std::str::FromStr;

fn main() {
    let mut index = Index::new();
    index.add_deps("a", (1, 0, 0), &[("b", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_deps("a", (1, 0, 0), &[("c", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_deps("c", (1, 0, 0), &[("d", (1, 0, 0)..(2, 0, 0), &["alpha"])]);
    index.add_deps("b", (1, 0, 0), &[("d", (2, 0, 0)..(3, 0, 0), &["beta"])]);
    index.add_feature::<RangeFull>("d", (1, 0, 0), "alpha", &[]);
    index.add_feature::<RangeFull>("d", (1, 0, 0), "beta", &[]);
    index.add_feature::<RangeFull>("d", (2, 0, 0), "alpha", &[]);
    index.add_feature::<RangeFull>("d", (2, 0, 0), "beta", &[]);
    index.add_feature::<RangeFull>("d", (3, 0, 0), "alpha", &[]);
    index.add_feature::<RangeFull>("d", (3, 0, 0), "beta", &[]);
    index.add_feature::<RangeFull>("d", (4, 0, 0), "alpha", &[]);
    index.add_feature::<RangeFull>("d", (4, 0, 0), "beta", &[]);

    let pkg = Package::from_str("a#1").unwrap();
    let sol : SelectedDependencies<Package, SemVer> = match pubgrub::solver::resolve(&index, pkg, (1, 0, 0)) {
        Ok(sol) => sol,
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_no_versions();
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
            panic!("failed to find a solution");
        },
        Err(err) => panic!("{:?}", err),
    };

    println!("{:?}", sol);

    let mut resolved_graph: HashMap<_, Vec<_>> = HashMap::new();
    let mut selected_features: HashMap<_, Vec<_>> = HashMap::new();
    for (package, version) in &sol {
        let dependencies = index.get_dependencies(&package, &version);
        match dependencies {
            Ok(Dependencies::Known(constraints)) => {
                let sol: &HashMap<Package, SemVer, std::hash::BuildHasherDefault<rustc_hash::FxHasher>> = &sol;
                let mut dependents = Vec::new();
                for (dep_package, _dep_versions) in constraints {
                    let solved_version = sol.get(&dep_package).unwrap();
                    let dep_name = match dep_package {
                        Package::Bucket(bucket) => bucket.name,
                        Package::Feature { base, feature : _} => base.name,
                        Package::Proxy{ source : _, target, feature : _ } => target,
                    };
                    dependents.push((dep_name, solved_version));
                };
                match package {
                    Package::Feature { base, feature } => {
                        selected_features.entry((base.name.clone(), version)).or_insert_with(Vec::new).push(feature);
                    },
                    Package::Bucket(bucket) => {
                        resolved_graph.insert((bucket.name.clone(), version), dependents);
                    },
                    Package::Proxy{ source : _, target : _, feature : _} => {},
               }
            }
            _ => {
                println!("No available dependencies for package {}", package);
            }
        }
    }

    println!("Resolved Dependency Graph:");
    for ((name, version), dependents) in resolved_graph {
        print!("({}, {}", name, version);
        match selected_features.get(&(name, version)) {
            Some(features) => print!(", {:?})", features),
            None => print!(")"),
        }
        if dependents.len() > 0 {
            print!(" -> ")
        }
        let mut first = true;
        for (dep_name, dep_version) in &dependents {
            if !first {
                print!(", ");
            }
            print!("({}, {})", dep_name, dep_version);
            first = false;
        }
        println!()
    }
}
