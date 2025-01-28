use pubgrub::{resolve, DefaultStringReporter, OfflineDependencyProvider, PubGrubError, Ranges, Reporter};

fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, Ranges<u32>>::new();

    dependency_provider.add_dependencies("A", 1u32, [
        // proxy package
        ("&", Ranges::singleton(0u32)),
        ("&", Ranges::singleton(1u32)),
    ]);
    dependency_provider.add_dependencies("&", 0u32, [
        ("B", Ranges::singleton(1u32)),
        ("C", Ranges::singleton(1u32)),
    ]);
    dependency_provider.add_dependencies("&", 1u32, [
        ("B", Ranges::singleton(2u32)),
        // we can't depict the conflict with C
    ]);
    dependency_provider.add_dependencies("B", 1u32, []);
    dependency_provider.add_dependencies("B", 2u32, []);
    dependency_provider.add_dependencies("C", 1u32, []);

    let sol = match resolve(&dependency_provider, "A", 1u32) {
        Ok(sol) => sol,
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_no_versions();
            panic!("{}", DefaultStringReporter::report(&derivation_tree));
        }
        Err(err) => panic!("{:?}", err),
    };

    println!("{:?}", sol)
}
