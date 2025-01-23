// SPDX-License-Identifier: MPL-2.0
// https://github.com/pubgrub-rs/advanced_dependency_providers/

use crate::index::{Dep, Index};
use core::borrow::Borrow;
use core::fmt::Display;
use itertools::Either;
use pubgrub::range::Range;
use pubgrub::solver::{Dependencies, DependencyConstraints, DependencyProvider};
use pubgrub::type_aliases::Map;
use pubgrub::version::SemanticVersion as SemVer;
use std::str::FromStr;

/// A package is either a bucket, or a proxy between two packages.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Package {
    /// "a#1"
    Bucket(Bucket),
    /// source -> target
    Proxy {
        source: (Bucket, SemVer),
        target: String,
        feature: Option<String>
    },
    Feature { base: Bucket, feature: String },
}

/// A bucket corresponds to a given package, and match versions in a range identified by their
/// major component.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Bucket {
    pub name: String,
    pub bucket: u32, // 1 maps to the range 1.0.0 <= v < 2.0.0
}

impl Package {
    fn pkg_name(&self) -> &String {
        match self {
            Package::Bucket(b) => &b.name,
            Package::Proxy { source, .. } => &source.0.name,
            Package::Feature { base, .. } => &base.name,
        }
    }
}

impl FromStr for Package {
    type Err = String;
    /// "a#1" -> Package::Bucket
    fn from_str(pkg: &str) -> Result<Self, Self::Err> {
        let mut pkg_parts = pkg.split('#');
        match (pkg_parts.next(), pkg_parts.next()) {
            (Some(name), Some(version)) => {
                let mut pkg_parts = version.split('/');
                match (pkg_parts.next(), pkg_parts.next()) {
                    (Some(bucket), None) =>
                        Ok(Package::Bucket(Bucket {
                            name: name.to_string(),
                            bucket: bucket.parse().unwrap(),
                        })),
                    (Some(bucket), Some(feat)) => Ok(Package::Feature {
                        base: Bucket {
                            name: name.to_string(),
                            bucket: bucket.parse().unwrap(),
                        },
                        feature: feat.to_string(),
                    }),
                    _ => Err(format!("{} is not a valid package name", pkg)),
                }

            },
            _ => Err(format!("{} is not a valid package name", pkg)),
        }
    }
}

impl Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Package::Bucket(pkg) => write!(f, "{}", pkg),
            Package::Proxy { source, target, feature } =>
                match feature {
                    None => write!(f, "{}@{}->{}", source.0, source.1, target),
                    Some(feat) => write!(f, "{}@{}/{}->{}", source.0, source.1, feat, target),
                },
            Package::Feature { base, feature } => write!(f, "{}/{}", base, feature),
        }
    }
}

impl Display for Bucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.name, self.bucket)
    }
}

impl Index {
    /// List existing versions for a given package with newest versions first.
    pub fn list_versions(&self, package: &Package) -> impl Iterator<Item = SemVer> + '_ {
        match package {
            // If we are on a bucket, we need to filter versions
            // to only keep those within the bucket.
            Package::Bucket(p) | Package::Feature { base : p, feature : _} => {
                let bucket_range = Range::between((p.bucket, 0, 0), (p.bucket + 1, 0, 0));
                Either::Left(
                    self.available_versions(&p.name)
                        .filter(move |v| bucket_range.contains(*v))
                        .cloned(),
                )
            }
            // If we are on a proxy, there is one version per bucket in the target package.
            // We can additionally filter versions to only those inside the dependency range.
            Package::Proxy { target, source, feature : _ } => {
                let dep_range = self
                    .packages
                    .get(&source.0.name)
                    .unwrap()
                    .get(&source.1)
                    .unwrap()
                    // TODO
                    .mandatory
                    .get(target)
                    .unwrap();
                Either::Right(bucket_versions(
                    self.available_versions(&target)
                        .filter(move |v| dep_range.range.contains(v))
                        .cloned(),
                ))
            }
        }
    }
}

/// Take a list of versions, and output a list of the corresponding bucket versions.
/// So [1.1, 1.2, 2.3] -> [1.0, 2.0]
fn bucket_versions(versions: impl Iterator<Item = SemVer>) -> impl Iterator<Item = SemVer> {
    let mut current_bucket = None;
    // This filter_map makes the hypothesis that versions are sorted in a normal or reverse order.
    // Would need a bit more work if they are not ordered due to prioritizations, etc.
    versions.filter_map(move |v| {
        let v_bucket = Some(bucket_version(v));
        if v_bucket != current_bucket {
            current_bucket = v_bucket;
            v_bucket
        } else {
            None
        }
    })
}

fn bucket_version(v: SemVer) -> SemVer {
    let (major, _, _) = v.into();
    (major, 0, 0).into()
}

