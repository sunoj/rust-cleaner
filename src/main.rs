// Entry point for the macOS WD-40 menu bar app.
// Uses objc2 AppKit bindings for native macOS status bar integration.
mod cleaner;
mod config;
mod scanner;
mod icon;
mod menu;

use cleaner::{clean_all, clean_old};
use config::Config;
use menu::refresh_menu;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSMenuItem, NSStatusBar,
    NSStatusItem,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSString, NSTimer};
use scanner::{human_size, scan, ArtifactGroup, TargetDir};
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const SECONDS_PER_DAY: u64 = 86_400;

static CLEANING: AtomicBool = AtomicBool::new(false);
static SCANNING: AtomicBool = AtomicBool::new(false);
static POST_SCAN_CLEAN: AtomicBool = AtomicBool::new(false);
static ANIM_FRAME: AtomicUsize = AtomicUsize::new(0);
static SCAN_RESULT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);

thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = RefCell::new(None);
    pub(crate) static HANDLER: RefCell<Option<Retained<MenuHandler>>> = RefCell::new(None);
    static ANIM_TIMER: RefCell<Option<Retained<NSTimer>>> = RefCell::new(None);
    static AUTO_TIMER: RefCell<Option<Retained<NSTimer>>> = RefCell::new(None);
    static SHINE_TIMER: RefCell<Option<Retained<NSTimer>>> = RefCell::new(None);
}

struct AppState {
    config: Config,
    targets: Vec<TargetDir>,
    status_item: Retained<NSStatusItem>,
}

impl AppState {
    fn total_size(&self) -> u64 {
        self.targets.iter().map(|t| t.size_bytes).sum()
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "MenuHandler"]
    pub struct MenuHandler;

    impl MenuHandler {
        #[unsafe(method(handleCleanProject:))]
        fn handle_clean_project(&self, sender: &NSMenuItem) {
            let idx = sender.tag() as usize;
            let work = with_state_ret(|state| {
                state.targets.get(idx).map(|td| (td.path.clone(), td.size_bytes))
            }).flatten();
            if let Some((path, size)) = work {
                start_clean(move || {
                    match std::fs::remove_dir_all(&path) {
                        Ok(()) => println!("Cleaned {} ({})", path.display(), human_size(size)),
                        Err(e) => eprintln!("Failed {}: {}", path.display(), e),
                    }
                });
            }
        }

        #[unsafe(method(handleCleanAll:))]
        fn handle_clean_all(&self, _sender: &NSMenuItem) {
            let targets: Vec<_> = with_state_ret(|state| {
                state.targets.iter().map(|t| (t.path.clone(), t.size_bytes, t.last_modified, t.kind)).collect()
            }).unwrap_or_default();
            start_clean(move || {
                let tds: Vec<TargetDir> = targets.into_iter()
                    .map(|(path, size_bytes, last_modified, kind)| TargetDir { path, size_bytes, last_modified, kind })
                    .collect();
                let r = clean_all(&tds);
                if r.removed_count > 0 {
                    println!("Clean All freed {} from {} dirs", human_size(r.freed_bytes), r.removed_count);
                }
            });
        }

        #[unsafe(method(handleCleanOld:))]
        fn handle_clean_old(&self, _sender: &NSMenuItem) {
            let work = with_state_ret(|state| {
                let targets: Vec<_> = state.targets.iter()
                    .map(|t| (t.path.clone(), t.size_bytes, t.last_modified, t.kind)).collect();
                let max_age = Duration::from_secs(state.config.max_age_days.saturating_mul(SECONDS_PER_DAY));
                (targets, max_age)
            });
            if let Some((targets, max_age)) = work {
                start_clean(move || {
                    let tds: Vec<TargetDir> = targets.into_iter()
                        .map(|(path, size_bytes, last_modified, kind)| TargetDir { path, size_bytes, last_modified, kind })
                        .collect();
                    let r = clean_old(&tds, max_age);
                    if r.removed_count > 0 {
                        println!("Clean Old freed {} from {} dirs", human_size(r.freed_bytes), r.removed_count);
                    }
                });
            }
        }

        #[unsafe(method(handleCleanGroup:))]
        fn handle_clean_group(&self, sender: &NSMenuItem) {
            let Some(group) = ArtifactGroup::from_tag(sender.tag()) else { return };
            let targets: Vec<_> = with_state_ret(|state| {
                state.targets.iter()
                    .filter(|t| t.kind.group() == group)
                    .map(|t| (t.path.clone(), t.size_bytes, t.last_modified, t.kind))
                    .collect()
            }).unwrap_or_default();
            let label = group.label();
            start_clean(move || {
                let tds: Vec<TargetDir> = targets.into_iter()
                    .map(|(path, size_bytes, last_modified, kind)| TargetDir { path, size_bytes, last_modified, kind })
                    .collect();
                let r = clean_all(&tds);
                if r.removed_count > 0 {
                    println!("Clean {} freed {} from {} dirs", label, human_size(r.freed_bytes), r.removed_count);
                }
            });
        }

