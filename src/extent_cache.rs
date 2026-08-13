// Extent maps kept for the life of the process. Level one only: four hundred
// thousand runs is ten megabytes of TOML and no launch is slow enough to be
// worth writing that — but within a run it is the difference between reading
// every target every ten minutes and reading the two a build touched.
// Exports: `extents_of`, `put_extents`, `forget`.
// Deps: std, crate::extents.

use crate::extents::TargetExtents;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Extent maps, level one only. Four hundred thousand runs is ten megabytes of
/// TOML and no launch is slow enough to be worth writing that; within a run,
/// though, it is the difference between re-reading every target every ten
/// minutes and re-reading the two that a build touched.
static KEPT: Mutex<Option<HashMap<String, KeptExtents>>> = Mutex::new(None);

struct KeptExtents {
    modified: u64,
    /// The size measured this scan. A directory's own mtime says nothing about
    /// a write deep inside it, and the size does — so both have to match.
    bytes: u64,
    value: TargetExtents,
}

/// One target's extent map, if the directory has neither been touched nor
/// changed size since it was read.
///
/// No TTL here, and none is wanted: an extent map is not an estimate that goes
/// stale with the clock, it is either still true of the directory on disk or it
/// is not, and mtime plus size is what answers that.
pub fn extents_of(path: &Path, bytes: u64) -> Option<TargetExtents> {
    let modified = seconds(std::fs::metadata(path).ok()?.modified().ok()?)?;
    let guard = KEPT.lock().ok()?;
    let kept = guard.as_ref()?.get(&key(path))?;
    (kept.modified == modified && kept.bytes == bytes).then(|| kept.value.clone())
}

pub fn put_extents(path: &Path, bytes: u64, value: TargetExtents) {
    let Some(modified) = std::fs::metadata(path).ok().and_then(|m| m.modified().ok()).and_then(seconds)
    else {
        return;
    };
    if let Ok(mut guard) = KEPT.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(key(path), KeptExtents { modified, bytes, value });
    }
}


/// Drop one path, so a target that comes back is read rather than answered
/// from before it went.
pub fn forget(path: &Path) {
    if let Ok(mut guard) = KEPT.lock() {
        if let Some(kept) = guard.as_mut() {
            kept.remove(&key(path));
        }
    }
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn seconds(at: SystemTime) -> Option<u64> {
    at.duration_since(UNIX_EPOCH).ok().map(|since| since.as_secs())
}

#[cfg(test)]
mod tests {
    use super::{extents_of, put_extents};
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("wd40-ext-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        let _ = std::fs::create_dir_all(&path);
        path
    }

    /// The claim the granularity change rests on: a target nothing touched is
    /// answered without reading it, and one that moved is not.
    #[test]
    fn an_extent_map_survives_until_the_directory_moves() {
        use crate::extents::read_target;
        let path = scratch("extents");
        let _ = std::fs::write(path.join("payload.bin"), vec![b'x'; 1 << 20]);
        let bytes = 1 << 20;

        assert!(extents_of(&path, bytes).is_none(), "nothing kept yet");
        put_extents(&path, bytes, read_target(&path));
        assert!(extents_of(&path, bytes).is_some(), "unchanged: answered");

        // A different size is a different directory, whatever the clock says.
        assert!(extents_of(&path, bytes + 1).is_none(), "resized: read again");

        std::thread::sleep(std::time::Duration::from_millis(1100));
        let _ = std::fs::write(path.join("another.bin"), b"x");
        assert!(extents_of(&path, bytes).is_none(), "touched: read again");
        let _ = std::fs::remove_dir_all(&path);
    }

}
