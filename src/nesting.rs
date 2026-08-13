// Overlapping-target bookkeeping for WD-40 sizing.
// Nesting is decided by paths alone, so it is known before a single byte is
// measured; that is what lets a size be published the moment it lands instead
// of after the whole scan. Exports: `Publisher`.
// Deps: std only.

use std::path::{Path, PathBuf};

/// Tracks which measured sizes are final. A target that encloses another is
/// only final once that other has been measured, because its own figure has to
/// have the descendant subtracted out of it first.
pub struct Publisher {
    /// Nearest enclosing target, if any.
    parent: Vec<Option<usize>>,
    /// Targets directly enclosed by this one.
    children: Vec<Vec<usize>>,
    /// Enclosed targets not measured yet.
    unmeasured: Vec<usize>,
    measured: Vec<Option<u64>>,
}

impl Publisher {
    pub fn new(paths: &[PathBuf]) -> Self {
        let parent: Vec<Option<usize>> = (0..paths.len())
            .map(|index| nearest_ancestor(paths, index))
            .collect();
        let mut children = vec![Vec::new(); paths.len()];
        for (index, owner) in parent.iter().enumerate() {
            if let Some(owner) = owner {
                children[*owner].push(index);
            }
        }
        let unmeasured = children.iter().map(Vec::len).collect();
        Self {
            parent,
            children,
            unmeasured,
            measured: vec![None; paths.len()],
        }
    }

    /// Record one raw measurement and return every target that is now final,
    /// paired with the size that excludes anything nested inside it.
    pub fn record(&mut self, index: usize, bytes: u64) -> Vec<(usize, u64)> {
        self.measured[index] = Some(bytes);
        let mut settled = Vec::new();
        if self.unmeasured[index] == 0 {
            settled.push(index);
        }
        if let Some(parent) = self.parent[index] {
            self.unmeasured[parent] -= 1;
            if self.unmeasured[parent] == 0 && self.measured[parent].is_some() {
                settled.push(parent);
            }
        }
        settled.into_iter().map(|i| (i, self.exclusive(i))).collect()
    }

    /// Bytes this target holds that no target inside it also claims. Only direct
    /// children are subtracted: a grandchild's bytes are already inside its own
    /// parent's figure, so taking it off again would charge it twice.
    fn exclusive(&self, index: usize) -> u64 {
        self.children[index]
            .iter()
            .filter_map(|child| self.measured[*child])
            .fold(self.measured[index].unwrap_or(0), u64::saturating_sub)
    }
}

/// The longest other path that contains `paths[index]`. Equal paths never
/// enclose one another, so a duplicate entry keeps its own full size.
fn nearest_ancestor(paths: &[PathBuf], index: usize) -> Option<usize> {
    let target: &Path = &paths[index];
    paths
        .iter()
        .enumerate()
        .filter(|(other, path)| {
            *other != index && path.as_path() != target && target.starts_with(path)
        })
        .max_by_key(|(_, path)| path.components().count())
        .map(|(other, _)| other)
}

#[cfg(test)]
mod tests {
    use super::Publisher;
    use std::path::PathBuf;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn an_unrelated_target_is_final_the_moment_it_lands() {
        let mut publisher = Publisher::new(&paths(&["/a/target", "/b/target"]));
        assert_eq!(publisher.record(0, 100), [(0, 100)]);
        assert_eq!(publisher.record(1, 40), [(1, 40)]);
    }

    #[test]
    fn an_enclosing_target_waits_for_what_is_inside_it() {
        // /root measured first: nothing may be published, because the figure
        // still has the session's bytes in it.
        let mut publisher = Publisher::new(&paths(&["/root", "/root/session"]));
        assert!(publisher.record(0, 100).is_empty());
        assert_eq!(publisher.record(1, 30), [(1, 30), (0, 70)]);
    }

    #[test]
    fn a_child_measured_first_releases_its_parent_immediately() {
        let mut publisher = Publisher::new(&paths(&["/root", "/root/session"]));
        assert_eq!(publisher.record(1, 30), [(1, 30)]);
        assert_eq!(publisher.record(0, 100), [(0, 70)]);
    }

    #[test]
    fn a_chain_charges_each_layer_once() {
        // /a contains /a/b contains /a/b/c. /a's own figure already includes
        // both, and /a/b's includes /a/b/c, so each level subtracts only the
        // level directly beneath it.
        let mut publisher = Publisher::new(&paths(&["/a", "/a/b", "/a/b/c"]));
        assert_eq!(publisher.record(2, 10), [(2, 10)]);
        assert_eq!(publisher.record(1, 40), [(1, 30)]);
        assert_eq!(publisher.record(0, 100), [(0, 60)]);
    }

    #[test]
    fn two_siblings_both_come_off_their_parent() {
        let mut publisher = Publisher::new(&paths(&["/a", "/a/x", "/a/y"]));
        assert!(publisher.record(0, 100).is_empty());
        assert_eq!(publisher.record(1, 20), [(1, 20)]);
        assert_eq!(publisher.record(2, 30), [(2, 30), (0, 50)]);
    }

    #[test]
    fn a_sibling_directory_is_not_an_ancestor() {
        // /a/target-extra starts with the *string* /a/target but not the path.
        let mut publisher = Publisher::new(&paths(&["/a/target", "/a/target-extra"]));
        assert_eq!(publisher.record(0, 100), [(0, 100)]);
        assert_eq!(publisher.record(1, 30), [(1, 30)]);
    }
}