impl DependencyProvider<Package, SemVer> for Index {
    fn choose_package_version<T: Borrow<Package>, U: Borrow<Range<SemVer>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<SemVer>), Box<dyn std::error::Error>> {
        Ok(pubgrub::solver::choose_package_with_fewest_versions(
            |p| self.list_versions(p),
            potential_packages,
        ))
    }

    fn get_dependencies(
        &self,
        package: &Package,
        version: &SemVer,
    ) -> Result<Dependencies<Package, SemVer>, Box<dyn std::error::Error>> {
        let all_versions = match self.packages.get(package.pkg_name()) {
            None => return Ok(Dependencies::Unknown),
            Some(all_versions) => all_versions,
        };
        let deps = match all_versions.get(version) {
            None => return Ok(Dependencies::Unknown),
            Some(deps) => deps,
        };

        match package {
            Package::Bucket(pkg) => {
                // If we asked for a base package, we return the mandatory dependencies.
                Ok(Dependencies::Known(from_deps(pkg, version, &deps.mandatory)))
            },
            Package::Proxy { source, target, feature } => {
                // If this is a proxy package, it depends on a single bucket package, the target,
                // at a range of versions corresponding to the bucket range of the version asked,
                // intersected with the original dependency range.
                let proxy_deps = match all_versions.get(&source.1) {
                    None => return Ok(Dependencies::Unknown),
                    Some(d) => d,
                };
                let (target_bucket, _, _) = version.clone().into();
                let bucket_range = Range::between((target_bucket, 0, 0), (target_bucket + 1, 0, 0));
                // TODO what if feature package has optional deps
                let target_range = proxy_deps.mandatory.get(target).unwrap();
                let mut deps = Map::default();
                let bucket = Bucket {
                    name: target.clone(),
                    bucket: target_bucket,
                };
                let dep = match feature {
                    None => Package::Bucket(bucket),
                    Some(feat) => Package::Feature { base: bucket, feature : feat.to_string() },
                };
                deps.insert(
                    dep,
                    bucket_range.intersection(&target_range.range),
                );
                Ok(Dependencies::Known(deps))
            }
            // If this is a feature package we concatenate the feature deps with a dependency to the base package.
            Package::Feature { base, feature } => match deps.optional.get(feature) {
                None => Ok(Dependencies::Unknown),
                Some(feature_deps) => {
                    let mut all_deps = from_deps(base, version, feature_deps);
                    all_deps.insert(
                        Package::Bucket(base.clone()),
                        Range::exact(version.clone()),
                    );
                    Ok(Dependencies::Known(all_deps))
                }
            },
        }
    }
}

/// Helper function to convert Index deps into what is expected by the dependency provider.
fn from_deps(pkg: &Bucket, version: &SemVer, deps: &Map<String, Dep>) -> DependencyConstraints<Package, SemVer> {
    deps.iter()
        .flat_map(|(name, dep)| {
            let feature_count = dep.features.len();
            dep.features
                .iter()
                .map(move |feat| {
                    if let Some(bucket) = single_bucket_spanned(&dep.range) {
                        let name = name.clone();
                        let bucket_dep = Bucket { name, bucket };
                        (Package::Feature { base: bucket_dep, feature: feat.clone() }, dep.range.clone())
                    } else {
                        let proxy = Package::Proxy {
                            source: (pkg.clone(), version.clone()),
                            target: name.clone(),
                            feature: Some(feat.to_string()),
                        };
                        (proxy, Range::any())
                    }
                })
                .chain(std::iter::once(
                    if let Some(bucket) = single_bucket_spanned(&dep.range) {
                        let name = name.clone();
                        let bucket_dep = Bucket { name, bucket };
                        (Package::Bucket(bucket_dep), dep.range.clone())
                    } else {
                        let proxy = Package::Proxy {
                            source: (pkg.clone(), version.clone()),
                            target: name.clone(),
                            feature: None
                        };
                        (proxy, Range::any())
                    }
                ))
                // If there was no feature, we take the base package, otherwise, we don't.
                .take(feature_count.max(1))
        })
        .collect()
}

/// If the range is fully contained within one bucket,
/// this returns that bucket identifier.
/// Otherwise, it returns None.
fn single_bucket_spanned(range: &Range<SemVer>) -> Option<u32> {
    range.lowest_version().and_then(|low| {
        let bucket_range = Range::between(low, low.bump_major());
        let bucket_intersect_range = range.intersection(&bucket_range);
        if range == &bucket_intersect_range {
            let (major, _, _) = low.into();
            Some(major)
        } else {
            None
        }
    })
}

// TESTS #######################################################################

// #[cfg(test)]
// pub mod tests {
//     use super::*;
//     use core::fmt::Debug;
//     use pubgrub::error::PubGrubError;
//     use pubgrub::type_aliases::{Map, SelectedDependencies};
//     type R = core::ops::RangeFull;

//     /// Helper function to simplify the tests code.
//     fn resolve(
//         provider: &impl DependencyProvider<Package, SemVer>,
//         pkg: &str,
//         version: u32,
//     ) -> Result<SelectedDependencies<Package, SemVer>, PubGrubError<Package, SemVer>> {
//         let pkg = Package::from_str(pkg).unwrap();
//         pubgrub::solver::resolve(provider, pkg, version)
//     }

