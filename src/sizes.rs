// Directory sizing for WD-40. A bounded pool (see `walk`) draws directories
// from every target at once, so the largest target cannot serialise the scan,
// and each size is handed back the moment it is final.
// Exports: `SizedTarget`, `size_targets`, `scan_sizes`, directory measurement.
// Deps: getattrlistbulk, walkdir, crate::{disk, nesting, scanner, walk}.

use crate::disk::disk_space;
use crate::nesting::Publisher;
use crate::scanner::TargetDir;
use crate::cache;
use crate::walk::Walk;
use getattrlistbulk::{DirReader, ObjectType, RequestedAttributes};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use walkdir::WalkDir;

const BULK_ATTRS: RequestedAttributes = RequestedAttributes {
    name: true,
    object_type: true,
    size: true,
    alloc_size: true,
    modified_time: true,
    permissions: false,
    inode: false,
    entry_count: false,
};
const BULK_BUF: usize = 256 * 1024;

/// A target whose size is settled: anything nested inside it has already been
/// subtracted, so this figure is never revised.
pub struct SizedTarget {
    pub index: usize,
    pub bytes: u64,
    pub last_modified: SystemTime,
}

/// Allocated bytes and the newest content timestamp found below one target.
#[derive(Clone, Copy)]
pub struct Measurement {
    pub bytes: u64,
    pub last_modified: SystemTime,
}

/// Measure every target, calling `on_size` once per target as its size settles.
/// Indices are positions in `targets`. The callback runs on worker threads.
pub fn size_targets(targets: &[TargetDir], on_size: impl Fn(SizedTarget) + Sync) {
    if targets.is_empty() {
        return;
    }
    let paths: Vec<PathBuf> = targets.iter().map(|target| target.path.clone()).collect();
    let publisher = Mutex::new(Publisher::new(&paths));

    // Anything already known goes to the publisher before the walk starts, so
    // a target enclosing one of them still settles correctly.
    let cached: Vec<Option<(u64, SystemTime)>> = targets
        .iter()
        .map(|target| cache::measurement_of(&target.path, target.kind))
        .collect();
    let modified = Mutex::new(targets.iter().map(|target| target.last_modified).collect::<Vec<_>>());
    for (index, measurement) in cached.iter().copied().enumerate() {
        let Some((bytes, last_modified)) = measurement else { continue };
        modified.lock().unwrap_or_else(|error| error.into_inner())[index] = last_modified;
        let settled = publisher
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .record(index, bytes);
        for (index, bytes) in settled {
            let last_modified = modified
                .lock()
                .unwrap_or_else(|error| error.into_inner())[index];
            on_size(SizedTarget { index, bytes, last_modified });
        }
    }

    let known: Vec<bool> = cached.iter().map(Option::is_some).collect();
    let walk = Walk::new(&paths, &known);
    let remember = |sized: SizedTarget| {
        if let Some(target) = targets.get(sized.index) {
            cache::put_measurement(&target.path, target.kind, sized.bytes, sized.last_modified);
        }
        on_size(sized);
    };
    std::thread::scope(|scope| {
        for _ in 0..crate::qos::workers() {
            scope.spawn(|| {
                // Housekeeping: never take the disk away from a build.
                crate::qos::background();
                walk.run(&paths, &publisher, &modified, &remember);
            });
        }
    });
}

/// Blocking sizing for callers with nothing to show in the meantime (the CLI).
pub fn scan_sizes(targets: &mut [TargetDir]) {
    let initial: Vec<_> = targets.iter().map(|target| (0_u64, target.last_modified)).collect();
    let settled = Mutex::new(initial);
    size_targets(targets, |sized| {
        settled.lock().unwrap_or_else(|error| error.into_inner())[sized.index] =
            (sized.bytes, sized.last_modified);
    });
    let settled = settled.into_inner().unwrap_or_default();
    for (target, (bytes, last_modified)) in targets.iter_mut().zip(settled) {
        target.size_bytes = bytes;
        target.last_modified = last_modified;
    }
    targets.sort_by_key(|target| std::cmp::Reverse(target.size_bytes));
}

/// What one directory contributed.
pub(crate) struct Found {
    pub bytes: u64,
    pub last_modified: SystemTime,
    pub subdirs: Vec<PathBuf>,
    pub broken: bool,
}

/// One directory measured on the calling thread. Used to find out how much of
/// a target a failed removal actually took off, where spinning up the pool for
/// a single path would cost more than the walk.
pub fn measure_dir(path: &Path) -> u64 {
    measure_dir_with_modified(path).bytes
}

/// Measure a target and retain the newest timestamp anywhere in its contents.
pub fn measure_dir_with_modified(path: &Path) -> Measurement {
    let limit = disk_space(path).map(|stats| stats.total_bytes);
    let mut total: u64 = 0;
    let mut last_modified = SystemTime::UNIX_EPOCH;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let found = read_dir(&dir);
        if found.broken {
            let mut fallback = dir_measurement_fallback(path);
            fallback.bytes = limit.map_or(fallback.bytes, |max| fallback.bytes.min(max));
            return fallback;
        }
        total = total.saturating_add(found.bytes);
        last_modified = last_modified.max(found.last_modified);
        stack.extend(found.subdirs);
    }
    Measurement {
        bytes: limit.map_or(total, |max| total.min(max)),
        last_modified,
    }
}

