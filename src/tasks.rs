// Background scan/clean orchestration for the WD-40 popover app.
// Exports: scan/clean spawners, timer control, main-thread completion hooks.
// Deps: objc2, libdispatch FFI, crate::{names, popover, state, tasks_clean}.

use crate::names::display_names;
use crate::tasks_clean;
use crate::popover;
use crate::state::{
    with_state, with_state_ret, CleanItem, CleanItemStatus, CleanProgress, DoneSummary, UiScreen,
};
use crate::{MenuHandler, HANDLER};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, sel};
use objc2_foundation::{MainThreadMarker, NSTimer};
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use wd40::disk::disk_space;
use wd40::scanner::{scan_discover, scan_sizes, TargetDir};

const AUTO_SCAN_INTERVAL: f64 = 5.0 * 60.0;

static CLEANING: AtomicBool = AtomicBool::new(false);
static SCANNING: AtomicBool = AtomicBool::new(false);
static POST_SCAN_CLEAN: AtomicBool = AtomicBool::new(false);
static STOP_AFTER: AtomicBool = AtomicBool::new(false);
static SCAN_RESULT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);
static SIZES_RESULT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);
static DONE_RESULT: Mutex<Option<DoneSummary>> = Mutex::new(None);
static PROGRESS: Mutex<Option<CleanProgress>> = Mutex::new(None);

thread_local! {
    static AUTO_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    static SCAN_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

pub fn is_busy() -> bool {
    CLEANING.load(Ordering::Relaxed) || SCANNING.load(Ordering::Relaxed)
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
    let Some(targets) = SCAN_RESULT.lock().unwrap().take() else {
        SCANNING.store(false, Ordering::Relaxed);
        return;
    };
    with_state(|state| {
        state.targets = targets;
        state.reset_selection();
    });
    popover::refresh(mtm);
    let mut pending: Vec<TargetDir> = with_state_ret(|state| {
        state.targets.iter().map(|td| TargetDir { size_bytes: 0, ..td.clone() }).collect()
    })
    .unwrap_or_default();
    std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scan_sizes(&mut pending)));
        *SIZES_RESULT.lock().unwrap() = Some(pending);
        dispatch_to_main(sizes_done_trampoline);
    });
}

pub fn on_sizes_done(mtm: MainThreadMarker) {
    SCANNING.store(false, Ordering::Relaxed);
    if let Some(targets) = SIZES_RESULT.lock().unwrap().take() {
        with_state(|state| {
            state.targets = targets;
            state.reset_selection();
        });
        popover::refresh(mtm);
    }
    crate::screenshot::maybe_start(mtm);
    if POST_SCAN_CLEAN.swap(false, Ordering::Relaxed) {
        spawn_clean_selected();
    }
}

/// Clean exactly the checked items. Irreversible; nothing else is touched.
pub fn spawn_clean_selected() {
    let Some(work) = with_state_ret(|state| {
        let names = display_names(&state.targets);
        let items: Vec<(usize, TargetDir, String)> = state
            .selected
            .iter()
            .copied()
            .filter_map(|i| state.targets.get(i).map(|td| (i, td.clone(), names[i].clone())))
            .collect();
        let skipped_count = state.targets.len().saturating_sub(items.len());
        let skipped_bytes = state
            .targets
            .iter()
            .enumerate()
            .filter(|(i, _)| !state.selected.contains(i))
            .map(|(_, td)| td.size_bytes)
            .fold(0_u64, |a, b| a.saturating_add(b));
        (items, skipped_count, skipped_bytes, state.reference_path())
    }) else {
        return;
    };
    let (items, skipped_count, skipped_bytes, reference) = work;
    if items.is_empty() || CLEANING.swap(true, Ordering::Relaxed) {
        return;
    }
    STOP_AFTER.store(false, Ordering::Relaxed);
    let before = reference.as_deref().and_then(disk_space);
    let progress = CleanProgress {
        items: items
            .iter()
            .map(|(index, td, name)| CleanItem {
                index: *index,
                name: name.clone(),
                size_bytes: td.size_bytes,
                status: CleanItemStatus::Pending,
            })
            .collect(),
        freed_so_far: 0,
        current_path: String::new(),
        done_count: 0,
        total_count: items.len(),
    };
    *PROGRESS.lock().unwrap() = Some(progress.clone());
    with_state(|state| {
        state.screen = UiScreen::Cleaning;
        state.cleaning = Some(progress.clone());
        state.done = None;
    });
    if let Some(mtm) = MainThreadMarker::new() {
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
                dispatch_to_main(progress_trampoline);
            },
            &STOP_AFTER,
        );
        *DONE_RESULT.lock().unwrap() = Some(summary);
        dispatch_to_main(clean_done_trampoline);
    });
}

pub fn on_progress(mtm: MainThreadMarker) {
    let snapshot = PROGRESS.lock().unwrap().clone();
    with_state(|state| state.cleaning = snapshot);
    popover::refresh(mtm);
}

pub fn request_stop() {
    STOP_AFTER.store(true, Ordering::Relaxed);
}

pub fn on_clean_done(mtm: MainThreadMarker) {
    CLEANING.store(false, Ordering::Relaxed);
    let summary = DONE_RESULT.lock().unwrap().take();
    *PROGRESS.lock().unwrap() = None;
    with_state(|state| {
        state.cleaning = None;
        state.done = summary;
        state.screen = UiScreen::Done;
    });
    popover::refresh(mtm);
    crate::screenshot::on_clean_finished(mtm);
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

type TimerSlot = std::thread::LocalKey<RefCell<Option<Retained<NSTimer>>>>;

fn schedule(slot: &'static TimerSlot, interval: f64, selector: objc2::runtime::Sel, repeats: bool) {
    HANDLER.with(|cell| {
        let Some(handler) = cell.borrow().as_ref().map(Retained::clone) else { return };
        let target: &AnyObject = unsafe { &*(&*handler as *const MenuHandler as *const AnyObject) };
        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                interval, target, selector, None, repeats,
            )
        };
        slot.with(|cell| *cell.borrow_mut() = Some(timer));
    });
}

fn stop_timer(slot: &'static TimerSlot) {
    slot.with(|cell| {
        if let Some(timer) = cell.borrow_mut().take() {
            timer.invalidate();
        }
    });
}

fn dispatch_to_main(work: extern "C" fn(*mut c_void)) {
    unsafe {
        dispatch_async_f(std::ptr::addr_of!(_dispatch_main_q), std::ptr::null_mut(), work);
    }
}

macro_rules! trampoline {
    ($name:ident, $selector:ident) => {
        extern "C" fn $name(_ctx: *mut c_void) {
            HANDLER.with(|cell| {
                if let Some(handler) = cell.borrow().as_ref() {
                    let obj: &AnyObject =
                        unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
                    let _: () = unsafe { msg_send![obj, $selector: std::ptr::null::<AnyObject>()] };
                }
            });
        }
    };
}

trampoline!(scan_done_trampoline, scanDone);
trampoline!(sizes_done_trampoline, sizesDone);
trampoline!(clean_done_trampoline, cleanDone);
trampoline!(progress_trampoline, cleanProgress);

#[link(name = "System", kind = "dylib")]
extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}
