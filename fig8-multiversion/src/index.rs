// SPDX-License-Identifier: MPL-2.0
// https://github.com/pubgrub-rs/advanced_dependency_providers/

use core::ops::{Bound, RangeBounds};
use pubgrub::range::Range;
use pubgrub::type_aliases::Map;
use pubgrub::version::SemanticVersion as SemVer;
use std::collections::BTreeMap;

/// Each package is identified by its name.
pub type PackageName = String;
/// Alias for dependencies.
pub type Deps = Map<PackageName, Range<SemVer>>;

/// Global registry of known packages.
pub struct Index {
    /// Specify dependencies of each package version.
    pub packages: Map<PackageName, BTreeMap<SemVer, Deps>>,
}

impl Index {
    /// Empty new index.
    pub fn new() -> Self {
        Self {
            packages: Map::default(),
        }
    }

    /// List existing versions for a given package with newest versions first.
    pub fn available_versions(&self, package: &PackageName) -> impl Iterator<Item = &SemVer> {
        self.packages
            .get(package)
            .into_iter()
            .flat_map(|k| k.keys())
            .rev()
    }

    /// Register a package and its dependencies in the index.
    pub fn add_deps<R: RangeBounds<(u32, u32, u32)>>(
        &mut self,
        package: &str,
        version: (u32, u32, u32),
        deps: &[(&str, R)],
    ) {
        let index_deps = self
            .packages
            .entry(package.to_string())
            .or_default()
            .entry(version.into())
            .or_default();
        for (p, r) in deps {
            index_deps.insert(String::from(*p), range_from_bounds(r));
        }
    }
}

/// Convert a range bounds into pubgrub Range type.
fn range_from_bounds<R: RangeBounds<(u32, u32, u32)>>(bounds: &R) -> Range<SemVer> {
    match (bounds.start_bound(), bounds.end_bound()) {
        (Bound::Unbounded, Bound::Unbounded) => Range::any(),
        (Bound::Unbounded, Bound::Excluded(end)) => Range::strictly_lower_than(*end),
        (Bound::Unbounded, Bound::Included(end)) => Range::strictly_lower_than(bump(end)),
        (Bound::Included(start), Bound::Unbounded) => Range::higher_than(*start),
        (Bound::Included(start), Bound::Included(end)) => Range::between(*start, bump(end)),
        (Bound::Included(start), Bound::Excluded(end)) => Range::between(*start, *end),
        (Bound::Excluded(start), Bound::Unbounded) => Range::higher_than(bump(start)),
        (Bound::Excluded(start), Bound::Included(end)) => Range::between(bump(start), bump(end)),
        (Bound::Excluded(start), Bound::Excluded(end)) => Range::between(bump(start), *end),
    }
}

fn bump(v: &(u32, u32, u32)) -> (u32, u32, u32) {
    SemVer::from(*v).bump_patch().into()
}
