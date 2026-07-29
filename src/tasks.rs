// Background scan/clean orchestration for the WD-40 menu bar app.
// Exports: scan/clean spawners, timer control, and main-thread completion hooks.
// Deps: objc2, libdispatch FFI, crate::{menu, with_state}.

use crate::menu::{add_caption, refresh_menu};
use crate::{with_state, with_state_ret, MenuHandler, HANDLER};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{msg_send, sel, MainThreadOnly};
use objc2_app_kit::NSMenu;
use objc2_foundation::{ns_string, MainThreadMarker, NSString, NSTimer};
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use wd40::cleaner::{clean_all, clean_old};
use wd40::scanner::{human_size, scan_discover, scan_sizes, TargetDir};

/// How often the app rescans on its own, in seconds.
const AUTO_SCAN_INTERVAL: f64 = 5.0 * 60.0;
/// Minimum time the sweeping animation stays visible, so it reads as work done.
const MIN_CLEAN_ANIMATION: Duration = Duration::from_secs(2);

static CLEANING: AtomicBool = AtomicBool::new(false);
static SCANNING: AtomicBool = AtomicBool::new(false);
static POST_SCAN_CLEAN: AtomicBool = AtomicBool::new(false);
static ANIM_FRAME: AtomicUsize = AtomicUsize::new(0);
static SCAN_RESULT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);
static SIZES_RESULT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);

thread_local! {
    static ANIM_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    static AUTO_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    static SCAN_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
    static SHINE_TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

pub fn is_busy() -> bool {
    CLEANING.load(Ordering::Relaxed) || SCANNING.load(Ordering::Relaxed)
}

/// Dispatch discovery to a background thread. `then_clean` chains auto-clean after sizing.
pub fn start_scan(then_clean: bool) {
    let Some(config) = with_state_ret(|state| state.config.clone()) else { return };
    if SCANNING.swap(true, Ordering::Relaxed) {
        return;
    }
    if then_clean {
        POST_SCAN_CLEAN.store(true, Ordering::Relaxed);
    }
    if let Some(mtm) = MainThreadMarker::new() {
        show_scanning_menu(mtm);
    }
    std::thread::spawn(move || {
        *SCAN_RESULT.lock().unwrap() = Some(scan_discover(&config));
        dispatch_to_main(scan_done_trampoline);
    });
}

fn show_scanning_menu(mtm: MainThreadMarker) {
    with_state(|state| {
        if let Some(button) = state.status_item.button(mtm) {
            button.setImage(None);
            button.setTitle(&NSString::from_str("\u{1f50d}"));
        }
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("WD-40"));
        menu.setAutoenablesItems(false);
        add_caption(&menu, "Scanning\u{2026}", mtm);
        state.status_item.setMenu(Some(&menu));
    });
}

/// Phase 1 finished: show the discovered paths, then size them in the background.
pub fn on_scan_done(mtm: MainThreadMarker) {
    let Some(targets) = SCAN_RESULT.lock().unwrap().take() else {
        SCANNING.store(false, Ordering::Relaxed);
        return;
    };
    with_state(|state| {
        state.targets = targets;
        refresh_menu(state, mtm);
    });
    let mut pending: Vec<TargetDir> = with_state_ret(|state| {
        state
            .targets
            .iter()
            .map(|td| TargetDir { size_bytes: 0, ..td.clone() })
            .collect()
    })
    .unwrap_or_default();
    std::thread::spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scan_sizes(&mut pending)));
        *SIZES_RESULT.lock().unwrap() = Some(pending);
        dispatch_to_main(sizes_done_trampoline);
    });
}

/// Phase 2 finished: publish sizes and run the queued auto-clean, if any.
pub fn on_sizes_done(mtm: MainThreadMarker) {
    SCANNING.store(false, Ordering::Relaxed);
    if let Some(targets) = SIZES_RESULT.lock().unwrap().take() {
        with_state(|state| {
            state.targets = targets;
            refresh_menu(state, mtm);
        });
    }
    if !POST_SCAN_CLEAN.swap(false, Ordering::Relaxed) {
        return;
    }
    if let Some((targets, max_age)) = with_state_ret(|state| (state.targets.clone(), state.max_age())) {
        spawn_clean_old(targets, max_age, "Auto Clean");
    }
}

pub fn spawn_clean_all(targets: Vec<TargetDir>, label: &'static str) {
    start_clean(move || {
        let result = clean_all(&targets);
        if result.removed_count > 0 {
            println!("{label} freed {} from {} dirs", human_size(result.freed_bytes), result.removed_count);
        }
    });
}

pub fn spawn_clean_old(targets: Vec<TargetDir>, max_age: Duration, label: &'static str) {
    start_clean(move || {
        let result = clean_old(&targets, max_age);
        if result.removed_count > 0 {
            println!("{label} freed {} from {} dirs", human_size(result.freed_bytes), result.removed_count);
        }
    });
}

pub fn spawn_remove(path: std::path::PathBuf, size: u64) {
    start_clean(move || match std::fs::remove_dir_all(&path) {
        Ok(()) => println!("Cleaned {} ({})", path.display(), human_size(size)),
        Err(err) => eprintln!("Failed {}: {}", path.display(), err),
    });
}

fn start_clean<F: FnOnce() + Send + 'static>(work: F) {
    if CLEANING.swap(true, Ordering::Relaxed) {
        return;
    }
    start_anim();
    std::thread::spawn(move || {
        let started = std::time::Instant::now();
        work();
        if let Some(remaining) = MIN_CLEAN_ANIMATION.checked_sub(started.elapsed()) {
            std::thread::sleep(remaining);
        }
        dispatch_to_main(clean_done_trampoline);
    });
}

/// Clean finished: show the sparkle, then rescan a second later.
pub fn on_clean_done(mtm: MainThreadMarker) {
    stop_timer(&ANIM_TIMER);
    CLEANING.store(false, Ordering::Relaxed);
    with_state(|state| {
        if let Some(button) = state.status_item.button(mtm) {
            button.setImage(None);
            button.setTitle(&NSString::from_str("\u{2728}"));
        }
    });
    schedule(&SHINE_TIMER, 1.0, sel!(shineTick:), false);
}

pub fn on_shine_done() {
    stop_timer(&SHINE_TIMER);
    start_scan(false);
}

pub fn tick_anim(mtm: MainThreadMarker) {
    let frame = ANIM_FRAME.fetch_add(1, Ordering::Relaxed);
    let dots = match frame % 4 {
        0 => "\u{1f9f9}",
        1 => "\u{1f9f9} .",
        2 => "\u{1f9f9} ..",
        _ => "\u{1f9f9} ...",
    };
    with_state(|state| {
        if let Some(button) = state.status_item.button(mtm) {
            button.setImage(None);
            button.setTitle(&NSString::from_str(dots));
        }
    });
}

fn start_anim() {
    ANIM_FRAME.store(0, Ordering::Relaxed);
    if let Some(mtm) = MainThreadMarker::new() {
        with_state(|state| {
            if let Some(button) = state.status_item.button(mtm) {
                button.setImage(None);
                button.setTitle(&NSString::from_str("\u{1f9f9}"));
            }
        });
    }
    schedule(&ANIM_TIMER, 0.25, sel!(animTick:), true);
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

// libdispatch FFI — _dispatch_main_q is the actual symbol on macOS
#[link(name = "System", kind = "dylib")]
extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}
