// Regression tests for unattended-clean policy, cancellation, and receipts.
// Exercises private policy helpers through `auto_clean`'s child test module.
// Deps: std, crate::{auto_clean, tasks}, wd40 config/scanner types.

use super::{
    can_start, candidate_indices, clean, clear_receipt, format_receipt, receipt_line,
    stop_for_popover, AutoCleanJob, Receipt, RECEIPT, RUNNING,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};
use wd40::config::Config;
use wd40::scanner::{ArtifactGroup, ArtifactKind, TargetDir};

fn target(now: SystemTime, age_days: u64, kind: ArtifactKind) -> TargetDir {
    TargetDir {
        path: PathBuf::from(format!("/tmp/auto-clean-{age_days}")),
        size_bytes: 1024,
        last_modified: now - Duration::from_secs(age_days * 86_400),
        kind,
    }
}

fn scratch(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("wd40-auto-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn manual_selection_does_not_affect_auto_clean_policy() {
    let now = SystemTime::now();
    let targets = vec![
        target(now, 8, ArtifactKind::RustTarget),
        target(now, 2, ArtifactKind::RustTarget),
    ];
    let manual_selection = HashSet::from([1]);
    let candidates = candidate_indices(&targets, &Config::default(), now);
    assert_eq!(candidates, [0]);
    assert!(!manual_selection.contains(&candidates[0]));
}

#[test]
fn target_one_day_under_keep_threshold_survives() {
    let now = SystemTime::now();
    let config = Config { max_age_days: 7, ..Config::default() };
    let targets = vec![target(now, 6, ArtifactKind::RustTarget)];
    assert!(candidate_indices(&targets, &config, now).is_empty());
}

#[test]
fn zero_keep_threshold_yields_no_auto_clean_candidates() {
    let now = SystemTime::now();
    let config = Config { max_age_days: 0, ..Config::default() };
    let targets = vec![target(now, 30, ArtifactKind::RustTarget)];
    assert!(candidate_indices(&targets, &config, now).is_empty());
}

#[test]
fn disabled_group_is_untouched() {
    let now = SystemTime::now();
    let mut config = Config::default();
    config.set_scans(ArtifactGroup::Caches, false);
    let targets = vec![target(now, 30, ArtifactKind::Cache)];
    assert!(candidate_indices(&targets, &config, now).is_empty());
}

#[test]
fn toolchains_are_never_auto_clean_candidates() {
    let now = SystemTime::now();
    let config = Config { max_age_days: 7, ..Config::default() };
    let targets = vec![target(now, 1000, ArtifactKind::Toolchain)];
    assert!(config.scans(ArtifactGroup::Toolchains));
    assert!(candidate_indices(&targets, &config, now).is_empty());
}

#[test]
fn start_guard_rejects_every_unsafe_ui_state() {
    assert!(can_start(false, false, false));
    assert!(!can_start(true, false, false));
    assert!(!can_start(false, true, false));
    assert!(!can_start(false, false, true));
}

#[test]
fn fresh_nested_content_is_rechecked_before_removal() {
    let root = scratch("fresh-recheck");
    std::fs::create_dir_all(root.join("stable")).expect("target");
    std::fs::write(root.join("stable/active"), b"building").expect("fresh content");
    let mut stale = target(SystemTime::now(), 30, ArtifactKind::RustTarget);
    stale.path = root.clone();
    let job = AutoCleanJob { items: vec![(stale, "active target".into())], reference: None };
    let config = Config { max_age_days: 7, ..Config::default() };
    let receipt = clean(job, &config, &AtomicBool::new(false));
    assert!(root.is_dir(), "fresh content must survive the final age check");
    assert!(receipt.groups.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn shared_stop_prevents_the_next_unattended_removal() {
    let root = scratch("shared-stop");
    std::fs::create_dir_all(&root).expect("target");
    let mut stale = target(SystemTime::now(), 30, ArtifactKind::RustTarget);
    stale.path = root.clone();
    let job = AutoCleanJob { items: vec![(stale, "stopped target".into())], reference: None };
    let receipt = clean(job, &Config::default(), &AtomicBool::new(true));
    assert!(root.is_dir());
    assert!(receipt.groups.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn opening_the_popover_requests_the_shared_stop() {
    crate::tasks::reset_clean_stop();
    RUNNING.store(true, Ordering::Relaxed);
    stop_for_popover();
    assert!(crate::tasks::stop_requested());
    RUNNING.store(false, Ordering::Relaxed);
    crate::tasks::reset_clean_stop();
}

#[test]
fn receipt_is_visible_then_cleared_by_its_lifecycle() {
    clear_receipt();
    assert!(format_receipt(None).is_none());
    *RECEIPT.lock().unwrap() = Some(Receipt {
        groups: vec![(ArtifactGroup::Rust, 2), (ArtifactGroup::Caches, 1)],
        names: vec!["alpha".into(), "beta".into(), "registry".into()],
        freed_bytes: 1536,
    });
    let line = receipt_line().expect("receipt line");
    assert!(line.contains("3 targets (alpha, beta, registry) \u{00b7} Rust/Caches"));
    assert!(line.ends_with("1.5K"));
    clear_receipt();
    assert!(receipt_line().is_none());
}
