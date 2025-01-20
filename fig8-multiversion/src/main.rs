use core::ops::RangeFull;

use hyperres_pubgrub_fig8_multiversion::index::Index;
use hyperres_pubgrub_fig8_multiversion::multiple_versions::Package;
use pubgrub::type_aliases::SelectedDependencies;
use pubgrub::version::SemanticVersion as SemVer;
use std::collections::HashMap;
use pubgrub::solver::{Dependencies, DependencyProvider};
use std::str::FromStr;

fn main() {
    let mut index = Index::new();
    index.add_deps("a", (1, 0, 0), &[("b", (1, 0, 0)..(2, 0, 0))]);
    index.add_deps("a", (1, 0, 0), &[("c", (1, 0, 0)..(2, 0, 0))]);
    index.add_deps("b", (1, 0, 0), &[("d", (1, 0, 0)..(2, 0, 0))]);
    index.add_deps("c", (1, 0, 0), &[("d", (3, 0, 0)..(4, 0, 0))]);
    index.add_deps::<RangeFull>("d", (1, 0, 0), &[]);
    index.add_deps::<RangeFull>("d", (3, 0, 0), &[]);

    let pkg = Package::from_str("a#1").unwrap();
    let sol: SelectedDependencies<Package, SemVer> = pubgrub::solver::resolve(&index, pkg, (1, 0, 0)).map(|solution| {
        // remove proxy packages from the solution
        solution
            .into_iter()
            .filter(|(pkg, _)| match pkg {
                Package::Bucket(_) => true,
                _ => false,
            })
            .collect()
    }).unwrap();

    println!("{:?}", sol);

    let mut resolved_graph: HashMap<_, Vec<_>> = HashMap::new();
    for (package, version) in &sol {
        let dependencies = index.get_dependencies(&package, &version);
        match dependencies {
            Ok(Dependencies::Known(constraints)) => {
                let mut dependents = Vec::new();
                for (dep_package, dep_versions) in constraints {
                    let solved_version = sol.get(&dep_package).unwrap();
                    if dep_versions.contains(&solved_version) {
                        dependents.push((dep_package, solved_version));
                    }
                }
                resolved_graph.insert((package, version), dependents);
            }
            _ => {
                println!("No available dependencies for package {}", package);
            }
        }
    }

    println!("Resolved Dependency Graph:");
    for ((package, version), dependents) in resolved_graph {
        match package {
            Package::Bucket(bucket) => {
                print!("({}, {}) -> ", bucket.name, version);
            }
            Package::Proxy{ source, target } => {}
        }
        for (package, version) in &dependents {
            match package {
                Package::Bucket(bucket) => {
                    print!("({}, {}), ", bucket.name, version);
                }
                Package::Proxy { source, target } => {}
            }
        }
        println!()
    }
}
