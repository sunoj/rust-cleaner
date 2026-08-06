// Artifact roots that live where the ordinary walk will not go: /tmp, and the
// dot-directories under $HOME that `should_skip` deliberately refuses to enter.
// Exports: the `collect_*` gatherers and `is_cargo_target`.
// Deps: std, dirs, crate::scanner.

use crate::scanner::{ArtifactKind, TargetDir};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A cargo target dir always has something built in it.
pub(crate) fn is_cargo_target(path: &Path) -> bool {
    path.join("debug").is_dir() || path.join("release").is_dir()
}

/// Collect /tmp Cargo target dirs.
///
/// Recognized name patterns (all confirmed by a `debug/` or `release/` subdir):
///   * `cc-target-*`               — Claude Code per-session targets
///   * `*-target`                  — ad-hoc CARGO_TARGET_DIR (e.g. `smart-router-target`)
///   * `*-target-*`                — ad-hoc CARGO_TARGET_DIR with a suffix
pub(crate) fn collect_tmp_targets(found: &mut Vec<TargetDir>) {
    // macOS: /tmp is a symlink to /private/tmp. read_dir on either works.
    let Ok(entries) = std::fs::read_dir(Path::new("/tmp")) else { return };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        if !is_tmp_target_name(&name.to_string_lossy()) {
            continue;
        }
        // Validate it's actually a cargo target dir, not an unrelated dir that
        // happens to carry "-target" in its name.
        let path = entry.path();
        if path.is_symlink() || !path.is_dir() || !is_cargo_target(&path) {
            continue;
        }
        found.push(TargetDir {
            last_modified: modified(&path),
            path,
            size_bytes: 0,
            kind: ArtifactKind::TmpTarget,
        });
    }
}

fn is_tmp_target_name(name: &str) -> bool {
    name.starts_with("cc-target-") || name.ends_with("-target") || name.contains("-target-")
}

/// Collect ~/.aid/worktrees/<repo>/<branch>/target directories.
/// These live under a dot-prefixed dir so the main WalkDir skips them.
pub(crate) fn collect_aid_worktrees(found: &mut Vec<TargetDir>) {
    let Some(home) = dirs::home_dir() else { return };
    for repo in subdirs(&home.join(".aid").join("worktrees")) {
        for branch in subdirs(&repo) {
            let target = branch.join("target");
            if target.is_dir() && is_cargo_target(&target) {
                push_dir(found, target, ArtifactKind::RustTarget);
            }
        }
    }
}

/// Collect per-project subdirs under ~/.cargo-target/ (shared CARGO_TARGET_DIR
/// root). Supports both <project>/debug and <project>/<session>/debug layouts.
pub(crate) fn collect_shared_cargo_target(found: &mut Vec<TargetDir>) {
    let Some(home) = dirs::home_dir() else { return };
    for project in subdirs(&home.join(".cargo-target")) {
        if is_cargo_target(&project) {
            push_dir(found, project.clone(), ArtifactKind::RustTarget);
        }
        for session in subdirs(&project) {
            if is_cargo_target(&session) {
                push_dir(found, session, ArtifactKind::RustTarget);
            }
        }
    }
}

pub(crate) fn collect_dev_caches(found: &mut Vec<TargetDir>) {
    let Some(home) = dirs::home_dir() else { return };
    for path in [
        home.join("Library/Developer/Xcode/DerivedData"),
        home.join("Library/Developer/Xcode/ModuleCache.noindex"),
        home.join("Library/Caches/Homebrew"),
        // ~/.npm is npm's cache *root*, but it also holds _logs and npx state.
        // Only the content-addressable store is safe to drop wholesale.
        home.join(".npm/_cacache"),
        home.join("Library/pnpm/store"),
        home.join(".cache/pnpm"),
        home.join(".local/share/pnpm/store"),
        home.join(".pnpm-store"),
        home.join("Library/Caches/Yarn"),
        home.join(".cache/yarn"),
        PathBuf::from("/opt/homebrew/var/homebrew/cache"),
        PathBuf::from("/usr/local/var/homebrew/cache"),
    ] {
        if path.is_dir() && !path.is_symlink() {
            push_dir(found, path, ArtifactKind::Cache);
        }
    }

    for device in subdirs(&home.join("Library/Developer/CoreSimulator/Devices")) {
        let caches = device.join("data/Library/Caches");
        if caches.is_dir() && !caches.is_symlink() {
            push_dir(found, caches, ArtifactKind::Cache);
        }
    }
}

/// Real subdirectories of `root`, symlinks excluded.
fn subdirs(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| !path.is_symlink() && path.is_dir())
        .collect()
}

fn push_dir(found: &mut Vec<TargetDir>, path: PathBuf, kind: ArtifactKind) {
    found.push(TargetDir { last_modified: modified(&path), path, size_bytes: 0, kind });
}

fn modified(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use super::is_tmp_target_name;

    #[test]
    fn tmp_target_names_cover_the_three_layouts() {
        assert!(is_tmp_target_name("cc-target-wd40"));
        assert!(is_tmp_target_name("smart-router-target"));
        assert!(is_tmp_target_name("filler-target-issue144"));
        assert!(!is_tmp_target_name("com.apple.launchd.abc"));
    }
}
