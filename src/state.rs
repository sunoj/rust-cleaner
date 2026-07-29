// Shared application state for the WD-40 menu bar app.
// Exports: `AppState`, `with_state`, `with_state_ret`, `APP_STATE`.
// Deps: objc2_app_kit, crate::updater, wd40 lib.

use crate::updater::Updater;
use objc2::rc::Retained;
use objc2_app_kit::NSStatusItem;
use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;
use wd40::config::Config;
use wd40::disk::{disk_space, sum_bytes, DiskSpace};
use wd40::scanner::TargetDir;

const SECONDS_PER_DAY: u64 = 86_400;

thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

pub(crate) struct AppState {
    pub config: Config,
    pub targets: Vec<TargetDir>,
    pub status_item: Retained<NSStatusItem>,
    pub updater: Option<Updater>,
}

impl AppState {
    pub fn total_size(&self) -> u64 {
        sum_bytes(self.targets.iter().map(|t| t.size_bytes))
    }

    pub fn max_age(&self) -> Duration {
        Duration::from_secs(self.config.max_age_days.saturating_mul(SECONDS_PER_DAY))
    }

    /// A still-existing path on the volume being reported on.
    pub fn reference_path(&self) -> Option<PathBuf> {
        self.config
            .scan_dirs
            .iter()
            .find(|path| path.exists())
            .or_else(|| self.targets.first().map(|target| &target.path))
            .cloned()
    }

    /// Capacity of the volume holding the first scan dir that still exists.
    pub fn disk_stats(&self) -> Option<DiskSpace> {
        self.reference_path().as_deref().and_then(disk_space)
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
