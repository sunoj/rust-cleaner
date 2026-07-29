// Config definitions for WD-40.
// Handles defaults and TOML parsing.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Directory names recognized as cleanable dev artifacts.
pub const ARTIFACT_DIRS: &[&str] = &["target", "node_modules", ".next", "dist", "build"];

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub scan_dirs: Vec<PathBuf>,
    pub max_age_days: u64,
    pub max_depth: usize,
    pub auto_clean_hours: u64,
    /// Which artifact dir names to scan. Defaults to all known types.
    pub artifact_types: Vec<String>,
}

/// Replace the known artifact types with `selected`, keeping any custom entry
/// the user added by hand. The window can only represent `ARTIFACT_DIRS`, so a
/// blind rewrite would silently delete anything outside that list.
pub fn merge_artifact_types(existing: &[String], selected: &[String]) -> Vec<String> {
    let mut merged: Vec<String> = selected.to_vec();
    merged.extend(
        existing
            .iter()
            .filter(|name| !ARTIFACT_DIRS.iter().any(|known| known == &name.as_str()))
            .cloned(),
    );
    merged
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan_dirs: default_scan_dirs(),
            max_age_days: 7,
            max_depth: 5,
            auto_clean_hours: 0,
            artifact_types: ARTIFACT_DIRS.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(parsed) = toml::from_str::<Config>(&contents) {
                    return parsed;
                } else {
                    eprintln!("wd-40: failed to parse {}", path.display());
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(toml_str) = toml::to_string_pretty(self) {
                let _ = fs::write(&path, toml_str);
            }
        }
    }

    fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".config/wd-40/config.toml"))
    }
}

fn default_scan_dirs() -> Vec<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join("Develop"))
        .map(|path| vec![path])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::merge_artifact_types;

    fn owned(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn merge_keeps_custom_entries_the_window_cannot_show() {
        let merged = merge_artifact_types(&owned(&["target", "vendor"]), &owned(&["target", "dist"]));
        assert_eq!(merged, owned(&["target", "dist", "vendor"]));
    }

    #[test]
    fn merge_drops_known_types_that_were_unchecked() {
        let merged = merge_artifact_types(&owned(&["target", "dist"]), &owned(&["target"]));
        assert_eq!(merged, owned(&["target"]));
    }

    #[test]
    fn merge_handles_an_empty_selection_without_losing_custom_entries() {
        let merged = merge_artifact_types(&owned(&["dist", "vendor"]), &[]);
        assert_eq!(merged, owned(&["vendor"]));
    }
}
