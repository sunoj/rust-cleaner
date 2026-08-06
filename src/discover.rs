// Phase 1 of a scan: find artifact directories without measuring them.
// Fast (<1s) because it never descends into an artifact, and because the roots
// that live under dot-directories are collected by name instead of walked.
// Exports: `scan_discover`. Deps: walkdir, crate::{config, roots, scanner}.

use crate::config::Config;
use crate::roots;
use crate::scanner::{ArtifactGroup, ArtifactKind, TargetDir};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};

/// Directories that never contain dev artifacts — skip to avoid slow traversal.
const SKIP_DIRS: &[&str] = &[
    // Media & personal
    "Music", "Movies", "Photos", "Pictures",
    // macOS system
    "Library", "Applications", "System",
    // iCloud & cloud storage
    "Mobile Documents", "iCloud Drive", "Google Drive", "OneDrive", "Dropbox",
    // Network & external volumes
    "Volumes",
];

/// Directory names to match, once the groups the user switched off are taken
/// out. A name the app does not know counts as build output, which is the same
/// kind `walk` will give it if it is found.
pub fn walk_types(config: &Config) -> Vec<&str> {
    config
        .artifact_types
        .iter()
        .map(String::as_str)
        .filter(|name| config.scans(ArtifactKind::for_dir_name(name).group()))
        .collect()
}

/// Discover artifact directories. Sizes are all zero on return.
pub fn scan_discover(config: &Config) -> Vec<TargetDir> {
    let types = walk_types(config);
    let dirs: Vec<&PathBuf> = match types.is_empty() {
        // Nothing the walk could match, so there is nothing to walk for.
        true => Vec::new(),
        false => config.scan_dirs.iter().filter(|dir| dir.exists()).collect(),
    };
    let mut found: Vec<TargetDir> = std::thread::scope(|scope| {
        let handles: Vec<_> = dirs
            .iter()
            .map(|dir| {
                let types = &types;
                let artifact_types = &config.artifact_types;
                let max_depth = config.max_depth;
                scope.spawn(move || walk(dir, types, artifact_types, max_depth))
            })
            .collect();
        handles.into_iter().flat_map(|handle| handle.join().unwrap_or_default()).collect()
    });
    // The roots below are collected by name rather than walked for, so each one
    // has to be gated on its own group here.
    if config.scans(ArtifactGroup::Rust) {
        roots::collect_tmp_targets(&mut found);
        roots::collect_shared_cargo_target(&mut found);
        roots::collect_aid_worktrees(&mut found);
    }
    if config.scans(ArtifactGroup::Caches) {
        roots::collect_dev_caches(&mut found);
    }
    if config.scans(ArtifactGroup::Toolchains) {
        roots::collect_toolchains(&mut found, &config.scan_dirs, config.max_depth);
    }
    found
}

fn walk(dir: &Path, types: &[&str], artifact_types: &[String], max_depth: usize) -> Vec<TargetDir> {
    let mut local = Vec::new();
    let walker = WalkDir::new(dir).max_depth(max_depth);
    for entry in walker
        .into_iter()
        .filter_entry(|entry| !should_skip(entry, artifact_types))
        .filter_map(Result::ok)
    {
        if entry.depth() == 0 || !entry.file_type().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if !types.contains(&name.as_ref()) || !is_dev_artifact(entry.path(), &name) {
            continue;
        }
        let kind = ArtifactKind::for_dir_name(name.as_ref());
        let last_modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        local.push(TargetDir { path: entry.into_path(), size_bytes: 0, last_modified, kind });
    }
    local
}

fn should_skip(entry: &DirEntry, artifact_types: &[String]) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return false;
    }
    // Skip symlinks — don't follow into network mounts or iCloud aliases
    if entry.path_is_symlink() {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    // Skip hidden dirs (except known artifact types like .next)
    if name.starts_with('.') && !artifact_types.iter().any(|t| t == name.as_ref()) {
        return true;
    }
    // Skip non-dev directories (media, cloud, system)
    if SKIP_DIRS.contains(&name.as_ref()) {
        return true;
    }
    // Don't descend INTO artifact directories — we only need the dir itself.
    entry
        .path()
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|parent| artifact_types.iter().any(|t| t.as_str() == parent))
}

/// Validate that a directory is actually a dev artifact, not a false positive.
pub(crate) fn is_dev_artifact(path: &Path, name: &str) -> bool {
    match name {
        "target" => roots::is_cargo_target(path),
        "node_modules" => {
            path.parent().is_some_and(|parent| parent.join("package.json").is_file())
                || path.join(".package-lock.json").is_file()
                || path.join(".yarn-integrity").is_file()
                || path.join(".modules.yaml").is_file()
                || path.join(".pnpm").is_dir()
                || path.join(".bin").is_dir()
        }
        ".next" => path.join("cache").is_dir() || path.join("static").is_dir(),
        "dist" | "build" => path.parent().is_some_and(|parent| {
            parent.join("package.json").is_file()
                || parent.join("Cargo.toml").is_file()
                || parent.join("build.gradle").is_file()
                || parent.join("platformio.ini").is_file()
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_dev_artifact, walk_types};
    use crate::config::Config;
    use crate::scanner::ArtifactGroup;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        path.push(format!("wd40-{name}-{}-{stamp}", std::process::id()));
        path
    }

    fn cleanup(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn pnpm_markers_allow_node_modules_detection() {
        let root = temp_dir("pnpm-markers");
        let node_modules = root.join("node_modules");
        let _ = fs::create_dir_all(node_modules.join(".pnpm"));
        let _ = fs::create_dir_all(node_modules.join(".bin"));
        let _ = fs::write(node_modules.join(".modules.yaml"), "hoistPattern: []");

        assert!(is_dev_artifact(&node_modules, "node_modules"));
        cleanup(&root);
    }

    #[test]
    fn a_group_switched_off_takes_its_dir_names_out_of_the_walk() {
        let mut config = Config::default();
        config.set_scans(ArtifactGroup::BuildOutput, false);
        let types = walk_types(&config);
        assert!(types.contains(&"target"));
        assert!(types.contains(&"node_modules"));
        assert!(!types.contains(&"dist"));
        assert!(!types.contains(&".next"));
    }

    /// A name the app does not know is build output as far as the walk is
    /// concerned, so the build output switch has to govern it too.
    #[test]
    fn a_custom_dir_name_follows_the_build_output_group() {
        let mut config = Config::default();
        config.artifact_types.push("vendor".to_string());
        assert!(walk_types(&config).contains(&"vendor"));
        config.set_scans(ArtifactGroup::BuildOutput, false);
        assert!(!walk_types(&config).contains(&"vendor"));
    }

    #[test]
    fn node_modules_without_markers_are_ignored() {
        let root = temp_dir("node-modules-empty");
        let node_modules = root.join("node_modules");
        let _ = fs::create_dir_all(&node_modules);

        assert!(!is_dev_artifact(&node_modules, "node_modules"));
        cleanup(&root);
    }
}
