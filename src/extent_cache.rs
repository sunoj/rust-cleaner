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
    /// The newest descendant timestamp measured this scan. It is evidence that
    /// the target was active, not proof that its extents stayed unchanged:
    /// filesystem timestamps can be coarse and a writer can restore them.
    content_modified: u64,
    /// The size measured this scan. A same-size deep write is why the content
    /// timestamp is checked alongside the aggregate size.
    bytes: u64,
    value: TargetExtents,
}

/// One target's extent map, if the directory has neither been touched nor
/// changed size or descendant timestamp since it was read.
///
/// No TTL here, and none is wanted. The extra timestamp is strictly stronger
/// evidence of activity than the root mtime plus size, but strictly weaker
/// than a guarantee that the physical extents are unchanged.
pub fn extents_of(path: &Path, bytes: u64, content_modified: SystemTime) -> Option<TargetExtents> {
    let modified = timestamp(std::fs::metadata(path).ok()?.modified().ok()?)?;
    let content_modified = timestamp(content_modified)?;
    let guard = KEPT.lock().ok()?;
    let kept = guard.as_ref()?.get(&key(path))?;
    (kept.modified == modified && kept.content_modified == content_modified && kept.bytes == bytes)
        .then(|| kept.value.clone())
}

pub fn put_extents(path: &Path, bytes: u64, content_modified: SystemTime, value: TargetExtents) {
    let Some(modified) = std::fs::metadata(path).ok().and_then(|m| m.modified().ok()).and_then(timestamp)
    else {
        return;
    };
    let Some(content_modified) = timestamp(content_modified) else { return };
    if let Ok(mut guard) = KEPT.lock() {
        guard
            .get_or_insert_with(HashMap::new)
            .insert(key(path), KeptExtents { modified, content_modified, bytes, value });
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

fn timestamp(at: SystemTime) -> Option<u64> {
    at.duration_since(UNIX_EPOCH).ok().map(|since| {
        since.as_nanos().min(u64::MAX as u128) as u64
    })
}

#[cfg(test)]
mod tests {
    use super::{extents_of, put_extents};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

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

        let content_modified = SystemTime::now();
        assert!(extents_of(&path, bytes, content_modified).is_none(), "nothing kept yet");
        put_extents(&path, bytes, content_modified, read_target(&path));
        assert!(extents_of(&path, bytes, content_modified).is_some(), "unchanged: answered");
        assert!(
            extents_of(&path, bytes, content_modified + Duration::from_nanos(1)).is_none(),
            "newer activity: read again"
        );

        // A different size is a different directory, whatever the clock says.
        assert!(extents_of(&path, bytes + 1, content_modified).is_none(), "resized: read again");

        std::thread::sleep(Duration::from_millis(1100));
        let _ = std::fs::write(path.join("another.bin"), b"x");
        assert!(extents_of(&path, bytes, content_modified).is_none(), "touched: read again");
        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn a_future_mtime_does_not_pin_later_content_activity() {
        let path = scratch("future-mtime");
        let file = path.join("payload.bin");
        let _ = std::fs::write(&file, b"old");
        let future = SystemTime::now() + Duration::from_secs(86_400);
        std::fs::File::open(&file).expect("payload").set_modified(future).expect("mtime");
        let before = crate::sizes::measure_dir_with_modified(&path).last_modified.expect("mtime");
        std::thread::sleep(Duration::from_millis(2));
        let _ = std::fs::write(&file, b"new");
        let after = crate::sizes::measure_dir_with_modified(&path).last_modified.expect("mtime");
        assert!(after > before);
        assert!(after <= SystemTime::now());
        let _ = std::fs::remove_dir_all(&path);
    }

}
