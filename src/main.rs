// Entry point for the macOS WD-40 menu bar app.
// Uses objc2 AppKit bindings for native macOS status bar integration.
mod cleaner;
mod config;
mod scanner;

use cleaner::{clean_all, clean_old};
use config::Config;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{define_class, msg_send, sel, AnyThread, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSBezierPath, NSColor,
    NSCompositingOperation, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSImage,
    NSImageSymbolConfiguration, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSArray, NSDictionary, NSObject, NSPoint, NSRect,
    NSSize, NSAttributedString, NSString, NSTimer,
};
use scanner::{human_size, scan, ArtifactGroup, ArtifactKind, TargetDir};
use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const INFO_LIMIT: usize = 15;
const MAX_BAR: usize = 12;
const SECONDS_PER_DAY: u64 = 86_400;
const ICON_NORMAL: &str = "internaldrive";

static CLEANING: AtomicBool = AtomicBool::new(false);
static SCANNING: AtomicBool = AtomicBool::new(false);
static POST_SCAN_CLEAN: AtomicBool = AtomicBool::new(false);
static ANIM_FRAME: AtomicUsize = AtomicUsize::new(0);
static SCAN_RESULT: Mutex<Option<Vec<TargetDir>>> = Mutex::new(None);

thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = RefCell::new(None);
    static HANDLER: RefCell<Option<Retained<MenuHandler>>> = RefCell::new(None);
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

#[allow(deprecated)]
/// Create a "rusty" icon — the more disk used, the rustier it looks.
/// Uses two-tone palette (style B) + rust spot overlay (style D), clipped to icon shape.
fn rusty_icon(total_bytes: u64) -> Option<Retained<NSImage>> {
    let gb = total_bytes / (1024 * 1024 * 1024);
    let s = NSString::from_str(ICON_NORMAL);
    let base = NSImage::imageWithSystemSymbolName_accessibilityDescription(&s, None)?;

    if gb < 5 {
        base.setTemplate(true);
        return Some(base);
    }

    // Style B: two-tone palette colors based on rust severity
    let (c1, c2, spot_rgba, spot_count) = match gb {
        5..=19 => (
            (0.80, 0.50, 0.20), (0.90, 0.70, 0.40),  // light rust palette
            (0.75, 0.40, 0.12, 0.7), 10u32,
        ),
        20..=49 => (
            (0.70, 0.30, 0.10), (0.85, 0.50, 0.20),  // medium rust palette
            (0.60, 0.20, 0.06, 0.8), 20,
        ),
        _ => (
            (0.55, 0.15, 0.05), (0.75, 0.30, 0.10),  // heavy rust palette
            (0.45, 0.10, 0.03, 0.95), 35,
        ),
    };

    let color1 = NSColor::colorWithSRGBRed_green_blue_alpha(c1.0, c1.1, c1.2, 1.0);
    let color2 = NSColor::colorWithSRGBRed_green_blue_alpha(c2.0, c2.1, c2.2, 1.0);
    let palette = NSArray::from_retained_slice(&[color1, color2]);
    let config = NSImageSymbolConfiguration::configurationWithPaletteColors(&palette);
    let tinted = base.imageWithSymbolConfiguration(&config)?;
    tinted.setTemplate(false);

    // Draw icon + rust spots composited within icon alpha
    let size = tinted.size();
    let canvas = NSImage::initWithSize(NSImage::alloc(), size);
    canvas.lockFocus();

    // Draw base tinted icon
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
    tinted.drawInRect_fromRect_operation_fraction(
        rect, NSRect::ZERO, NSCompositingOperation::SourceOver, 1.0,
    );

    // Style D: rust spots clipped to icon shape via sourceAtop
    let mut seed: u64 = 42;
    for _ in 0..spot_count {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fx = (seed >> 33) as f64 / (u32::MAX as f64);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fy = (seed >> 33) as f64 / (u32::MAX as f64);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fr = (seed >> 33) as f64 / (u32::MAX as f64);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fj = (seed >> 33) as f64 / (u32::MAX as f64);

        let x = fx * size.width;
        let y = fy * size.height;
        let r = fr * 3.5 + 1.0;
        let jr = spot_rgba.0 + fj * 0.12;
        let jg = spot_rgba.1 + fj * 0.08;
        let jb = spot_rgba.2 + fj * 0.04;

        // Draw spot into a temp image, then composite sourceAtop
        let spot_img = NSImage::initWithSize(NSImage::alloc(), size);
        spot_img.lockFocus();
        let spot_color = NSColor::colorWithSRGBRed_green_blue_alpha(jr, jg, jb, spot_rgba.3);
        spot_color.setFill();
        let oval = NSRect::new(NSPoint::new(x - r, y - r), NSSize::new(r * 2.0, r * 2.0));
        NSBezierPath::bezierPathWithOvalInRect(oval).fill();
        spot_img.unlockFocus();

        spot_img.drawInRect_fromRect_operation_fraction(
            rect, NSRect::ZERO, NSCompositingOperation::SourceAtop, 1.0,
        );
    }

    canvas.unlockFocus();
    canvas.setTemplate(false);
    Some(canvas)
}

