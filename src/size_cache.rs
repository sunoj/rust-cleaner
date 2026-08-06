// Remembered directory sizes, so a rescan does not re-walk a tree that cannot
// have changed. ~/.rustup/toolchains alone is 252,692 files and 3.6 s of walking
// on this Mac, and an installed toolchain is written exactly once — at install.
// Exports: `remembered`, `remember`, `forget`, `caches`.
// Deps: std, crate::scanner.

use crate::scanner::ArtifactKind;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Sizes measured earlier this run, keyed by the directory they belong to.
static REMEMBERED: Mutex<Option<HashMap<PathBuf, Entry>>> = Mutex::new(None);

struct Entry {
    modified: SystemTime,
    bytes: u64,
}

/// Whether a kind's size is worth remembering.
///
/// A Rust target is written by every build, so its figure has to be live or it
/// is a lie about the thing the app exists to watch. A toolchain is written
/// once by `rustup toolchain install`; a download cache only when something is
/// fetched. Those two are what the walk spends its time on and what never
/// changes between two scans a few minutes apart.
pub fn caches(kind: ArtifactKind) -> bool {
    matches!(kind, ArtifactKind::Toolchain | ArtifactKind::Cache)
}

/// The size measured for `path` last time, if the directory has not been
/// touched since. `None` means it has to be walked.
pub fn remembered(path: &Path, kind: ArtifactKind) -> Option<u64> {
    if !caches(kind) {
        return None;
    }
    let modified = modified_at(path)?;
    let guard = REMEMBERED.lock().ok()?;
    let entry = guard.as_ref()?.get(path)?;
    (entry.modified == modified).then_some(entry.bytes)
}

/// Keep `bytes` for `path` until its directory is touched again.
pub fn remember(path: &Path, kind: ArtifactKind, bytes: u64) {
    if !caches(kind) {
        return;
    }
    let Some(modified) = modified_at(path) else { return };
    let Ok(mut guard) = REMEMBERED.lock() else { return };
    guard
        .get_or_insert_with(HashMap::new)
        .insert(path.to_path_buf(), Entry { modified, bytes });
}

/// Drop what was remembered for `path`. Called when a target is removed, so a
/// path that comes back is measured rather than answered from before.
pub fn forget(path: &Path) {
    if let Ok(mut guard) = REMEMBERED.lock() {
        if let Some(entries) = guard.as_mut() {
            entries.remove(path);
        }
    }
}

fn modified_at(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::{caches, forget, remember, remembered};
    use crate::scanner::ArtifactKind;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("wd40-cache-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[test]
    fn a_toolchain_is_answered_from_memory_the_second_time() {
        let path = scratch("toolchain");
        assert_eq!(remembered(&path, ArtifactKind::Toolchain), None);
        remember(&path, ArtifactKind::Toolchain, 4096);
        assert_eq!(remembered(&path, ArtifactKind::Toolchain), Some(4096));
        let _ = std::fs::remove_dir_all(&path);
    }

    /// The red line: a build directory must never answer from memory, or the
    /// app reports a stale figure for the thing it exists to watch.
    #[test]
    fn a_rust_target_is_never_remembered() {
        let path = scratch("target");
        remember(&path, ArtifactKind::RustTarget, 4096);
        assert_eq!(remembered(&path, ArtifactKind::RustTarget), None);
        assert!(!caches(ArtifactKind::RustTarget));
        assert!(!caches(ArtifactKind::TmpTarget));
        assert!(!caches(ArtifactKind::NodeModules));
        assert!(!caches(ArtifactKind::BuildOutput));
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn touching_the_directory_invalidates_what_was_remembered() {
        let path = scratch("touched");
        remember(&path, ArtifactKind::Cache, 4096);
        assert_eq!(remembered(&path, ArtifactKind::Cache), Some(4096));
        // Adding an entry moves the directory's own mtime.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let _ = std::fs::write(path.join("new"), b"x");
        assert_eq!(remembered(&path, ArtifactKind::Cache), None);
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn forgetting_a_removed_path_makes_it_measurable_again() {
        let path = scratch("forgotten");
        remember(&path, ArtifactKind::Cache, 4096);
        forget(&path);
        assert_eq!(remembered(&path, ArtifactKind::Cache), None);
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn a_path_that_is_gone_is_never_answered_from_memory() {
        let path = scratch("vanished");
        remember(&path, ArtifactKind::Cache, 4096);
        let _ = std::fs::remove_dir_all(&path);
        assert_eq!(remembered(&path, ArtifactKind::Cache), None);
    }
}
