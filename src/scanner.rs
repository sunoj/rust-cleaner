// Target directory scanner for WD-40.
// Finds Rust targets and reports size/mtime.
use crate::config::Config;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

pub struct TargetDir {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub last_modified: SystemTime,
}

pub fn scan(config: &Config) -> Vec<TargetDir> {
    let mut found = Vec::new();
    for dir in &config.scan_dirs {
        if !dir.exists() {
            continue;
        }
        let walker = WalkDir::new(dir).max_depth(config.max_depth);
        for entry in walker.into_iter().filter_map(Result::ok) {
            if entry.depth() == 0 || !entry.file_type().is_dir() {
                continue;
            }
            if entry.file_name() != "target" {
                continue;
            }
            if !is_rust_target_dir(entry.path()) {
                continue;
            }
            let metadata = entry.metadata().ok();
            let last_modified = metadata
                .as_ref()
                .and_then(|m| m.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let path = entry.into_path();
            let size_bytes = dir_size(&path);
            found.push(TargetDir {
                path,
                size_bytes,
                last_modified,
            });
        }
    }
    found.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    found
}

fn is_rust_target_dir(path: &Path) -> bool {
    path.join("debug").is_dir() || path.join("release").is_dir()
}

fn dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .map(|meta| meta.len())
        .sum()
}

pub fn human_size(bytes: u64) -> String {
    let mut value = bytes as f64;
    let mut unit = 0;
    let units = ["B", "K", "M", "G", "T"];
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{}{}", bytes, units[unit])
    } else {
        format!("{:.1}{}", value, units[unit])
    }
}
