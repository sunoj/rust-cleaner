// Background scan/clean orchestration for the WD-40 popover app.
// Exports: scan/clean spawners, timer control, main-thread completion hooks.
// Deps: objc2, crate::{live, mainthread, names, popover, state, tasks_clean}.

use crate::live;
use crate::popover;
use crate::state::{
    with_state, with_state_ret, CleanProgress, DoneSummary, UiScreen,
};
use crate::tasks_clean::{self, gather_job, initial_progress, CleanJob};
use objc2::rc::Retained;
use objc2::sel;
use objc2_foundation::{MainThreadMarker, NSTimer};
use crate::mainthread::{
    clean_done_trampoline, dispatch_to_main, progress_trampoline, scan_done_trampoline, schedule,
    sizes_done_trampoline, sizes_tick_trampoline, stop_timer,
};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use wd40::disk::disk_space;
use wd40::discover::scan_discover;
use wd40::scanner::TargetDir;
use wd40::sizes::size_targets;

/// How often the app rescans on its own, in seconds. Ten minutes rather than
/// five: nothing on screen goes stale enough in that time to be worth waking
/// the disk for, and the figures that cost the most to measure do not change
/// at all between two of them.
const AUTO_SCAN_INTERVAL: f64 = 10.0 * 60.0;

/// A clean that takes a moment is worth watching, and one that takes none at
/// all used to flash past before anything could be read. The floor is on the
/// screen alone: removal starts the instant it is asked for, nothing waits on
/// it, and every figure on the held screen is the finished one.
const MIN_CLEAN_SCREEN: f64 = 2.0;

static CLEANING: AtomicBool = AtomicBool::new(false);
static SCANNING: AtomicBool = AtomicBool::new(false);
static POST_SCAN_CLEAN: AtomicBool = AtomicBool::new(false);
static STOP_AFTER: AtomicBool = AtomicBool::new(false);
static SCAN_RESULT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);
static RECLAIM_RESULT: Mutex<Option<wd40::reclaim::Reclaim>> = Mutex::new(None);
static DONE_RESULT: Mutex<Option<DoneSummary>> = Mutex::new(None);
/// The finished summary, waiting for the cleaning screen to have been up long
/// enough to have been read.
static PENDING_DONE: Mutex<Option<DoneSummary>> = Mutex::new(None);
static CLEAN_SHOWN: Mutex<Option<Instant>> = Mutex::new(None);
static PROGRESS: Mutex<Option<CleanProgress>> = Mutex::new(None);
/// Sizes that have settled but not yet reached the screen.
static SIZES: Mutex<Vec<(usize, u64)>> = Mutex::new(Vec::new());
/// One pending hop to the main thread at a time, however fast sizes arrive.
static SIZE_HOP: AtomicBool = AtomicBool::new(false);
static PROGRESS_HOP: AtomicBool = AtomicBool::new(false);

thread_local! {
    static AUTO_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    static SCAN_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    /// Runs out the rest of the cleaning screen's minimum time. It carries no
    /// work; the run is already over by the time it is scheduled.
    static HOLD_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

pub fn is_busy() -> bool {
    CLEANING.load(Ordering::Relaxed) || SCANNING.load(Ordering::Relaxed)
}

/// True once the user has asked the current run to stop starting new targets.
pub fn stop_requested() -> bool {
    STOP_AFTER.load(Ordering::Relaxed)
}

pub fn start_scan(then_clean: bool) {
    let Some(config) = with_state_ret(|state| state.config.clone()) else { return };
    if SCANNING.swap(true, Ordering::Relaxed) {
        return;
    }
    if then_clean {
        POST_SCAN_CLEAN.store(true, Ordering::Relaxed);
    }
    if let Some(mtm) = MainThreadMarker::new() {
        with_state(|state| {
            if state.screen == UiScreen::Done {
                state.screen = UiScreen::Scan;
            }
        });
        popover::refresh(mtm);
    }
    std::thread::spawn(move || {
        *SCAN_RESULT.lock().unwrap() = Some(scan_discover(&config));
        dispatch_to_main(scan_done_trampoline);
    });
}

pub fn on_scan_done(mtm: MainThreadMarker) {
    // A new scan replaces the rows, so the remembered offset would point at
    // targets that are no longer there.
    crate::scrolling::reset_scroll();
    let Some(targets) = SCAN_RESULT.lock().unwrap().take() else {
        SCANNING.store(false, Ordering::Relaxed);
        return;
    };
    SIZES.lock().unwrap().clear();
    with_state(|state| {
        state.targets = targets;
        state.measured.clear();
        state.reset_selection();
    });
    popover::refresh(mtm);
    let pending: Vec<TargetDir> =
        with_state_ret(|state| state.targets.clone()).unwrap_or_default();
    std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            size_targets(&pending, |sized| {
                SIZES.lock().unwrap().push((sized.index, sized.bytes));
                if !SIZE_HOP.swap(true, Ordering::SeqCst) {
                    dispatch_to_main(sizes_tick_trampoline);
                }
            });
        }));
        dispatch_to_main(sizes_done_trampoline);
    });
}

