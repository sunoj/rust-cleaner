// What a selection is actually worth, in device bytes rather than in summed
// allocated sizes. Only the targets that can share blocks are read through the
// extent map — everything else occupies exactly what it says it does, and
// paying 9x to confirm that would be waste.
// Exports: `Reclaim`.
// Deps: std, crate::{extents, scanner}.

use crate::extents::{attribute, Attribution};
use crate::scanner::TargetDir;
use std::path::Path;

/// A directory seeded by `cp -Rc` shares blocks with whatever it was cloned
/// from. `aid` does that for every session target under this root, and nothing
/// else on the disk is built that way.
fn can_share(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == ".cargo-target")
}

/// Device-byte accounting for the targets that can overlap, alongside the
/// summed sizes of the ones that cannot.
pub struct Reclaim {
    /// Target index behind each owner in `attribution`, in owner order.
    owners: Vec<usize>,
    attribution: Attribution,
}

impl Reclaim {
    /// Read the extent maps of every target that can share blocks. `None` when
    /// none can, which is the common case and costs nothing to find out.
    pub fn measure(targets: &[TargetDir]) -> Option<Self> {
        let owners: Vec<usize> = targets
            .iter()
            .enumerate()
            .filter(|(_, target)| can_share(&target.path))
            .map(|(index, _)| index)
            .collect();
        if owners.is_empty() {
            return None;
        }
        let paths: Vec<std::path::PathBuf> =
            owners.iter().map(|index| targets[*index].path.clone()).collect();
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

    /// True once the targets it was measured against are no longer the ones on
    /// screen, so a stale accounting is dropped rather than believed.
    pub fn covers(&self, targets: &[TargetDir]) -> bool {
        self.owners.iter().all(|index| {
            targets.get(*index).is_some_and(|target| can_share(&target.path))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{can_share, Reclaim};
    use crate::scanner::{ArtifactKind, TargetDir};
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::time::SystemTime;

    fn target(path: &str, size_bytes: u64) -> TargetDir {
        TargetDir {
            path: PathBuf::from(path),
            size_bytes,
            last_modified: SystemTime::UNIX_EPOCH,
            kind: ArtifactKind::RustTarget,
        }
    }

    #[test]
    fn only_the_clone_seeded_root_can_share() {
        assert!(can_share(Path::new("/Users/x/.cargo-target/proj/session-a")));
        assert!(!can_share(Path::new("/Users/x/Develop/proj/target")));
        assert!(!can_share(Path::new("/Users/x/.rustup/toolchains/stable-x")));
    }

    /// Nothing that can share means nothing to read off the device.
    #[test]
    fn a_scan_with_no_clone_root_needs_no_accounting() {
        let targets = vec![target("/Users/x/Develop/a/target", 10), target("/Users/x/b/target", 20)];
        assert!(Reclaim::measure(&targets).is_none());
    }

    /// The ordinary targets keep contributing their own figures, so switching
    /// the accounting on never makes a selection worth less than its parts that
    /// share nothing.
    #[test]
    fn targets_that_share_nothing_still_count_in_full() {
        let root = std::env::temp_dir().join(format!("wd40-reclaim-{}", std::process::id()));
        let shared = root.join(".cargo-target/proj");
        let _ = std::fs::create_dir_all(&shared);
        let _ = std::fs::write(shared.join("payload.bin"), vec![b'x'; 1 << 20]);

        let targets = vec![
            target(shared.to_str().unwrap(), 1 << 20),
            target("/Users/x/Develop/plain/target", 4096),
        ];
        let found = Reclaim::measure(&targets).expect("one root can share");

        let picked: HashSet<usize> = [1].into_iter().collect();
        let only_plain = found.bytes(&targets, &|index| picked.contains(&index));
        assert_eq!(only_plain, 4096, "a target outside the accounting counts in full");

        let none = found.bytes(&targets, &|_| false);
        assert_eq!(none, 0, "selecting nothing is worth nothing");
        let _ = std::fs::remove_dir_all(&root);
    }
}