        #[unsafe(method(handleGroupInfo:))]
        fn handle_group_info(&self, sender: &NSMenuItem) {
            let Some(group) = ArtifactGroup::from_tag(sender.tag()) else { return };
            let mtm = self.mtm();
            let alert = NSAlert::new(mtm);
            alert.setMessageText(&NSString::from_str(group.label()));
            alert.setInformativeText(&NSString::from_str(group.description()));
            alert.setAlertStyle(NSAlertStyle::Informational);
            let app = NSApplication::sharedApplication(mtm);
            #[allow(deprecated)]
            app.activateIgnoringOtherApps(true);
            alert.runModal();
        }

        #[unsafe(method(handleRescan:))]
        fn handle_rescan(&self, _sender: &NSMenuItem) {
            start_scan(false);
        }

        #[unsafe(method(handleSetAutoInterval:))]
        fn handle_set_auto_interval(&self, sender: &NSMenuItem) {
            let hours = sender.tag() as u64;
            let mtm = self.mtm();
            with_state(|state| {
                state.config.auto_clean_hours = hours;
                state.config.save();
                if hours > 0 {
                    start_auto_clean(hours);
                } else {
                    stop_auto_clean();
                }
                refresh_menu(state, mtm);
            });
        }

        #[unsafe(method(handleSetMaxAge:))]
        fn handle_set_max_age(&self, sender: &NSMenuItem) {
            let days = sender.tag() as u64;
            let mtm = self.mtm();
            with_state(|state| {
                state.config.max_age_days = days;
                state.config.save();
                refresh_menu(state, mtm);
            });
        }

        #[unsafe(method(animTick:))]
        fn anim_tick(&self, _sender: &NSTimer) {
            let mtm = self.mtm();
            let frame = ANIM_FRAME.fetch_add(1, Ordering::Relaxed);
            let dots = match frame % 4 {
                0 => "🧹",
                1 => "🧹 .",
                2 => "🧹 ..",
                _ => "🧹 ...",
            };
            with_state(|state| {
                if let Some(button) = state.status_item.button(mtm) {
                    button.setImage(None);
                    button.setTitle(&NSString::from_str(dots));
                }
            });
        }

        #[unsafe(method(autoCleanTick:))]
        fn auto_clean_tick(&self, _sender: &NSTimer) {
            if CLEANING.load(Ordering::Relaxed) || SCANNING.load(Ordering::Relaxed) {
                return;
            }
            start_scan(true);
        }

        #[unsafe(method(shineTick:))]
        fn shine_tick(&self, _sender: &NSTimer) {
            // Fire once — stop timer, then trigger background rescan
            SHINE_TIMER.with(|cell| {
                if let Some(timer) = cell.borrow_mut().take() {
                    timer.invalidate();
                }
            });
            start_scan(false);
        }

        #[unsafe(method(scanDone:))]
        fn scan_done(&self, _sender: *mut AnyObject) {
            let mtm = self.mtm();
            SCANNING.store(false, Ordering::Relaxed);
            let results = SCAN_RESULT.lock().unwrap().take();
            if let Some(targets) = results {
                with_state(|state| {
                    state.targets = targets;
                    refresh_menu(state, mtm);
                });
            }
            // If auto-clean requested this scan, trigger clean now
            if POST_SCAN_CLEAN.swap(false, Ordering::Relaxed) {
                let work = with_state_ret(|state| {
                    let targets: Vec<_> = state.targets.iter()
                        .map(|t| (t.path.clone(), t.size_bytes, t.last_modified, t.kind)).collect();
                    let max_age = Duration::from_secs(state.config.max_age_days.saturating_mul(SECONDS_PER_DAY));
                    (targets, max_age)
                });
                if let Some((targets, max_age)) = work {
                    start_clean(move || {
                        let tds: Vec<TargetDir> = targets.into_iter()
                            .map(|(path, size_bytes, last_modified, kind)| TargetDir { path, size_bytes, last_modified, kind })
                            .collect();
                        let r = clean_old(&tds, max_age);
                        if r.removed_count > 0 {
                            println!("Auto Clean freed {} from {} dirs", human_size(r.freed_bytes), r.removed_count);
                        }
                    });
                }
            }
        }

        #[unsafe(method(cleanDone:))]
        fn clean_done(&self, _sender: *mut AnyObject) {
            let mtm = self.mtm();
            stop_anim();
            CLEANING.store(false, Ordering::Relaxed);
            // Show ✨ on status bar, then shineTick triggers background rescan after 1s
            with_state(|state| {
                if let Some(button) = state.status_item.button(mtm) {
                    button.setImage(None);
                    button.setTitle(&NSString::from_str("✨"));
                }
            });
            HANDLER.with(|cell| {
                if let Some(handler) = cell.borrow().as_ref() {
                    let target: &AnyObject = unsafe {
                        &*(handler.as_ref() as *const MenuHandler as *const AnyObject)
                    };
                    let timer = unsafe {
                        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                            1.0,
                            target,
                            sel!(shineTick:),
                            None,
                            false,
                        )
                    };
                    SHINE_TIMER.with(|cell| *cell.borrow_mut() = Some(timer));
                }
            });
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: &NSMenuItem) {
            std::process::exit(0);
        }
    }
);