/// Sizes that settled since the last hop. Patches the rows in place — the list
/// keeps its order and its scroll position until the whole pass is done.
pub fn on_sizes_tick(mtm: MainThreadMarker) {
    SIZE_HOP.store(false, Ordering::SeqCst);
    let arrived = std::mem::take(&mut *SIZES.lock().unwrap());
    if arrived.is_empty() {
        return;
    }
    let indices: Vec<usize> = arrived.iter().map(|(index, _)| *index).collect();
    with_state(|state| {
        for (index, bytes) in arrived {
            state.apply_size(index, bytes);
        }
    });
    if !live::sizes_arrived(&indices, mtm) {
        popover::refresh(mtm);
    }
    popover::refresh_status(mtm);
}

pub fn on_sizes_done(mtm: MainThreadMarker) {
    SCANNING.store(false, Ordering::Relaxed);
    on_sizes_tick(mtm);
    // Only now is there a full set of figures to sort by, so this is the one
    // point at which rows are allowed to move.
    with_state(|state| state.finish_sizing());
    popover::refresh(mtm);
    start_reclaim();
    #[cfg(debug_assertions)]
    crate::screenshot::maybe_start(mtm);
    if POST_SCAN_CLEAN.swap(false, Ordering::Relaxed) {
        spawn_clean_selected();
    }
}

/// Work out what the targets really occupy on the device. One `open` per file
/// over every target is 71 s on this Mac, so it runs after the sizes are on
/// screen rather than before, at background priority, and the totals use summed
/// sizes until it lands.
///
/// It is also skipped outright when nothing has moved. An automatic rescan every
/// ten minutes usually finds the same directories at the same sizes, and reading
/// four hundred thousand files again to reach the same answer is the kind of
/// work a menu bar app has no business doing.
fn start_reclaim() {
    let targets = with_state_ret(|state| state.targets.clone()).unwrap_or_default();
    if targets.is_empty() {
        return;
    }
    std::thread::spawn(move || {
        // `measure` answers from the two-level store when the same directories
        // come back at the same sizes, so this is free far more often than it
        // is expensive — including on the first scan after a restart.
        *RECLAIM_RESULT.lock().unwrap() = wd40::reclaim::Reclaim::measure(&targets);
        wd40::cache::flush();
        dispatch_to_main(crate::mainthread::reclaim_done_trampoline);
    });
}

/// The accounting is in: totals stop being a sum and start being a promise.
pub fn on_reclaim_done(mtm: MainThreadMarker) {
    let Some(found) = RECLAIM_RESULT.lock().unwrap().take() else { return };
    with_state(|state| state.reclaim = Some(found));
    popover::refresh(mtm);
    popover::refresh_status(mtm);
}

