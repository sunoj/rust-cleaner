// What a selection is actually worth, in device bytes rather than in summed
// allocated sizes. Every target goes through the extent map together: sharing
// is a relation between two directories, and one read in isolation cannot tell
// whether the blocks it holds are its own.
// Exports: `Reclaim`.
// Deps: std, crate::{extents, scanner}.

use crate::extents::{attribute, Attribution};
use crate::scanner::TargetDir;

/// Device-byte accounting across the whole target list.
pub struct Reclaim {
    /// Target index behind each owner in `attribution`, in owner order.
    owners: Vec<usize>,
    attribution: Attribution,
}

impl Reclaim {
    /// Read the extent maps of every target.
    ///
    /// Every target, not the ones a path pattern calls clone-prone: `aid`
    /// clones *from* a project's own target directory *into* a session one, so
    /// one end of the sharing is an ordinary `target/` and the other is under
    /// `.cargo-target` or in /tmp. Leaving either end out does not merely miss
    /// the saving — it makes the number wrong in the dangerous direction, by
    /// crediting a selection with blocks a copy outside the set still holds.
    ///
    /// Measured on this Mac: reading only `.cargo-target` reported 61.78 G
    /// against a true 33.52 G, because the largest clone tree of all was a
    /// 40 G directory in /tmp.
    pub fn measure(targets: &[TargetDir]) -> Option<Self> {
        if targets.is_empty() {
            return None;
        }
        let owners: Vec<usize> = (0..targets.len()).collect();
        let paths: Vec<std::path::PathBuf> =
            targets.iter().map(|target| target.path.clone()).collect();
        Some(Self { owners, attribution: attribute(&paths) })
    }

    /// Bytes removing `selected` would return: the shared set measured against
    /// the device, plus the summed sizes of everything that shares nothing.
    ///
    /// A target the accounting has never heard of contributes its own figure,
    /// so a selection made after a rescan is under-promised rather than
    /// over-promised while the next accounting is still running.
    pub fn bytes(&self, targets: &[TargetDir], selected: &impl Fn(usize) -> bool) -> u64 {
        let mut going = vec![false; self.owners.len()];
        for (owner, index) in self.owners.iter().enumerate() {
            going[owner] = selected(*index);
        }
        let shared = self.attribution.union_of(&going);
        targets
            .iter()
            .enumerate()
            .filter(|(index, _)| selected(*index) && !self.owners.contains(index))
            .map(|(_, target)| target.size_bytes)
            .fold(shared, u64::saturating_add)
    }

    /// What the whole list occupies on the device.
    pub fn total_of(&self, targets: &[TargetDir]) -> u64 {
        self.bytes(targets, &|_| true)
    }

    /// True while this accounting still describes the rows on screen. A rescan
    /// that added or dropped a target invalidates the owner indices, and a
    /// stale promise is worse than a conservative one.
    pub fn covers(&self, targets: &[TargetDir]) -> bool {
        self.owners.len() == targets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Reclaim;
    use crate::scanner::{ArtifactKind, TargetDir};
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::SystemTime;

    fn target(path: PathBuf, size_bytes: u64) -> TargetDir {
        TargetDir { path, size_bytes, last_modified: SystemTime::UNIX_EPOCH, kind: ArtifactKind::RustTarget }
    }

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("wd40-reclaim-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn an_empty_list_needs_no_accounting() {
        assert!(Reclaim::measure(&[]).is_none());
    }

    /// The red line, and the bug this replaced: a clone in /tmp is as much a
    /// clone as one under `.cargo-target`. Nothing about where a directory sits
    /// may decide whether its blocks are counted.
    #[test]
    fn sharing_is_found_wherever_the_directories_sit() {
        let root = scratch("anywhere");
        let source = root.join("Develop/proj/target");
        let session = root.join("tmp/cc-target-session");
        let _ = std::fs::create_dir_all(&source);
        let _ = std::fs::create_dir_all(&session);
        let payload = source.join("payload.bin");
        let _ = std::fs::write(&payload, vec![b'x'; 4 << 20]);
        let cloned = std::process::Command::new("cp")
            .arg("-c").arg(&payload).arg(session.join("payload.bin")).status();
        if !cloned.is_ok_and(|status| status.success()) {
            eprintln!("skipped: cp -c unavailable");
            return;
        }

        let targets = vec![target(source, 4 << 20), target(session, 4 << 20)];
        let found = Reclaim::measure(&targets).expect("two targets");

        let four_mb = (4 << 20) as u64;
        assert!(found.total_of(&targets) < four_mb * 2, "the clone is not a second copy");

        // Neither one alone frees anything the other still holds.
        let only_session: HashSet<usize> = [1].into_iter().collect();
        assert_eq!(found.bytes(&targets, &|index| only_session.contains(&index)), 0);
        // Both together free what they share.
        assert!(found.bytes(&targets, &|_| true) >= four_mb);
        let _ = std::fs::remove_dir_all(&root);
    }
}