//     /// Helper function to build a solution selection.
//     fn select(packages: &[(&str, u32)]) -> SelectedDependencies<Package, SemVer> {
//         packages
//             .iter()
//             .map(|(p, v)| (Package::from_str(p).unwrap(), SemVer::from(*v)))
//             .collect()
//     }

//     /// Helper function to compare a solution to an exact selection of package versions.
//     fn assert_map_eq<K: Eq + std::hash::Hash, V: PartialEq + Debug>(
//         h1: &Map<K, V>,
//         h2: &Map<K, V>,
//     ) {
//         assert_eq!(h1.len(), h2.len());
//         for (k, v) in h1.iter() {
//             assert_eq!(h2.get(k), Some(v));
//         }
//     }

//     #[test]
//     fn success_when_no_feature() {
//         let mut index = Index::new();
//         index.add_deps::<R>("a", 0, &[]);
//         assert_map_eq(&resolve(&index, "a", 0).unwrap(), &select(&[("a", 0)]));
//     }

//     #[test]
//     fn failure_when_missing_feature() {
//         let mut index = Index::new();
//         index.add_deps::<R>("a", 0, &[]);
//         assert!(resolve(&index, "a/missing_feat", 0).is_err());
//     }

//     #[test]
//     fn success_when_feature_with_no_dep() {
//         let mut index = Index::new();
//         index.add_feature::<R>("a", 0, "feat", &[]);
//         assert_map_eq(
//             &resolve(&index, "a/feat", 0).unwrap(),
//             &select(&[("a", 0), ("a/feat", 0)]),
//         );
//     }

//     #[test]
//     fn success_when_feature_with_one_dep() {
//         let mut index = Index::new();
//         index.add_feature("a", 0, "feat", &[("f", .., &[])]);
//         index.add_deps::<R>("f", 0, &[]);
//         assert_map_eq(
//             &resolve(&index, "a/feat", 0).unwrap(),
//             &select(&[("a", 0), ("a/feat", 0), ("f", 0)]),
//         );
//     }

//     #[test]
//     fn success_when_feature_with_two_deps() {
//         let mut index = Index::new();
//         index.add_feature("a", 0, "feat", &[("f1", .., &[]), ("f2", .., &[])]);
//         index.add_deps::<R>("f1", 0, &[]);
//         index.add_deps::<R>("f2", 0, &[]);
//         assert_map_eq(
//             &resolve(&index, "a/feat", 0).unwrap(),
//             &select(&[("a", 0), ("a/feat", 0), ("f1", 0), ("f2", 0)]),
//         );
//     }

//     #[test]
//     fn success_when_transitive_feature() {
//         let mut index = Index::new();
//         index.add_deps("a", 0, &[("b", .., &["feat"])]);
//         index.add_feature("b", 0, "feat", &[("f", .., &[])]);
//         index.add_deps::<R>("f", 0, &[]);
//         assert_map_eq(
//             &resolve(&index, "a", 0).unwrap(),
//             &select(&[("a", 0), ("b", 0), ("b/feat", 0), ("f", 0)]),
//         );
//     }

//     #[test]
//     fn success_when_recursive_feature() {
//         let mut index = Index::new();
//         index.add_deps("a", 0, &[("b", .., &["feat"])]);
//         index.add_feature("b", 0, "feat", &[("f", .., &["rec_feat"])]);
//         index.add_feature::<R>("f", 0, "rec_feat", &[]);
//         assert_map_eq(
//             &resolve(&index, "a", 0).unwrap(),
//             &select(&[
//                 ("a", 0),
//                 ("b", 0),
//                 ("b/feat", 0),
//                 ("f", 0),
//                 ("f/rec_feat", 0),
//             ]),
//         );
//     }

//     #[test]
//     fn success_when_multiple_features() {
//         let mut index = Index::new();
//         index.add_deps("a", 0, &[("b", .., &["feat1", "feat2"])]);
//         index.add_feature("b", 0, "feat1", &[("f1", .., &[])]);
//         index.add_feature("b", 0, "feat2", &[("f2", .., &[])]);
//         index.add_deps::<R>("f1", 0, &[]);
//         index.add_deps::<R>("f2", 0, &[]);
//         assert_map_eq(
//             &resolve(&index, "a", 0).unwrap(),
//             &select(&[
//                 ("a", 0),
//                 ("b", 0),
//                 ("b/feat1", 0),
//                 ("b/feat2", 0),
//                 ("f1", 0),
//                 ("f2", 0),
//             ]),
//         );
//     }

//     #[test]
//     /// b/feat1 and b/feat2 are not available with the same version of b.
//     fn failure_when_different_feature_versions() {
//         let mut index = Index::new();
//         index.add_deps("a", 0, &[("b", .., &["feat1", "feat2"])]);
//         index.add_feature("b", 0, "feat1", &[("f1", .., &[])]);
//         // feat2 is only available for version 1 of b
//         index.add_feature("b", 1, "feat2", &[("f2", .., &[])]);
//         index.add_deps::<R>("f1", 0, &[]);
//         index.add_deps::<R>("f2", 0, &[]);
//         assert!(resolve(&index, "a", 0).is_err());
//     }
// }
