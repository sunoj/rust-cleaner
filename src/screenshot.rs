// Screenshot harness: render each popover screen to PNG via view cache.
// Compiled only in debug builds (`cfg(debug_assertions)`); activated by WD40_SCREENSHOT_DIR.
// Deps: objc2 AppKit/Foundation, crate::{names, popover, state, tasks}.

use crate::names::display_names;
use crate::popover;
use crate::state::{
    with_state, CleanItem, CleanItemStatus, CleanProgress, UiScreen,
};
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep};
use objc2_foundation::{MainThreadMarker, NSDictionary, NSString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use wd40::scanner::ArtifactGroup;

static STARTED: AtomicBool = AtomicBool::new(false);

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

fn sequence() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let dir = PathBuf::from(std::env::var("WD40_SCREENSHOT_DIR").unwrap());
    let _ = std::fs::create_dir_all(&dir);
    let appearance = std::env::var("WD40_APPEARANCE").unwrap_or_else(|_| "light".into());

    stage_mixed_group();
    show_screen(UiScreen::Scan, mtm);
    save_popover(&dir.join(format!("scan-{appearance}.png")), mtm);

    show_screen(UiScreen::Settings, mtm);
    save_popover(&dir.join(format!("settings-{appearance}.png")), mtm);

    stage_cleaning_frame();
    show_screen(UiScreen::Cleaning, mtm);
    save_popover(&dir.join(format!("cleaning-{appearance}.png")), mtm);

    stage_done_frame();
    show_screen(UiScreen::Done, mtm);
    save_popover(&dir.join(format!("done-{appearance}.png")), mtm);

    eprintln!("wd-40: screenshots written to {}", dir.display());
    objc2_app_kit::NSApplication::sharedApplication(mtm).terminate(None);
}

/// Build the done summary from what a scan already found. Earlier this ran a
/// real clean to reach this screen, and when its fixture was absent it fell
/// back to deleting the smallest real target it could find — a screenshot tool
/// must not delete anything a user owns.
fn stage_done_frame() {
    with_state(|state| {
        let mut removed: Vec<crate::state::GroupSummary> = Vec::new();
        for &group in wd40::scanner::ArtifactGroup::ALL {
            let members: Vec<&wd40::scanner::TargetDir> =
                state.targets.iter().filter(|td| td.kind.group() == group).collect();
            if members.is_empty() {
                continue;
            }
            removed.push(crate::state::GroupSummary {
                group,
                count: members.len().min(3),
                bytes: members.iter().take(3).map(|td| td.size_bytes).sum(),
            });
        }
        let freed: u64 = removed.iter().map(|g| g.bytes).sum();
        let skipped_count = state.targets.len() - removed.iter().map(|g| g.count).sum::<usize>();
        let disk = state.disk_stats();
        state.done = Some(crate::state::DoneSummary {
            freed_bytes: freed,
            duration: Duration::from_secs(48),
            free_before: disk.map_or(0, |d| d.free_bytes),
            free_after: disk.map_or(0, |d| d.free_bytes.saturating_add(freed)),
            total_bytes: disk.map_or(0, |d| d.total_bytes),
            removed,
            skipped_count,
            skipped_bytes: state.targets.iter().map(|td| td.size_bytes).sum::<u64>().saturating_sub(freed),
        });
    });
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

fn stage_mixed_group() {
    with_state(|state| {
        for &group in ArtifactGroup::ALL {
            let indices: Vec<usize> = state.targets.iter().enumerate()
                .filter(|(_, target)| target.kind.group() == group)
                .map(|(index, _)| index)
                .collect();
            if indices.len() < 2 {
                continue;
            }
            for index in &indices {
                state.selected.remove(index);
            }
            state.selected.insert(indices[0]);
            break;
        }
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
