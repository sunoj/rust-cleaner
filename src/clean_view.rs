// Cleaning-progress popover body: disk header, job progress, and the rust
// plate that lifts a tile as each target is really removed.
// Exports: `build`, `TAG_STOP`, and the zone painters `live` calls each tick.
// Deps: crate::{controls, crust, header, live, plate, state, widgets}, wd40.

use crate::controls::filled_button;
use crate::crust::{crust_region, PLATE_H};
use crate::header::{self, scan_size, HeaderModel};
use crate::live::{self, Clean, Zone};
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
use std::cell::Cell;
use std::path::Path;
use std::time::Instant;
use wd40::disk::sum_bytes;

pub const TAG_STOP: isize = 2004;
const STRIP_H: f64 = 46.0;
const PLATE_BLOCK: f64 = PLATE_H + 46.0;
const PATH_H: f64 = 42.0;
const FOOTER_H: f64 = 78.0;

thread_local! {
    /// When this run started, kept only so throughput can be measured.
    static CLOCK: Cell<Option<(Instant, usize, usize)>> = const { Cell::new(None) };
}

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let progress = state.cleaning.clone().unwrap_or(CleanProgress {
        items: Vec::new(),
        freed_so_far: 0,
        current_path: String::new(),
        done_count: 0,
        total_count: 0,
    });
    let height = header::HEADER_HEIGHT + STRIP_H + PLATE_BLOCK + PATH_H + FOOTER_H;
    let root = widgets::root_view(height, theme.surface, mtm);

    let header_zone = Zone::new(&root, height - header::HEADER_HEIGHT, header::HEADER_HEIGHT, mtm);
    draw_disk_header(header_zone.view(), header_zone.top(), state, &progress, theme, mtm);

    let strip_top = height - header::HEADER_HEIGHT;
    let strip_zone = Zone::new(&root, strip_top - STRIP_H, STRIP_H, mtm);
    draw_strip(strip_zone.view(), strip_zone.top(), &progress, theme, mtm);

    let (plate_bottom, plate, crust) = draw_plate(&root, strip_top - STRIP_H, state, &progress, theme, mtm);
    let path = draw_path(&root, plate_bottom, &progress, theme, mtm);

    let footer_zone = Zone::new(&root, 0.0, FOOTER_H, mtm);
    draw_footer(footer_zone.view(), &progress, theme, target, mtm);

    live::install_clean(Clean {
        theme: *theme,
        header: header_zone,
        strip: strip_zone,
        footer: footer_zone,
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
            detail_left: Some(format!("{} freed so far", scan_size(p.freed_so_far))),
            detail_right: None,
        },
        theme,
        mtm,
    );
}

pub fn draw_strip(root: &NSView, y_top: f64, p: &CleanProgress, theme: &Theme, mtm: MainThreadMarker) {
    let elapsed = tick_clock(p);
    let total = sum_bytes(p.items.iter().map(|i| i.size_bytes));
    let done = if total == 0 { 0.0 } else { p.freed_so_far as f64 / total as f64 };
    add_fill(root, PAD_X, y_top - 22.0, CONTENT_WIDTH, 11.0, theme.ink_4, 1.0, 3.0, mtm);
    add_fill(root, PAD_X, y_top - 22.0, (CONTENT_WIDTH * done.clamp(0.0, 1.0)).max(0.5), 11.0, theme.pos, 0.6, 3.0, mtm);
    label(root, &status_line(p), PAD_X, y_top - 42.0, 230.0, 16.0, 12.5, false, theme.ink, false, mtm);
    if let Some(eta) = eta_label(p, elapsed) {
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

/// Bytes still to be attempted, which is what an ETA may extrapolate over.
fn queued_bytes(p: &CleanProgress) -> u64 {
    sum_bytes(
        p.items
            .iter()
            .filter(|i| matches!(i.status, CleanItemStatus::Pending | CleanItemStatus::Active))
            .map(|i| i.size_bytes),
    )
}

/// The only ETA this app is entitled to: bytes it has actually removed divided
/// by the seconds that took, applied to the bytes still queued. Until something
/// has been removed there is no rate to extrapolate, so there is no label.
fn eta_label(p: &CleanProgress, elapsed: f64) -> Option<String> {
    if p.done_count == 0 || p.freed_so_far == 0 || elapsed < 2.0 {
        return None;
    }
    let remaining = queued_bytes(p);
    if remaining == 0 {
        return None;
    }
    let seconds = remaining as f64 * elapsed / p.freed_so_far as f64;
    if !seconds.is_finite() || seconds > 3600.0 {
        return None;
    }
    Some(format!("~{} left", short_duration(seconds)))
}

fn short_duration(seconds: f64) -> String {
    if seconds < 90.0 {
        return format!("{}s", (seconds.ceil() as u64).max(1));
    }
    format!("{}m", (seconds / 60.0).ceil() as u64)
}

/// Seconds this run has been measurably working. A run is new when nothing has
/// come off yet, when the finished count drops, or when the queue length
/// changes; that is also when a rub is wiped.
fn tick_clock(p: &CleanProgress) -> f64 {
    let now = Instant::now();
    CLOCK.with(|cell| {
        let fresh = (p.done_count == 0 && p.freed_so_far == 0)
            || match cell.get() {
                Some((_, done, total)) => p.done_count < done || p.total_count != total,
                None => true,
            };
        if fresh {
            crate::spray::clear_wiped();
        }
        let start = if fresh { now } else { cell.get().map_or(now, |value| value.0) };
        cell.set(Some((start, p.done_count, p.total_count)));
        now.duration_since(start).as_secs_f64()
    })
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
) -> Retained<NSTextField> {
    label(root, "removing", PAD_X, y_top - 20.0, CONTENT_WIDTH, 14.0, 11.0, false, theme.ink_3, true, mtm);
    let path = label(root, "", PAD_X, y_top - 38.0, CONTENT_WIDTH, 16.0, 12.0, false, theme.ink, true, mtm);
    show_path(&path, p);
    add_line(root, 0.0, y_top - PATH_H, POPOVER_WIDTH, theme.line, mtm);
    path
}

/// Several targets come off at once, so name the one most recently picked up
/// and say how many others are going with it rather than implying it is alone.
pub fn show_path(field: &NSTextField, p: &CleanProgress) {
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
    filled_button(
        root, &stop_title(p), PAD_X, 38.0, CONTENT_WIDTH, sel!(handleStopClean:),
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

#[cfg(test)]
mod tests {
    use super::short_duration;

    #[test]
    fn durations_read_in_seconds_then_minutes() {
        assert_eq!(short_duration(0.2), "1s");
        assert_eq!(short_duration(47.1), "48s");
        assert_eq!(short_duration(200.0), "4m");
    }
}
