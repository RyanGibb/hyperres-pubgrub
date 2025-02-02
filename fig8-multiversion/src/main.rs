use core::ops::RangeFull;

use hyperres_pubgrub_fig8_multiversion::index::Index;
use hyperres_pubgrub_fig8_multiversion::multiple_versions::Package;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::{error::PubGrubError, type_aliases::SelectedDependencies};
use pubgrub::version::SemanticVersion as SemVer;
use std::collections::HashMap;
use pubgrub::solver::{Dependencies, DependencyProvider};
use std::str::FromStr;

fn main() {
    let mut index = Index::new();
    index.add_deps("a", (1, 0, 0), &[("b", (1, 0, 0)..(2, 0, 0))]);
    index.add_deps("a", (1, 0, 0), &[("c", (1, 0, 0)..(2, 0, 0))]);
    index.add_deps("b", (1, 0, 0), &[("d", (1, 0, 0)..(4, 0, 0))]);
    index.add_deps("c", (1, 0, 0), &[("d", (3, 0, 0)..(4, 0, 0))]);
    index.add_deps::<RangeFull>("d", (1, 0, 0), &[]);
    index.add_deps::<RangeFull>("d", (2, 0, 0), &[]);
    index.add_deps::<RangeFull>("d", (3, 0, 0), &[]);

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
                        Package::Proxy{ source : _, target } => target,
                    };
                    dependents.push((dep_name, solved_version));
                };
                match package {
                    Package::Bucket(bucket) => {
                        resolved_graph.insert((bucket.name.clone(), version), dependents);
                    },
                    Package::Proxy{ source : _, target : _, } => {},
               }
            }
            _ => {
                println!("No available dependencies for package {}", package);
            }
        }
    }

    println!("Resolved Dependency Graph:");
    for ((name, version), dependents) in resolved_graph {
        print!("({}, {})", name, version);
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
