
use core::ops::RangeFull;

use hyperres_pubgrub_multiversion_features::index::Index;
use hyperres_pubgrub_multiversion_features::multiversion_optional_deps::Package;
use std::collections::{HashMap, HashSet, VecDeque};
use pubgrub::{error::PubGrubError, report::{DefaultStringReporter, Reporter}, solver::{Dependencies, DependencyProvider}, type_aliases::SelectedDependencies};
use pubgrub::version::SemanticVersion as SemVer;
use std::str::FromStr;

fn main() {
    let mut index = Index::new();
    index.add_deps("a", (1, 0, 0), &[("b", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_deps("a", (1, 0, 0), &[("c", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_deps("c", (1, 0, 0), &[("d", (1, 0, 0)..(3, 0, 0), &["alpha"])]);
    index.add_deps("b", (1, 0, 0), &[("d", (2, 0, 0)..(4, 0, 0), &["beta"])]);
    index.add_feature("d", (1, 0, 0), "alpha", &[("e", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_feature("d", (1, 0, 0), "beta", &[("f", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_feature("d", (2, 0, 0), "alpha", &[("e", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_feature("d", (2, 0, 0), "beta", &[("f", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_feature("d", (3, 0, 0), "alpha", &[("e", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_feature("d", (3, 0, 0), "beta", &[("f", (1, 0, 0)..(2, 0, 0), &[])]);
    index.add_deps::<RangeFull>("e", (1, 0, 0), &[]);
    index.add_deps::<RangeFull>("f", (1, 0, 0), &[]);

    let pkg = Package::from_str("a#1").unwrap();

    let mut visited : HashSet<Package> = HashSet::new();
    let mut queue : VecDeque<Package> = VecDeque::new();

    queue.push_back(pkg.clone());

    while let Some(package) = queue.pop_front() {
        if visited.contains(&package) {
            continue;
        }
        visited.insert(package.clone());
        for version in index.list_versions(&package) {
            print!("({}, {})", package, version);
            let mut first = true;
            if let Ok(Dependencies::Known(deps)) = index.get_dependencies(&package, &version) {
                for (dep_package, dep_version) in deps {
                    if first {
                        print!(" -> ")
                    } else {
                        print!(", ");
                    }
                    print!("({}, {})", dep_package, dep_version);
                    first = false;
                    if !visited.contains(&dep_package) {
                        queue.push_back(dep_package);
                    }
                }
            }
            println!();
        }
    }

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
