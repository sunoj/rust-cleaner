// Human-readable labels and paths for scanned artifact directories.
// Exports: `display_names`, `display_path`, `project_name`, `age`.
// Deps: dirs, wd40::scanner.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use wd40::scanner::{ArtifactKind, TargetDir};

const SECONDS_PER_DAY: u64 = 86_400;

/// One label per target, disambiguated so no two rows read the same.
pub fn display_names(targets: &[TargetDir]) -> Vec<String> {
    let mut names: Vec<String> = targets.iter().map(project_name).collect();
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for name in &names {
        *seen.entry(name.as_str()).or_insert(0) += 1;
    }
    let repeated: Vec<String> = seen
        .iter()
        .filter(|(_, &count)| count > 1)
        .map(|(name, _)| (*name).to_string())
        .collect();

    for (index, name) in names.iter_mut().enumerate() {
        if !repeated.iter().any(|dup| dup == name) {
            continue;
        }
        // Two projects share a directory name — say which tree each lives in.
        if let Some(context) = context_dir(&targets[index].path, name) {
            *name = format!("{context}/{name}");
        }
    }
    names
}

/// The directory one level above the project, when the project dir is what the
/// name came from. Anything else (tmp targets, shared cargo roots) is already
/// unique by construction.
fn context_dir(path: &Path, name: &str) -> Option<String> {
    let project = path.parent()?;
    if project.file_name()?.to_str()? != name {
        return None;
    }
    Some(project.parent()?.file_name()?.to_str()?.to_string())
}

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy().into_owned();
    let Some(home) = dirs::home_dir() else { return text };
    match path.strip_prefix(&home) {
        Ok(rel) => format!("~/{}", rel.display()),
        Err(_) => text,
    }
}

pub fn age_short(modified: SystemTime) -> String {
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return "now".to_string();
    };
    match elapsed.as_secs() / SECONDS_PER_DAY {
        0 => "today".to_string(),
        1 => "1d ago".to_string(),
        days => format!("{days}d ago"),
    }
}

/// Longer age phrase kept for the CLI / future menus.
#[allow(dead_code)]
pub fn age(modified: SystemTime) -> String {
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return "modified just now".to_string();
    };
    match elapsed.as_secs() / SECONDS_PER_DAY {
        0 => "modified today".to_string(),
        1 => "modified yesterday".to_string(),
        days => format!("modified {days} days ago"),
    }
}

pub fn project_name(td: &TargetDir) -> String {
    match td.kind {
        ArtifactKind::TmpTarget => {
            let dir_name = td.path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            // Strip the cc-target- prefix when present; otherwise show as-is so
            // names like "smart-router-target" stay recognizable.
            dir_name.strip_prefix("cc-target-").unwrap_or(dir_name).to_string()
        }
        ArtifactKind::Cache => td
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("cache")
            .to_string(),
        _ => {
            if let Some(home) = dirs::home_dir() {
                // Under ~/.cargo-target/<project>[/<session>]/, show path relative to the root.
                let shared_root = home.join(".cargo-target");
                if let Ok(rel) = td.path.strip_prefix(&shared_root) {
                    return rel.to_string_lossy().into_owned();
                }
                // Under ~/.aid/worktrees/<repo>/<branch>/target, show "<repo>/<branch>".
                let aid_root = home.join(".aid").join("worktrees");
                if let Ok(rel) = td.path.strip_prefix(&aid_root) {
                    if let Some(parent) = rel.parent() {
                        let name = parent.to_string_lossy();
                        if !name.is_empty() {
                            return name.into_owned();
                        }
                    }
                }
            }
            // Standard target/node_modules/.next — show the containing project dir name.
            td.path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::display_names;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use wd40::scanner::{ArtifactKind, TargetDir};

    fn target(path: &str, kind: ArtifactKind) -> TargetDir {
        TargetDir {
            path: PathBuf::from(path),
            size_bytes: 0,
            last_modified: SystemTime::UNIX_EPOCH,
            kind,
        }
    }

    #[test]
    fn repeated_project_names_gain_their_parent_directory() {
        let targets = vec![
            target("/w/alpha/web/node_modules", ArtifactKind::NodeModules),
            target("/w/beta/web/node_modules", ArtifactKind::NodeModules),
            target("/w/gamma/api/node_modules", ArtifactKind::NodeModules),
        ];
        assert_eq!(display_names(&targets), ["alpha/web", "beta/web", "api"]);
    }

    #[test]
    fn a_name_outside_the_home_directory_survives_display() {
        let targets = vec![target("/tmp/cc-target-solo", ArtifactKind::TmpTarget)];
        assert_eq!(display_names(&targets), ["solo"]);
    }
}
