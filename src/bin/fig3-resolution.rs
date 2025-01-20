use pubgrub::{resolve, DependencyProvider, Dependencies, OfflineDependencyProvider, Ranges};
use std::collections::HashMap;

fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, Ranges<u32>>::new();

    dependency_provider.add_dependencies("A", 1u32, [("B", Ranges::singleton(1u32)), ("C", Ranges::singleton(1u32))]);
    dependency_provider.add_dependencies("B", 1u32, [("D", Ranges::union(&Ranges::singleton(1u32), &Ranges::singleton(2u32)) )]);
    dependency_provider.add_dependencies("C", 1u32, [("D", Ranges::union(&Ranges::singleton(2u32), &Ranges::singleton(3u32)) )]);
    dependency_provider.add_dependencies("D", 1u32, []);
    dependency_provider.add_dependencies("D", 2u32, []);
    dependency_provider.add_dependencies("D", 3u32, []);

    let sol = resolve(&dependency_provider, "A", 1u32).unwrap();

    println!("{:?}", sol);

    let mut resolved_graph: HashMap<_, Vec<_>> = HashMap::new();
    for (package, version) in &sol {
        let dependencies = dependency_provider.get_dependencies(&package, &version);
        match dependencies {
            Ok(Dependencies::Available(constraints)) => {
                let mut dependents = Vec::new();
                for (dep_package, dep_versions) in constraints {
                    let solved_version = sol.get(dep_package).unwrap();
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
    for (package, dependents) in resolved_graph {
        println!("{:?} -> {:?}", package, dependents);
    }
}