impl MenuHandler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// Dispatch scan to a background thread. Set `then_clean` to chain auto-clean after scan.
fn start_scan(then_clean: bool) {
    if SCANNING.swap(true, Ordering::Relaxed) {
        return;
    }
    if then_clean {
        POST_SCAN_CLEAN.store(true, Ordering::Relaxed);
    }
    let config = with_state_ret(|state| state.config.clone());
    let Some(config) = config else {
        SCANNING.store(false, Ordering::Relaxed);
        return;
    };
    std::thread::spawn(move || {
        let results = scan(&config);
        *SCAN_RESULT.lock().unwrap() = Some(results);
        unsafe {
            dispatch_async_f(
                std::ptr::addr_of!(_dispatch_main_q),
                std::ptr::null_mut(),
                scan_done_trampoline,
            );
        }
    });
}

extern "C" fn scan_done_trampoline(_ctx: *mut c_void) {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.borrow().as_ref() {
            let obj: &AnyObject = unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
            let _: () = unsafe { msg_send![obj, scanDone: std::ptr::null::<AnyObject>()] };
        }
    });
}

fn start_clean<F: FnOnce() + Send + 'static>(work: F) {
    if CLEANING.swap(true, Ordering::Relaxed) {
        return;
    }
    start_anim();
    std::thread::spawn(move || {
        let t0 = std::time::Instant::now();
        work();
        // Ensure animation is visible for at least 2 seconds
        let elapsed = t0.elapsed();
        if elapsed < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_secs(2) - elapsed);
        }
        // Dispatch back to main thread
        unsafe {
            dispatch_async_f(
                std::ptr::addr_of!(_dispatch_main_q),
                std::ptr::null_mut(),
                clean_done_trampoline,
            );
        }
    });
}

extern "C" fn clean_done_trampoline(_ctx: *mut c_void) {
    HANDLER.with(|cell| {
        if let Some(handler) = cell.borrow().as_ref() {
            let obj: &AnyObject = unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
            let _: () = unsafe { msg_send![obj, cleanDone: std::ptr::null::<AnyObject>()] };
        }
    });
}

fn start_anim() {
    ANIM_FRAME.store(0, Ordering::Relaxed);
    let mtm = MainThreadMarker::new().unwrap();
    with_state(|state| {
        if let Some(button) = state.status_item.button(mtm) {
            button.setImage(None);
            button.setTitle(&NSString::from_str("🧹"));
        }
    });
    HANDLER.with(|cell| {
        if let Some(handler) = cell.borrow().as_ref() {
            let target: &AnyObject = unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
            let timer = unsafe {
                NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                    0.25,
                    target,
                    sel!(animTick:),
                    None,
                    true,
                )
            };
            ANIM_TIMER.with(|cell| *cell.borrow_mut() = Some(timer));
        }
    });
}

fn stop_anim() {
    ANIM_TIMER.with(|cell| {
        if let Some(timer) = cell.borrow_mut().take() {
            timer.invalidate();
        }
    });
}

fn start_auto_clean(hours: u64) {
    stop_auto_clean();
    let interval = hours as f64 * 3600.0;
    HANDLER.with(|cell| {
        if let Some(handler) = cell.borrow().as_ref() {
            let target: &AnyObject = unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
            let timer = unsafe {
                NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                    interval,
                    target,
                    sel!(autoCleanTick:),
                    None,
                    true,
                )
            };
            AUTO_TIMER.with(|cell| *cell.borrow_mut() = Some(timer));
        }
    });
}

fn stop_auto_clean() {
    AUTO_TIMER.with(|cell| {
        if let Some(timer) = cell.borrow_mut().take() {
            timer.invalidate();
        }
    });
}

fn main() {
    let mtm = MainThreadMarker::new().expect("must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let status_bar = NSStatusBar::systemStatusBar();
    let status_item = status_bar.statusItemWithLength(-1.0);
    let handler = MenuHandler::new(mtm);
    let config = Config::load();
    let auto_hours = config.auto_clean_hours;

    HANDLER.with(|cell| *cell.borrow_mut() = Some(handler));
    APP_STATE.with(|cell| {
        *cell.borrow_mut() = Some(AppState {
            config,
            targets: Vec::new(),
            status_item,
        })
    });

    // Show icon immediately, scan in background
    with_state(|state| refresh_menu(state, mtm));
    start_scan(false);

    if auto_hours > 0 {
        start_auto_clean(auto_hours);
    }
    app.run();
}

fn with_state<F: FnOnce(&mut AppState)>(f: F) {
    APP_STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            f(state);
        }
    });
}

fn with_state_ret<F: FnOnce(&mut AppState) -> R, R>(f: F) -> Option<R> {
    APP_STATE.with(|cell| cell.borrow_mut().as_mut().map(f))
}

// libdispatch FFI — _dispatch_main_q is the actual symbol on macOS
#[link(name = "System", kind = "dylib")]
extern "C" {
    static _dispatch_main_q: c_void;
    fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
}