fn rust_text_color(total_bytes: u64) -> Retained<NSColor> {
    let gb = total_bytes / (1024 * 1024 * 1024);
    match gb {
        0..=4 => NSColor::labelColor(),
        5..=19 => NSColor::colorWithSRGBRed_green_blue_alpha(0.95, 0.70, 0.40, 1.0),
        20..=49 => NSColor::colorWithSRGBRed_green_blue_alpha(0.90, 0.50, 0.25, 1.0),
        _ => NSColor::colorWithSRGBRed_green_blue_alpha(0.80, 0.35, 0.15, 1.0),
    }
}

fn refresh_menu(state: &mut AppState, mtm: MainThreadMarker) {
    let total = state.total_size();

    if let Some(button) = state.status_item.button(mtm) {
        if let Some(img) = rusty_icon(total) {
            button.setImage(Some(&img));
        }
        if total < 1024 * 1024 * 1024 {
            button.setTitle(ns_string!(""));
        } else {
            let title_text = format!(" {}", human_size(total));
            let color = rust_text_color(total);
            let font = NSFont::menuBarFontOfSize(0.0);
            let color_obj: &AnyObject = unsafe { &*(&*color as *const NSColor as *const AnyObject) };
            let font_obj: &AnyObject = unsafe { &*(&*font as *const NSFont as *const AnyObject) };
            let fg_key: &NSString = unsafe { NSForegroundColorAttributeName };
            let font_key: &NSString = unsafe { NSFontAttributeName };
            let attrs = NSDictionary::<NSString, AnyObject>::from_slices::<NSString>(
                &[fg_key, font_key],
                &[color_obj, font_obj],
            );
            let attr_str = unsafe {
                NSAttributedString::new_with_attributes(&NSString::from_str(&title_text), &attrs)
            };
            button.setAttributedTitle(&attr_str);
        }
    }

    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Rust Cleaner"));
    menu.setAutoenablesItems(false);

    add_disabled(&menu, "Rust Cleaner", mtm);
    menu.addItem(&NSMenuItem::separatorItem(mtm));

    HANDLER.with(|cell| {
        let handler = cell.borrow();
        let handler = match handler.as_ref() {
            Some(h) => h,
            None => return,
        };
        let target: &AnyObject = unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };

        if state.targets.is_empty() {
            add_disabled(&menu, "No targets found", mtm);
        } else {
            let max_size = state.targets.iter().map(|t| t.size_bytes).max().unwrap_or(1);
            let mut shown = 0usize;

            for &group in ArtifactGroup::ALL {
                let items: Vec<(usize, &TargetDir)> = state.targets.iter()
                    .enumerate()
                    .filter(|(_, td)| td.kind.group() == group)
                    .collect();
                if items.is_empty() {
                    continue;
                }

                let group_size: u64 = items.iter().map(|(_, td)| td.size_bytes).sum();
                add_disabled(&menu, &format!("{} — {}", group.label(), human_size(group_size)), mtm);

                let info_item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str("  \u{24d8} Scan rules"),
                        Some(sel!(handleGroupInfo:)),
                        ns_string!(""),
                    )
                };
                info_item.setTag(group.tag());
                unsafe { info_item.setTarget(Some(target)) };
                menu.addItem(&info_item);

                let budget = INFO_LIMIT.saturating_sub(shown);
                let show_count = items.len().min(budget);
                for &(i, td) in items.iter().take(show_count) {
                    let bar_len = ((td.size_bytes as f64 / max_size as f64) * MAX_BAR as f64)
                        .ceil().max(1.0) as usize;
                    let title = format!("  {}  [{}]  —  {}  {}", project_name(td), td.kind.label(), human_size(td.size_bytes), "█".repeat(bar_len));

                    let item = unsafe {
                        NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(mtm),
                            &NSString::from_str(&title),
                            Some(sel!(handleCleanProject:)),
                            ns_string!(""),
                        )
                    };
                    item.setTag(i as isize);
                    unsafe { item.setTarget(Some(target)) };
                    menu.addItem(&item);
                    shown += 1;
                }
                if items.len() > show_count {
                    add_disabled(&menu, &format!("  ... {} more", items.len() - show_count), mtm);
                }

                let clean_label = format!("  Clean {} ({})", group.label(), human_size(group_size));
                let clean_item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(&clean_label),
                        Some(sel!(handleCleanGroup:)),
                        ns_string!(""),
                    )
                };
                clean_item.setTag(group.tag());
                unsafe { clean_item.setTarget(Some(target)) };
                menu.addItem(&clean_item);

                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
        }

        menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_disabled(&menu, &format!("Total: {} in {} projects", human_size(total), state.targets.len()), mtm);
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        add_action(&menu, &format!("Clean All ({})", human_size(total)), sel!(handleCleanAll:), target, mtm);
        add_action(&menu, &format!("Clean Old (>{}d)", state.config.max_age_days), sel!(handleCleanOld:), target, mtm);
        add_action(&menu, "Rescan", sel!(handleRescan:), target, mtm);
        // Auto Clean submenu
        let auto_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Auto Clean"));
        auto_menu.setAutoenablesItems(false);

        add_disabled(&auto_menu, "Interval", mtm);

        let intervals: &[(u64, &str)] = &[
            (0, "Off"),
            (1, "Every 1h"),
            (6, "Every 6h"),
            (12, "Every 12h"),
            (24, "Every 24h"),
        ];
        for &(hours, label) in intervals {
            let text = if hours == state.config.auto_clean_hours {
                format!("{} ✓", label)
            } else {
                label.to_string()
            };
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(&text),
                    Some(sel!(handleSetAutoInterval:)),
                    ns_string!(""),
                )
            };
            item.setTag(hours as isize);
            unsafe { item.setTarget(Some(target)) };
            auto_menu.addItem(&item);
        }

        auto_menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_disabled(&auto_menu, "Clean older than", mtm);

        let ages: &[(u64, &str)] = &[
            (3, "3 days"),
            (7, "7 days"),
            (14, "14 days"),
            (30, "30 days"),
        ];
        for &(days, label) in ages {
            let text = if days == state.config.max_age_days {
                format!("{} ✓", label)
            } else {
                label.to_string()
            };
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &NSString::from_str(&text),
                    Some(sel!(handleSetMaxAge:)),
                    ns_string!(""),
                )
            };
            item.setTag(days as isize);
            unsafe { item.setTarget(Some(target)) };
            auto_menu.addItem(&item);
        }

        let auto_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                ns_string!("Auto Clean"),
                None,
                ns_string!(""),
            )
        };
        auto_item.setSubmenu(Some(&auto_menu));
        menu.addItem(&auto_item);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_action(&menu, "Quit", sel!(quit:), target, mtm);
    });

    state.status_item.setMenu(Some(&menu));
}

fn add_disabled(menu: &NSMenu, text: &str, mtm: MainThreadMarker) {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm), &NSString::from_str(text), None, ns_string!(""),
        )
    };
    item.setEnabled(false);
    menu.addItem(&item);
}

fn add_action(menu: &NSMenu, text: &str, action: Sel, target: &AnyObject, mtm: MainThreadMarker) {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm), &NSString::from_str(text), Some(action), ns_string!(""),
        )
    };
    unsafe { item.setTarget(Some(target)) };
    menu.addItem(&item);
}

fn project_name(td: &TargetDir) -> String {
    let name = match td.kind {
        ArtifactKind::CcTarget => {
            let dir_name = td.path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            return dir_name.strip_prefix("cc-target-").unwrap_or(dir_name).to_string();
        }
        _ => td.path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("unknown"),
    };
    name.to_string()
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
