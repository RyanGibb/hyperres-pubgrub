use pubgrub::{resolve, DefaultStringReporter, OfflineDependencyProvider, PubGrubError, Ranges, Reporter};

fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, Ranges<String>>::new();

    dependency_provider.add_dependencies("A", "1-x86_64", [("Arch", Ranges::singleton("x86_64"))],);
    dependency_provider.add_dependencies("A", "1-ARM64", [("Arch", Ranges::singleton("ARM64"))],);
    dependency_provider.add_dependencies("A", "1-i386", [("Arch", Ranges::singleton("i386"))],);
    dependency_provider.add_dependencies("Arch", "x86_64", []);
    dependency_provider.add_dependencies("Arch", "ARM64", []);
    dependency_provider.add_dependencies("Arch", "i386", []);
    dependency_provider.add_dependencies("root", "", [
        ("A", Ranges::union(&Ranges::singleton("1-x86_64"), &Ranges::union(&Ranges::singleton("1-ARM64"), &Ranges::singleton("1-i386"))))
    ]);

    match resolve(&dependency_provider, "root", "") {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_no_versions();
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
        }
        Err(err) => panic!("{:?}", err),
    };
}
