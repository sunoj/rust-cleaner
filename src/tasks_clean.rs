// Selected-item clean job: progress updates + done summary (background thread).
// Exports: `run_clean`, `remove_one`. Called only from `tasks`.
// Deps: std, wd40::{disk, scanner}, crate::state.

use crate::state::{CleanItemStatus, CleanProgress, DoneSummary, GroupSummary};
use std::path::PathBuf;
use std::time::Instant;
use wd40::disk::{disk_space, DiskSpace};
use wd40::scanner::{human_size, ArtifactGroup, TargetDir};

pub fn run_clean(
    items: Vec<(usize, TargetDir, String)>,
    skipped_count: usize,
    skipped_bytes: u64,
    reference: Option<PathBuf>,
    before: Option<DiskSpace>,
    mut progress: CleanProgress,
    on_progress: impl Fn(&CleanProgress),
    stop: &std::sync::atomic::AtomicBool,
) -> DoneSummary {
    let started = Instant::now();
    let mut removed: Vec<GroupSummary> = Vec::new();
    let mut claimed = 0_u64;

    for (slot, (_index, td, name)) in items.into_iter().enumerate() {
        if stop.load(std::sync::atomic::Ordering::Relaxed) && slot > 0 {
            for item in progress.items.iter_mut().skip(slot) {
                if item.status == CleanItemStatus::Pending {
                    item.status = CleanItemStatus::Skipped;
                }
            }
            on_progress(&progress);
            break;
        }
        if let Some(item) = progress.items.get_mut(slot) {
            item.status = CleanItemStatus::Active;
        }
        progress.current_path = td.path.display().to_string();
        on_progress(&progress);

        if remove_one(&td) {
            claimed = claimed.saturating_add(td.size_bytes);
            bump(&mut removed, td.kind.group(), td.size_bytes);
            println!("Cleaned {} ({})", name, human_size(td.size_bytes));
        }
        if let Some(item) = progress.items.get_mut(slot) {
            item.status = CleanItemStatus::Done;
        }
        progress.freed_so_far = claimed;
        progress.done_count = progress
            .items
            .iter()
            .filter(|i| i.status == CleanItemStatus::Done)
            .count();
        on_progress(&progress);
    }

    let after = reference.as_deref().and_then(disk_space);
    let free_before = before.map(|d| d.free_bytes).unwrap_or(0);
    let free_after = after.map(|d| d.free_bytes).unwrap_or(free_before);
    DoneSummary {
        freed_bytes: free_after.saturating_sub(free_before),
        duration: started.elapsed(),
        free_before,
        free_after,
        total_bytes: before.or(after).map(|d| d.total_bytes).unwrap_or(0),
        removed,
        skipped_count,
        skipped_bytes,
    }
}

pub fn remove_one(td: &TargetDir) -> bool {
    if td.path.is_symlink() || !td.path.is_dir() {
        return false;
    }
    std::fs::remove_dir_all(&td.path).is_ok()
}

fn bump(removed: &mut Vec<GroupSummary>, group: ArtifactGroup, bytes: u64) {
    if let Some(row) = removed.iter_mut().find(|row| row.group == group) {
        row.count += 1;
        row.bytes = row.bytes.saturating_add(bytes);
    } else {
        removed.push(GroupSummary { group, count: 1, bytes });
    }
}
