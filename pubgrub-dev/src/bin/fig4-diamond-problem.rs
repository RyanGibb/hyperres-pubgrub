use pubgrub::{resolve, DefaultStringReporter, OfflineDependencyProvider, PubGrubError, Ranges, Reporter};

fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, Ranges<u32>>::new();

    dependency_provider.add_dependencies("A", 1u32, [("B", Ranges::singleton(1u32)), ("C", Ranges::singleton(1u32))]);
    dependency_provider.add_dependencies("B", 1u32, [("D", Ranges::singleton(1u32))]);
    dependency_provider.add_dependencies("C", 1u32, [("D", Ranges::singleton(3u32))]);
    dependency_provider.add_dependencies("D", 1u32, []);
    dependency_provider.add_dependencies("D", 3u32, []);

    match resolve(&dependency_provider, "A", 1u32) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_no_versions();
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
        }
        Err(err) => panic!("{:?}", err),
    };
}
