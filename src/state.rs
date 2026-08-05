// Shared application state for the WD-40 menu bar popover.
// Exports: `AppState`, `UiScreen`, clean/done summaries, selection accessors.
// Deps: objc2_app_kit, crate::{selection, updater}, wd40 lib.

use crate::selection::{default_selection, selected_bytes};
use crate::updater::Updater;
use objc2::rc::Retained;
use objc2_app_kit::NSStatusItem;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use wd40::config::Config;
use wd40::disk::{disk_space, sum_bytes, DiskSpace};
use wd40::scanner::{ArtifactGroup, TargetDir};

const SECONDS_PER_DAY: u64 = 86_400;

thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiScreen {
    Scan,
    Cleaning,
    Done,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CleanItemStatus {
    Pending,
    Active,
    Done,
    Skipped,
}

#[derive(Clone)]
pub struct CleanItem {
    pub index: usize,
    pub name: String,
    pub size_bytes: u64,
    pub status: CleanItemStatus,
}

#[derive(Clone)]
pub struct CleanProgress {
    pub items: Vec<CleanItem>,
    pub freed_so_far: u64,
    pub current_path: String,
    pub done_count: usize,
    pub total_count: usize,
}

#[derive(Clone)]
pub struct GroupSummary {
    pub group: ArtifactGroup,
    pub count: usize,
    pub bytes: u64,
}

#[derive(Clone)]
pub struct DoneSummary {
    pub freed_bytes: u64,
    pub duration: Duration,
    pub free_before: u64,
    pub free_after: u64,
    pub total_bytes: u64,
    pub removed: Vec<GroupSummary>,
    pub skipped_count: usize,
    pub skipped_bytes: u64,
}

pub(crate) struct AppState {
    pub config: Config,
    pub targets: Vec<TargetDir>,
    pub selected: HashSet<usize>,
    pub show_all: bool,
    pub screen: UiScreen,
    pub cleaning: Option<CleanProgress>,
    pub done: Option<DoneSummary>,
    pub status_item: Retained<NSStatusItem>,
    pub updater: Option<Updater>,
}

impl AppState {
    pub fn total_size(&self) -> u64 {
        sum_bytes(self.targets.iter().map(|t| t.size_bytes))
    }

    pub fn selected_size(&self) -> u64 {
        selected_bytes(&self.targets, &self.selected)
    }

    pub fn max_age(&self) -> Duration {
        Duration::from_secs(self.config.max_age_days.saturating_mul(SECONDS_PER_DAY))
    }

    pub fn reference_path(&self) -> Option<PathBuf> {
        self.config
            .scan_dirs
            .iter()
            .find(|path| path.exists())
            .or_else(|| self.targets.first().map(|target| &target.path))
            .cloned()
    }

    pub fn disk_stats(&self) -> Option<DiskSpace> {
        self.reference_path().as_deref().and_then(disk_space)
    }

    pub fn reset_selection(&mut self) {
        self.selected = default_selection(&self.targets, self.config.max_age_days);
        self.show_all = false;
    }

    pub fn toggle_selected(&mut self, index: usize) {
        if !self.selected.insert(index) {
            self.selected.remove(&index);
        }
    }
}

pub(crate) fn install(state: AppState) {
    APP_STATE.with(|cell| *cell.borrow_mut() = Some(state));
}

pub(crate) fn with_state<F: FnOnce(&mut AppState)>(f: F) {
    APP_STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            f(state);
        }
    });
}

pub(crate) fn with_state_ret<F: FnOnce(&mut AppState) -> R, R>(f: F) -> Option<R> {
    APP_STATE.with(|cell| cell.borrow_mut().as_mut().map(f))
}