pub(crate) fn read_dir(dir: &Path) -> Found {
    let entries = DirReader::new(dir)
        .attributes(BULK_ATTRS)
        .buffer_size(BULK_BUF)
        .follow_symlinks(false)
        .read();
    let Ok(entries) = entries else {
        return Found {
            bytes: 0,
            last_modified: SystemTime::UNIX_EPOCH,
            subdirs: Vec::new(),
            broken: true,
        };
    };
    let last_modified = std::fs::metadata(dir)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut found = Found { bytes: 0, last_modified, subdirs: Vec::new(), broken: false };
    for entry in entries {
        let Ok(entry) = entry else {
            // Parse error — the data may be corrupt, so the whole target is
            // re-measured the slow way rather than half-counted.
            return Found {
                bytes: 0,
                last_modified: SystemTime::UNIX_EPOCH,
                subdirs: Vec::new(),
                broken: true,
            };
        };
        found.last_modified = found
            .last_modified
            .max(entry.modified_time.unwrap_or(SystemTime::UNIX_EPOCH));
        match entry.object_type {
            Some(ObjectType::Directory) => found.subdirs.push(dir.join(&entry.name)),
            Some(ObjectType::Symlink) => {}
            _ => {
                let bytes = entry.alloc_size.or(entry.size).unwrap_or(0);
                found.bytes = found.bytes.saturating_add(bytes);
            }
        }
    }
    found
}

pub(crate) fn dir_measurement_fallback(path: &Path) -> Measurement {
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .fold(
            Measurement { bytes: 0, last_modified: SystemTime::UNIX_EPOCH },
            |measurement, metadata| Measurement {
                bytes: measurement.bytes.saturating_add(metadata.blocks().saturating_mul(512)),
                last_modified: measurement.last_modified.max(
                    metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                ),
            },
        )
}

#[cfg(test)]
mod tests {
    use super::{scan_sizes, size_targets};
    use crate::scanner::{ArtifactKind, TargetDir};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::time::SystemTime;

    fn tree(root: &Path, files: &[(&str, usize)]) {
        for (relative, bytes) in files {
            let path = root.join(relative);
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(root));
            let _ = std::fs::write(path, vec![b'x'; *bytes]);
        }
    }

    fn target(path: PathBuf) -> TargetDir {
        TargetDir {
            path,
            size_bytes: 0,
            last_modified: SystemTime::UNIX_EPOCH,
            kind: ArtifactKind::RustTarget,
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("wd40-sizes-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn every_target_is_reported_exactly_once() {
        let root = scratch("once");
        tree(&root, &[("a/one.bin", 4096), ("b/two.bin", 4096), ("b/deep/three.bin", 4096)]);
        let targets = vec![target(root.join("a")), target(root.join("b"))];
        let seen = Mutex::new(Vec::new());
        size_targets(&targets, |sized| seen.lock().unwrap().push(sized.index));
        let mut seen = seen.into_inner().unwrap();
        seen.sort_unstable();
        assert_eq!(seen, [0, 1]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_nested_target_is_not_counted_in_the_one_around_it() {
        let root = scratch("nested");
        tree(&root, &[("outer/own.bin", 4096), ("outer/inner/kept.bin", 4096)]);
        let mut targets = vec![target(root.join("outer")), target(root.join("outer/inner"))];
        scan_sizes(&mut targets);
        // Sorted largest first; both hold one 4K file once the overlap is out.
        assert_eq!(targets.len(), 2);
        let total: u64 = targets.iter().map(|t| t.size_bytes).sum();
        let outer = targets.iter().find(|t| t.path.ends_with("outer")).unwrap();
        let inner = targets.iter().find(|t| t.path.ends_with("inner")).unwrap();
        assert!(inner.size_bytes > 0, "inner measured");
        assert_eq!(outer.size_bytes, total - inner.size_bytes);
        assert!(outer.size_bytes < total, "outer must not re-count the inner target");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_target_list_does_no_work() {
        let mut targets: Vec<TargetDir> = Vec::new();
        scan_sizes(&mut targets);
        assert!(targets.is_empty());
    }

    #[test]
    fn a_missing_directory_sizes_to_nothing_rather_than_hanging() {
        let mut targets = vec![target(scratch("absent"))];
        scan_sizes(&mut targets);
        assert_eq!(targets[0].size_bytes, 0);
    }

    #[test]
    fn newest_nested_content_replaces_the_target_directory_mtime() {
        let root = scratch("content-age");
        let nested = root.join("stable");
        std::fs::create_dir_all(&nested).expect("nested dir");
        let directory_mtime = std::fs::metadata(&root).and_then(|m| m.modified()).expect("mtime");
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(nested.join("fresh"), b"new").expect("nested content");
        let mut targets = vec![target(root.clone())];
        scan_sizes(&mut targets);
        assert!(targets[0].last_modified > directory_mtime);
        let _ = std::fs::remove_dir_all(&root);
    }
}
