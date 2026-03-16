// Config definitions for WD-40.
// Handles defaults and TOML parsing.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
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
    pub fn is_first_run() -> bool {
        Self::config_path().map_or(true, |p| !p.exists())
    }

    pub fn broad_scan_dirs() -> Vec<PathBuf> {
        let Some(home) = dirs::home_dir() else {
            return vec![];
        };
        let candidates = [
            "Develop",
            "Developer",
            "Projects",
            "Documents",
            "repos",
            "code",
            "src",
            "workspace",
            "Code",
            "Work",
        ];
        candidates
            .iter()
            .map(|name| home.join(name))
            .filter(|p| p.is_dir())
            .collect()
    }

    pub fn discover_scan_dirs(targets: &[crate::scanner::TargetDir]) -> Vec<PathBuf> {
        let mut dirs = BTreeSet::new();
        for target in targets {
            if let Some(project_dir) = target.path.parent() {
                if let Some(dev_dir) = project_dir.parent() {
                    dirs.insert(dev_dir.to_path_buf());
                }
            }
        }
        dirs.into_iter().collect()
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(parsed) = toml::from_str::<Config>(&contents) {
                    return parsed;
                } else {
                    eprintln!("rust-cleaner: failed to parse {}", path.display());
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
