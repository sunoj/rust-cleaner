// Cleaning-progress popover body: disk header, job progress, and the rust
// plate that lifts a tile as each target is really removed.
// Exports: `build`, `TAG_STOP`, and the zone painters `live` calls each tick.
// Deps: crate::{controls, crust, header, live, pace, plate, state, widgets}.

use crate::controls::filled_button;
use crate::crust::{crust_region, PLATE_H};
use crate::header::{self, scan_size, HeaderModel};
use crate::live::{self, Clean, Zone};
use crate::pace::{elapsed, eta_label};
use crate::plate::plate_view;
use crate::state::{AppState, CleanItemStatus, CleanProgress};
use crate::theme::Theme;
use crate::widgets::{
    self, add_fill, add_line, label, label_right, label_wrap, CONTENT_WIDTH, PAD_X, POPOVER_WIDTH,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2_app_kit::{NSTextField, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use std::path::Path;
use wd40::disk::sum_bytes;

pub const TAG_STOP: isize = 2004;
const STRIP_H: f64 = 46.0;
const PLATE_BLOCK: f64 = PLATE_H + 46.0;
const PATH_H: f64 = 42.0;
const FOOTER_H: f64 = 78.0;

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let progress = state.cleaning.clone().unwrap_or_default();
    let height = header::HEADER_HEIGHT + STRIP_H + PLATE_BLOCK + PATH_H + FOOTER_H;
    let root = widgets::root_view(height, theme.surface, mtm);

    let header_zone = Zone::new(&root, height - header::HEADER_HEIGHT, header::HEADER_HEIGHT, mtm);
    draw_disk_header(header_zone.view(), header_zone.top(), state, &progress, theme, mtm);

    let strip_top = height - header::HEADER_HEIGHT;
    let strip_zone = Zone::new(&root, strip_top - STRIP_H, STRIP_H, mtm);
    draw_strip(strip_zone.view(), strip_zone.top(), &progress, theme, mtm);

    let (plate_bottom, plate, crust) = draw_plate(&root, strip_top - STRIP_H, state, &progress, theme, mtm);
    let (caption, path) = draw_path(&root, plate_bottom, &progress, theme, mtm);

    let footer_zone = Zone::new(&root, 0.0, FOOTER_H, mtm);
    draw_footer(footer_zone.view(), &progress, theme, target, mtm);

    live::install_clean(Clean {
        theme: *theme,
        header: header_zone,
        strip: strip_zone,
        footer: footer_zone,
        caption,
        path,
        plate,
        crust,
    });
    (root, height)
}

pub fn draw_disk_header(
    root: &NSView,
    y_top: f64,
    state: &AppState,
    p: &CleanProgress,
    theme: &Theme,
    mtm: MainThreadMarker,
) {
    header::draw_header(
        root,
        y_top,
        &HeaderModel {
            disk: state.disk_stats(),
            // What this job has not taken off the disk — the freed part has
            // already moved into the free slice by the time we redraw.
            reclaimable: left_bytes(p),
            sizing: false,
            approximate: false,
            trailing: Some(format!("{} of {}", p.done_count, p.total_count)),
            detail_left: Some(match p.working() {
                true => format!("{} freed so far", scan_size(p.freed_so_far)),
                false => format!("{} freed", scan_size(p.freed_so_far)),
            }),
            detail_right: None,
        },
        theme,
        mtm,
    );
}

pub fn draw_strip(root: &NSView, y_top: f64, p: &CleanProgress, theme: &Theme, mtm: MainThreadMarker) {
    let so_far = elapsed(p);
    let total = sum_bytes(p.items.iter().map(|i| i.size_bytes));
    let done = if total == 0 { 0.0 } else { p.freed_so_far as f64 / total as f64 };
    add_fill(root, PAD_X, y_top - 22.0, CONTENT_WIDTH, 11.0, theme.ink_4, 1.0, 3.0, mtm);
    add_fill(root, PAD_X, y_top - 22.0, (CONTENT_WIDTH * done.clamp(0.0, 1.0)).max(0.5), 11.0, theme.pos, 0.6, 3.0, mtm);
    label(root, &status_line(p), PAD_X, y_top - 42.0, 230.0, 16.0, 12.5, false, theme.ink, false, mtm);
    if let Some(eta) = eta_label(p, so_far) {
        label_right(root, &eta, PAD_X + 230.0, y_top - 42.0, CONTENT_WIDTH - 230.0, 16.0, 11.5, theme.ink_3, true, mtm);
    }
}

fn status_line(p: &CleanProgress) -> String {
    if p.total_count == 0 {
        return "Nothing to remove".to_string();
    }
    let troubled = p.troubled_count();
    if troubled > 0 && p.items.iter().all(|item| item.status.settled()) {
        return format!("{troubled} of {} could not be removed", p.total_count);
    }
    let skipped = p.items.iter().filter(|i| i.status == CleanItemStatus::Skipped).count();
    if skipped > 0 && p.items.iter().all(|item| item.status.settled()) {
        return format!("Stopped \u{2014} {skipped} left in place");
    }
    match left_bytes(p) {
        0 if p.done_count == p.total_count => format!("All {} targets clear", p.total_count),
        left => format!("{} of crust left", scan_size(left)),
    }
}

/// Bytes this job has not taken off the disk: what is still queued, what is
/// running, and whatever a part-way removal left behind.
fn left_bytes(p: &CleanProgress) -> u64 {
    sum_bytes(
        p.items
            .iter()
            .map(|item| item.size_bytes.saturating_sub(item.freed_bytes)),
    )
}

fn draw_plate(
    root: &NSView,
    y_top: f64,
    state: &AppState,
    p: &CleanProgress,
    theme: &Theme,
    mtm: MainThreadMarker,
) -> (f64, Retained<crate::plate::PlateView>, NSRect) {
    let y = y_top - PLATE_BLOCK;
    let legend = label(root, "", PAD_X, y + 8.0, CONTENT_WIDTH, 16.0, 11.5, false, theme.ink_2, true, mtm);
    let frame = NSRect::new(NSPoint::new(PAD_X, y + 34.0), NSSize::new(CONTENT_WIDTH, PLATE_H));
    let job = sum_bytes(p.items.iter().map(|i| i.size_bytes));
    let (crust, share) = crust_region(job, state.disk_stats());
    let view = plate_view(frame, p, crust, legend, share, theme.dark, mtm);
    root.addSubview(&view);
    (y, view, crust)
}

fn draw_path(
    root: &NSView,
    y_top: f64,
    p: &CleanProgress,
    theme: &Theme,
    mtm: MainThreadMarker,
) -> (Retained<NSTextField>, Retained<NSTextField>) {
    let caption = label(root, "", PAD_X, y_top - 20.0, CONTENT_WIDTH, 14.0, 11.0, false, theme.ink_3, true, mtm);
    let path = label(root, "", PAD_X, y_top - 38.0, CONTENT_WIDTH, 16.0, 12.0, false, theme.ink, true, mtm);
    show_path(&caption, &path, p);
    add_line(root, 0.0, y_top - PATH_H, POPOVER_WIDTH, theme.line, mtm);
    (caption, path)
}

/// Several targets come off at once, so name the one most recently picked up
/// and say how many others are going with it rather than implying it is alone.
/// Once nothing is running the caption stops saying otherwise: the same path is
/// then the last target the run touched, not one being worked.
pub fn show_path(caption: &NSTextField, field: &NSTextField, p: &CleanProgress) {
    let heading = if p.working() { "removing" } else { "last target" };
    caption.setStringValue(&NSString::from_str(heading));
    let text = match p.current_path.is_empty() {
        true => "\u{2026}".to_string(),
        false => {
            let named = ellipsize(&crate::names::display_path(Path::new(&p.current_path)), 44);
            match p.active_count().saturating_sub(1) {
                0 => named,
                others => format!("{named}  +{others} more"),
            }
        }
    };
    field.setStringValue(&NSString::from_str(&text));
}

pub fn draw_footer(
    root: &NSView,
    p: &CleanProgress,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    // Once the last target has settled there is nothing left to stop, and a
    // Stop button would say there was. The screen is only still up because it
    // is being shown, so the button offers the way out of it.
    let (title, action) = match p.working() {
        true => (stop_title(p), sel!(handleStopClean:)),
        false => ("Show the result".to_string(), sel!(handleShowResult:)),
    };
    filled_button(
        root, &title, PAD_X, 38.0, CONTENT_WIDTH, action,
        target, TAG_STOP, theme.surface_2, theme.ink, mtm,
    );
    label_wrap(
        root,
        "Spray lifts residue off cleared steel; a tile goes only when its target is really gone. Nothing reaches the Trash.",
        PAD_X, 6.0, CONTENT_WIDTH, 30.0, 11.5, theme.ink_3, mtm,
    );
}

/// Several targets are removed at once, so the button has to name how many it
/// will let finish. Stopping starts nothing further; it never abandons a
/// removal half way, which would leave a target neither there nor gone.
fn stop_title(p: &CleanProgress) -> String {
    let active = p.active_count();
    if crate::tasks::stop_requested() {
        return match active {
            0 => "Stopping\u{2026}".to_string(),
            1 => "Stopping after this target".to_string(),
            n => format!("Stopping after these {n} targets"),
        };
    }
    match active {
        0 | 1 => "Stop after this target".to_string(),
        n => format!("Stop after these {n} targets"),
    }
}

fn ellipsize(path: &str, max_chars: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max_chars {
        return path.to_string();
    }
    let tail: String = chars[chars.len() - (max_chars - 1)..].iter().collect();
    format!("\u{2026}{tail}")
}
