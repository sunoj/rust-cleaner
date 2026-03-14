// Cleaning utilities for WD-40.
// Removes stale target directories and reports stats.
use crate::scanner::TargetDir;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub struct CleanResult {
    pub freed_bytes: u64,
    pub removed_count: usize,
    pub errors: Vec<(PathBuf, String)>,
}

impl Default for CleanResult {
    fn default() -> Self {
        Self {
            freed_bytes: 0,
            removed_count: 0,
            errors: Vec::new(),
        }
    }
}

pub fn clean_all(targets: &[TargetDir]) -> CleanResult {
    let mut result = CleanResult::default();
    for target in targets {
        match fs::remove_dir_all(&target.path) {
            Ok(()) => {
                result.removed_count += 1;
                result.freed_bytes += target.size_bytes;
            }
            Err(err) => result
                .errors
                .push((target.path.clone(), err.to_string())),
        }
    }
    result
}

pub fn clean_old(targets: &[TargetDir], max_age: Duration) -> CleanResult {
    let mut result = CleanResult::default();
    let now = SystemTime::now();
    for target in targets {
        let age = now
            .duration_since(target.last_modified)
            .unwrap_or(Duration::ZERO);
        if age >= max_age {
            match fs::remove_dir_all(&target.path) {
                Ok(()) => {
                    result.removed_count += 1;
                    result.freed_bytes += target.size_bytes;
                }
                Err(err) => result
                    .errors
                    .push((target.path.clone(), err.to_string())),
            }
        }
    }
    result
}
