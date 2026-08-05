// Screenshot harness: render each popover screen to PNG via view cache.
// Activated when WD40_SCREENSHOT_DIR is set. Numbers come from the live scan.
// Deps: objc2 AppKit/Foundation, crate::{names, popover, state, tasks}.

use crate::names::display_names;
use crate::popover;
use crate::state::{
    with_state, CleanItem, CleanItemStatus, CleanProgress, UiScreen,
};
use crate::tasks;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep};
use objc2_foundation::{MainThreadMarker, NSDictionary, NSString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static STARTED: AtomicBool = AtomicBool::new(false);
static AWAIT_DONE: AtomicBool = AtomicBool::new(false);

pub fn enabled() -> bool {
    std::env::var_os("WD40_SCREENSHOT_DIR").is_some()
}

pub fn maybe_start(_mtm: MainThreadMarker) {
    if !enabled() || STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
        dispatch(sequence);
    });
}

/// Called from `tasks::on_clean_done` so the done frame is captured after a real clean.
pub fn on_clean_finished(mtm: MainThreadMarker) {
    if !AWAIT_DONE.swap(false, Ordering::Relaxed) {
        return;
    }
    let dir = PathBuf::from(std::env::var("WD40_SCREENSHOT_DIR").unwrap_or_default());
    let appearance = std::env::var("WD40_APPEARANCE").unwrap_or_else(|_| "light".into());
    popover::refresh(mtm);
    popover::ensure_open(mtm);
    save_popover(&dir.join(format!("done-{appearance}.png")), mtm);
    eprintln!("wd-40: screenshots written to {}", dir.display());
    objc2_app_kit::NSApplication::sharedApplication(mtm).terminate(None);
}

fn sequence() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let dir = PathBuf::from(std::env::var("WD40_SCREENSHOT_DIR").unwrap());
    let _ = std::fs::create_dir_all(&dir);
    let appearance = std::env::var("WD40_APPEARANCE").unwrap_or_else(|_| "light".into());

    show_screen(UiScreen::Scan, mtm);
    save_popover(&dir.join(format!("scan-{appearance}.png")), mtm);

    show_screen(UiScreen::Settings, mtm);
    save_popover(&dir.join(format!("settings-{appearance}.png")), mtm);

    stage_cleaning_frame();
    show_screen(UiScreen::Cleaning, mtm);
    save_popover(&dir.join(format!("cleaning-{appearance}.png")), mtm);

    prepare_smallest_selection();
    AWAIT_DONE.store(true, Ordering::Relaxed);
    tasks::spawn_clean_selected();
}

fn show_screen(screen: UiScreen, mtm: MainThreadMarker) {
    with_state(|state| state.screen = screen);
    popover::refresh(mtm);
    popover::ensure_open(mtm);
}

fn stage_cleaning_frame() {
    with_state(|state| {
        let names = display_names(&state.targets);
        let mut items: Vec<CleanItem> = state
            .targets
            .iter()
            .enumerate()
            .take(8)
            .map(|(index, td)| CleanItem {
                index,
                name: names[index].clone(),
                size_bytes: td.size_bytes,
                status: CleanItemStatus::Pending,
            })
            .collect();
        if items.is_empty() {
            return;
        }
        let active_at = (items.len() / 2).max(1) - 1;
        for item in items.iter_mut().take(active_at) {
            item.status = CleanItemStatus::Done;
        }
        items[active_at].status = CleanItemStatus::Active;
        let freed = items
            .iter()
            .filter(|i| i.status == CleanItemStatus::Done)
            .map(|i| i.size_bytes)
            .fold(0_u64, |a, b| a.saturating_add(b));
        let current = state
            .targets
            .get(active_at)
            .map(|td| td.path.display().to_string())
            .unwrap_or_default();
        let done_count = items.iter().filter(|i| i.status == CleanItemStatus::Done).count();
        let total_count = items.len();
        state.cleaning = Some(CleanProgress {
            items,
            freed_so_far: freed,
            current_path: current,
            done_count,
            total_count,
        });
    });
}

fn prepare_smallest_selection() {
    with_state(|state| {
        state.selected.clear();
        // Prefer the disposable screenshot fixture so we never delete real projects.
        let fixture = state.targets.iter().enumerate().find(|(_, td)| {
            td.path.file_name().and_then(|n| n.to_str()) == Some("cc-target-wd40-shot")
        });
        if let Some((i, _)) = fixture {
            state.selected.insert(i);
        } else if let Some((i, _)) =
            state.targets.iter().enumerate().filter(|(_, td)| td.size_bytes < 2_000_000).min_by_key(|(_, td)| td.size_bytes)
        {
            state.selected.insert(i);
        }
        state.cleaning = None;
    });
}

fn save_popover(path: &Path, mtm: MainThreadMarker) {
    let Some(view) = popover::content_view(mtm) else {
        eprintln!("wd-40: no popover view for {}", path.display());
        return;
    };
    let bounds = view.bounds();
    let Some(rep) = view.bitmapImageRepForCachingDisplayInRect(bounds) else {
        eprintln!("wd-40: bitmap alloc failed for {}", path.display());
        return;
    };
    view.cacheDisplayInRect_toBitmapImageRep(bounds, &rep);
    write_png(&rep, path);
}

fn write_png(rep: &NSBitmapImageRep, path: &Path) {
    let props = NSDictionary::new();
    let Some(data) =
        (unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props) })
    else {
        eprintln!("wd-40: png encode failed for {}", path.display());
        return;
    };
    if !data.writeToFile_atomically(&NSString::from_str(&path.to_string_lossy()), true) {
        eprintln!("wd-40: write failed for {}", path.display());
    }
}

fn dispatch(work: fn()) {
    use std::ffi::c_void;
    static NEXT: std::sync::Mutex<Option<fn()>> = std::sync::Mutex::new(None);
    *NEXT.lock().unwrap() = Some(work);
    extern "C" fn run(_ctx: *mut c_void) {
        if let Some(work) = NEXT.lock().unwrap().take() {
            work();
        }
    }
    unsafe {
        extern "C" {
            static _dispatch_main_q: c_void;
            fn dispatch_async_f(
                queue: *const c_void,
                context: *mut c_void,
                work: extern "C" fn(*mut c_void),
            );
        }
        dispatch_async_f(std::ptr::addr_of!(_dispatch_main_q), std::ptr::null_mut(), run);
    }
}
