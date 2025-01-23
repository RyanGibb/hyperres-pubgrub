
use core::ops::RangeFull;

use hyperres_pubgrub_fig9_features::index::Index;
use hyperres_pubgrub_fig9_features::optional_deps::Package;
use std::collections::HashMap;
use pubgrub::solver::{Dependencies, DependencyProvider};
use std::str::FromStr;

fn main() {
    let mut index = Index::new();
    index.add_deps("a", 1, &[("b", .., &[])]);
    index.add_deps("a", 1, &[("c", .., &[])]);
    index.add_deps("c", 1, &[("d", .., &["alpha"])]);
    index.add_deps("b", 1, &[("d", .., &["beta"])]);
    index.add_feature::<RangeFull>("d", 1, "alpha", &[]);
    index.add_feature::<RangeFull>("d", 1, "beta", &[]);

    let pkg = Package::from_str("a").unwrap();
    let sol = pubgrub::solver::resolve(&index, pkg, 1).unwrap();

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
            Package::Base(base) => {
                print!("({}, {}) -> ", base, version);
            }
            Package::Feature { base, feature } => {
                print!("({}/{}, {}) -> ", base, feature, version);
            }
        };
        for (package, version) in &dependents {
            match package {
                Package::Base(base) => {
                    print!("({}, {}), ", base, version);
                }
                Package::Feature { base, feature } => {
                    print!("({}/{}, {}), ", base, feature, version);
                }
            }
        }
        println!()
    }
}
