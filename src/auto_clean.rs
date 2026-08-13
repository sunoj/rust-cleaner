// Unattended clean policy and runner, independent of manual row selection.
// Exports: timer entry point, receipt text, pending-snapshot application.
// Deps: crate::{names, popover, state, tasks, tasks_clean}, wd40 scan/clean APIs.

use crate::names::display_names;
use crate::state::{with_state, with_state_ret, UiScreen};
use crate::tasks_clean::{self, CleanJob};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use wd40::config::Config;
use wd40::scanner::{human_size, ArtifactGroup, ArtifactKind, TargetDir};

const SECONDS_PER_DAY: u64 = 86_400;

static RECEIPT: Mutex<Option<Receipt>> = Mutex::new(None);
static SNAPSHOT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);

#[derive(Clone)]
struct Receipt {
    groups: Vec<(ArtifactGroup, usize)>,
    freed_bytes: u64,
}

pub fn start() {
    let cleaning_screen = with_state_ret(|state| state.screen == UiScreen::Cleaning)
        .unwrap_or(false);
    if !can_start(crate::tasks::is_busy(), cleaning_screen, crate::popover::is_open()) {
        return;
    }
    let Some(config) = with_state_ret(|state| state.config.clone()) else { return };
    if !crate::tasks::claim_cleaning() {
        return;
    }
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(|| run(config));
        if result.is_err() {
            eprintln!("wd-40: automatic clean stopped unexpectedly");
        }
        crate::tasks::finish_cleaning();
    });
}

pub fn apply_pending_snapshot() {
    let snapshot = SNAPSHOT.lock().unwrap_or_else(|error| error.into_inner()).take();
    let Some(targets) = snapshot else { return };
    with_state(|state| {
        state.targets = targets;
        state.measured = (0..state.targets.len()).collect();
        state.reclaim = None;
        state.reset_selection();
    });
}

pub fn discard_pending_snapshot() {
    SNAPSHOT.lock().unwrap_or_else(|error| error.into_inner()).take();
}

pub fn receipt_line() -> Option<String> {
    let receipt = RECEIPT.lock().unwrap_or_else(|error| error.into_inner());
    format_receipt(receipt.as_ref())
}

fn run(config: Config) {
    let mut targets = wd40::discover::scan_discover(&config);
    wd40::sizes::scan_sizes(&mut targets);
    let job = gather_job(&targets, &config, SystemTime::now());
    if let Some(job) = job {
        let receipt = clean(job);
        if !receipt.groups.is_empty() {
            *RECEIPT.lock().unwrap_or_else(|error| error.into_inner()) = Some(receipt);
        }
        targets.retain(|target| target.path.is_dir());
        wd40::cache::flush();
    }
    *SNAPSHOT.lock().unwrap_or_else(|error| error.into_inner()) = Some(targets);
}

fn gather_job(targets: &[TargetDir], config: &Config, now: SystemTime) -> Option<CleanJob> {
    let indices = candidate_indices(targets, config, now);
    if indices.is_empty() {
        return None;
    }
    let names = display_names(targets);
    let items = indices
        .into_iter()
        .map(|index| (index, targets[index].clone(), names[index].clone()))
        .collect();
    let reference = config
        .scan_dirs
        .iter()
        .find(|path| path.exists())
        .or_else(|| targets.first().map(|target| &target.path))
        .cloned();
    Some(CleanJob { items, skipped_count: 0, skipped_bytes: 0, reference })
}

fn clean(job: CleanJob) -> Receipt {
    let CleanJob { items, skipped_count, skipped_bytes, reference } = job;
    let before = reference.as_deref().and_then(wd40::disk::disk_space);
    let progress = tasks_clean::initial_progress(&items);
    let stop = std::sync::atomic::AtomicBool::new(false);
    let summary = tasks_clean::run_clean(
        items, skipped_count, skipped_bytes, reference, before, progress, |_| {}, &stop,
    );
    Receipt {
        groups: summary.removed.iter().map(|row| (row.group, row.count)).collect(),
        freed_bytes: summary.freed_bytes,
    }
}

fn can_start(busy: bool, cleaning_screen: bool, popover_open: bool) -> bool {
    !busy && !cleaning_screen && !popover_open
}

fn candidate_indices(targets: &[TargetDir], config: &Config, now: SystemTime) -> Vec<usize> {
    let threshold = Duration::from_secs(config.max_age_days.saturating_mul(SECONDS_PER_DAY));
    targets
        .iter()
        .enumerate()
        .filter(|(_, target)| !matches!(target.kind, ArtifactKind::Toolchain))
        .filter(|(_, target)| config.scans(target.kind.group()))
        .filter(|(_, target)| {
            now.duration_since(target.last_modified)
                .is_ok_and(|age| age >= threshold)
        })
        .map(|(index, _)| index)
        .collect()
}

fn format_receipt(receipt: Option<&Receipt>) -> Option<String> {
    let receipt = receipt?;
    let count: usize = receipt.groups.iter().map(|(_, count)| count).sum();
    let kinds = receipt
        .groups
        .iter()
        .map(|(group, _)| receipt_group_label(*group))
        .collect::<Vec<_>>()
        .join("/");
    Some(format!(
        "Auto-cleaned {count} targets \u{00b7} {kinds} \u{00b7} {}",
        human_size(receipt.freed_bytes)
    ))
}

fn receipt_group_label(group: ArtifactGroup) -> &'static str {
    match group {
        ArtifactGroup::Rust => "Rust",
        ArtifactGroup::NodeModules => "Node",
        ArtifactGroup::BuildOutput => "Builds",
        ArtifactGroup::Caches => "Caches",
        ArtifactGroup::Toolchains => "Toolchains",
    }
}

#[cfg(test)]
mod tests {
    use super::{can_start, candidate_indices, format_receipt, Receipt};
    use std::collections::HashSet;
    use std::path::PathBuf;
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
        let config = Config { max_age_days: 0, ..Config::default() };
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
    fn receipt_line_exists_only_after_a_removal() {
        assert!(format_receipt(None).is_none());
        let receipt = Receipt {
            groups: vec![(ArtifactGroup::Rust, 2), (ArtifactGroup::Caches, 1)],
            freed_bytes: 1536,
        };
        let line = format_receipt(Some(&receipt)).expect("receipt line");
        assert!(line.contains("3 targets \u{00b7} Rust/Caches"));
        assert!(line.ends_with("1.5K"));
    }
}
