// Selected-item clean job: bounded parallel removal, progress updates and the
// done summary (background thread). Exports: `run_clean`. Called only from `tasks`.
// Deps: std, wd40::{cleaner, disk, scanner}, crate::{names, state}.

use crate::names::display_names;
use crate::state::{
    with_state_ret, CleanItem, CleanItemStatus, CleanProgress, DoneSummary, GroupSummary,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use wd40::cleaner::{remove_targets, Removal};
use wd40::disk::{disk_space, DiskSpace};
use wd40::scanner::{human_size, ArtifactGroup, TargetDir};

/// Exactly what the user ticked, resolved to targets before anything starts.
pub struct CleanJob {
    pub items: Vec<(usize, TargetDir, String)>,
    pub skipped_count: usize,
    pub skipped_bytes: u64,
    pub reference: Option<PathBuf>,
}

/// Read the ticked rows out of the state. Nothing outside `selected` is ever
/// put in the job, and the job is fixed from here on.
pub fn gather_job() -> Option<CleanJob> {
    with_state_ret(|state| {
        let names = display_names(&state.targets);
        let items: Vec<(usize, TargetDir, String)> = state
            .selected
            .iter()
            .copied()
            .filter_map(|i| state.targets.get(i).map(|td| (i, td.clone(), names[i].clone())))
            .collect();
        CleanJob {
            skipped_count: state.targets.len().saturating_sub(items.len()),
            skipped_bytes: state
                .targets
                .iter()
                .enumerate()
                .filter(|(i, _)| !state.selected.contains(i))
                .map(|(_, td)| td.size_bytes)
                .fold(0_u64, |a, b| a.saturating_add(b)),
            reference: state.reference_path(),
            items,
        }
    })
}

pub fn initial_progress(items: &[(usize, TargetDir, String)]) -> CleanProgress {
    CleanProgress {
        items: items
            .iter()
            .map(|(index, td, name)| CleanItem {
                index: *index,
                name: name.clone(),
                size_bytes: td.size_bytes,
                freed_bytes: 0,
                status: CleanItemStatus::Pending,
            })
            .collect(),
        freed_so_far: 0,
        current_path: String::new(),
        done_count: 0,
        total_count: items.len(),
    }
}

/// Everything the run accumulates, behind one lock because several targets are
/// coming off at once.
struct Run {
    progress: CleanProgress,
    removed: Vec<GroupSummary>,
    /// Slots that were picked up. Anything not in here was never touched.
    started: Vec<bool>,
}

#[allow(clippy::too_many_arguments)]
pub fn run_clean(
    items: Vec<(usize, TargetDir, String)>,
    skipped_count: usize,
    skipped_bytes: u64,
    reference: Option<PathBuf>,
    before: Option<DiskSpace>,
    progress: CleanProgress,
    on_progress: impl Fn(&CleanProgress) + Sync,
    stop: &AtomicBool,
) -> DoneSummary {
    let started = Instant::now();
    let targets: Vec<TargetDir> = items.iter().map(|(_, target, _)| target.clone()).collect();
    let names: Vec<String> = items.into_iter().map(|(_, _, name)| name).collect();
    let run = Mutex::new(Run {
        progress,
        removed: Vec::new(),
        started: vec![false; targets.len()],
    });

    remove_targets(
        &targets,
        || !stop.load(Ordering::Relaxed),
        |slot| {
            let snapshot = {
                let mut run = run.lock().unwrap();
                run.begin(slot, &targets[slot]);
                run.progress.clone()
            };
            on_progress(&snapshot);
        },
        |slot, removal| {
            let snapshot = {
                let mut run = run.lock().unwrap();
                run.settle(slot, &targets[slot], &names[slot], removal);
                run.progress.clone()
            };
            on_progress(&snapshot);
        },
    );

    let mut run = run.into_inner().unwrap_or_else(|err| err.into_inner());
    run.leave_untouched();
    on_progress(&run.progress);
    summarise(run, started.elapsed(), reference, before, skipped_count, skipped_bytes)
}

/// What the run gave back, read off the volume itself rather than off the sizes
/// the scan reported — those were a claim about the past.
fn summarise(
    run: Run,
    duration: std::time::Duration,
    reference: Option<PathBuf>,
    before: Option<DiskSpace>,
    skipped_count: usize,
    skipped_bytes: u64,
) -> DoneSummary {
    let after = reference.as_deref().and_then(disk_space);
    let free_before = before.map(|d| d.free_bytes).unwrap_or(0);
    let free_after = after.map(|d| d.free_bytes).unwrap_or(free_before);
    DoneSummary {
        freed_bytes: free_after.saturating_sub(free_before),
        duration,
        free_before,
        free_after,
        total_bytes: before.or(after).map(|d| d.total_bytes).unwrap_or(0),
        troubled_count: run.progress.troubled_count(),
        removed: run.removed,
        skipped_count,
        skipped_bytes,
    }
}

impl Run {
    fn begin(&mut self, slot: usize, target: &TargetDir) {
        self.started[slot] = true;
        if let Some(item) = self.progress.items.get_mut(slot) {
            item.status = CleanItemStatus::Active;
        }
        self.progress.current_path = target.path.display().to_string();
    }

    /// Record what actually happened. A removal that only went part way is
    /// credited with the bytes it really returned and is never called done.
    fn settle(&mut self, slot: usize, target: &TargetDir, name: &str, removal: Removal) {
        let freed = removal.freed_bytes(target.size_bytes);
        let status = match removal {
            Removal::Gone => CleanItemStatus::Done,
            Removal::Partial { .. } => CleanItemStatus::Partial,
            Removal::Refused(_) => CleanItemStatus::Failed,
        };
        if let Some(item) = self.progress.items.get_mut(slot) {
            item.status = status;
            item.freed_bytes = freed;
        }
        if freed > 0 {
            bump(&mut self.removed, target.kind.group(), freed);
        }
        self.progress.freed_so_far = self.progress.freed_so_far.saturating_add(freed);
        self.progress.done_count = self
            .progress
            .items
            .iter()
            .filter(|item| item.status == CleanItemStatus::Done)
            .count();
        report(name, status, freed);
    }

    /// Everything the run never picked up. Stopping leaves these completely
    /// alone, which is the only thing "stop" is allowed to mean.
    fn leave_untouched(&mut self) {
        for (slot, item) in self.progress.items.iter_mut().enumerate() {
            if !self.started.get(slot).copied().unwrap_or(false) {
                item.status = CleanItemStatus::Skipped;
            }
        }
    }
}

fn report(name: &str, status: CleanItemStatus, freed: u64) {
    match status {
        CleanItemStatus::Done => println!("Cleaned {name} ({})", human_size(freed)),
        CleanItemStatus::Partial => {
            println!("Partly cleaned {name} ({} freed, the rest is still there)", human_size(freed))
        }
        _ => println!("Could not clean {name}"),
    }
}

fn bump(removed: &mut Vec<GroupSummary>, group: ArtifactGroup, bytes: u64) {
    if let Some(row) = removed.iter_mut().find(|row| row.group == group) {
        row.count += 1;
        row.bytes = row.bytes.saturating_add(bytes);
    } else {
        removed.push(GroupSummary { group, count: 1, bytes });
    }
}