/// Clean exactly the checked items. Irreversible; nothing else is touched.
pub fn spawn_clean_selected() {
    let Some(job) = gather_job() else { return };
    let CleanJob { items, skipped_count, skipped_bytes, reference } = job;
    if items.is_empty() || CLEANING.swap(true, Ordering::Relaxed) {
        return;
    }
    STOP_AFTER.store(false, Ordering::Relaxed);
    let before = reference.as_deref().and_then(disk_space);
    let progress = initial_progress(&items);
    *PROGRESS.lock().unwrap() = Some(progress.clone());
    with_state(|state| {
        state.screen = UiScreen::Cleaning;
        state.cleaning = Some(progress.clone());
        state.done = None;
    });
    if let Some(mtm) = MainThreadMarker::new() {
        // A run of its own cancels whatever the last one was still showing.
        stop_timer(&HOLD_TIMER);
        *PENDING_DONE.lock().unwrap() = None;
        *CLEAN_SHOWN.lock().unwrap() = Some(Instant::now());
        popover::refresh(mtm);
    }

    std::thread::spawn(move || {
        let summary = tasks_clean::run_clean(
            items,
            skipped_count,
            skipped_bytes,
            reference,
            before,
            progress,
            |p| {
                *PROGRESS.lock().unwrap() = Some(p.clone());
                if !PROGRESS_HOP.swap(true, Ordering::SeqCst) {
                    dispatch_to_main(progress_trampoline);
                }
            },
            &STOP_AFTER,
        );
        *DONE_RESULT.lock().unwrap() = Some(summary);
        dispatch_to_main(clean_done_trampoline);
    });
}

pub fn on_progress(mtm: MainThreadMarker) {
    PROGRESS_HOP.store(false, Ordering::SeqCst);
    let snapshot = PROGRESS.lock().unwrap().clone();
    with_state(|state| state.cleaning = snapshot);
    if !live::progress_changed(mtm) {
        popover::refresh(mtm);
    }
}

pub fn request_stop() {
    STOP_AFTER.store(true, Ordering::Relaxed);
    // Say so straight away: the button has to stop offering what it has
    // already been asked to do.
    if let Some(mtm) = MainThreadMarker::new() {
        if !live::progress_changed(mtm) {
            popover::refresh(mtm);
        }
    }
}

pub fn on_clean_done(mtm: MainThreadMarker) {
    CLEANING.store(false, Ordering::Relaxed);
    *PENDING_DONE.lock().unwrap() = DONE_RESULT.lock().unwrap().take();
    // The last thing the run published is what really happened. It goes on
    // screen before anything is held back, so a held screen is a finished one:
    // the counts, the bar and the plate all read done because they are.
    let settled = PROGRESS.lock().unwrap().clone();
    with_state(|state| state.cleaning = settled);
    let Some(seconds) = hold_left() else { return show_result(mtm) };
    if !live::progress_changed(mtm) {
        popover::refresh(mtm);
    }
    schedule(&HOLD_TIMER, seconds, sel!(cleanHoldTick:), false);
}

/// Seconds the cleaning screen has still to run, or None when it owes none —
/// a job that already took longer than the floor, or Reduce Motion, which is
/// never made to wait for a screen.
fn hold_left() -> Option<f64> {
    if crate::metal::reduce_motion() {
        return None;
    }
    let shown = (*CLEAN_SHOWN.lock().unwrap())?;
    let left = MIN_CLEAN_SCREEN - shown.elapsed().as_secs_f64();
    (left > 0.05).then_some(left)
}

/// Hand the finished run to the done screen. Called by the floor's timer, and
/// straight away when there is no floor left to run out.
pub fn show_result(mtm: MainThreadMarker) {
    stop_timer(&HOLD_TIMER);
    // A new run started while this one was being shown; its screen wins.
    if CLEANING.load(Ordering::Relaxed) {
        return;
    }
    let summary = PENDING_DONE.lock().unwrap().take();
    *PROGRESS.lock().unwrap() = None;
    *CLEAN_SHOWN.lock().unwrap() = None;
    with_state(|state| {
        state.cleaning = None;
        state.done = summary;
        state.screen = UiScreen::Done;
    });
    popover::refresh(mtm);
}

/// The button on the held screen. It only skips the wait — there is nothing to
/// show until the run has handed its summary over.
pub fn show_result_now(mtm: MainThreadMarker) {
    if PENDING_DONE.lock().unwrap().is_some() {
        show_result(mtm);
    }
}

pub fn start_auto_scan() {
    schedule(&SCAN_TIMER, AUTO_SCAN_INTERVAL, sel!(autoScanTick:), true);
}

pub fn start_auto_clean(hours: u64) {
    stop_auto_clean();
    schedule(&AUTO_TIMER, hours as f64 * 3600.0, sel!(autoCleanTick:), true);
}

pub fn stop_auto_clean() {
    stop_timer(&AUTO_TIMER);
}
