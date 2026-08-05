// Disk headers shared by popover screens (scan capacity and clean progress).
// Exports: `HEADER_HEIGHT`, `SCAN_HEADER_HEIGHT`, `draw_header`, `draw_scan_header`.
// Deps: crate::{disk_gauge, theme, widgets}, wd40.

use crate::disk_gauge::gauge_fractions;
use crate::theme::Theme;
use crate::widgets::{self, add_fill, add_line, label, label_right, label_tracked, CONTENT_WIDTH, PAD_X};
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use wd40::disk::DiskSpace;
use wd40::scanner::human_size;

pub const HEADER_HEIGHT: f64 = 78.0;
pub const SCAN_HEADER_HEIGHT: f64 = 125.0;

pub struct HeaderModel {
    pub disk: Option<DiskSpace>,
    /// Total found reclaimable (not the current selection).
    pub reclaimable: u64,
    pub sizing: bool,
    pub approximate: bool,
    /// Right-side caption; defaults to "of {total}" when None.
    pub trailing: Option<String>,
    /// Sub-label left text override (e.g. cleaning "X freed so far").
    pub detail_left: Option<String>,
    /// Sub-label right text; omit ETA-style guesses — pass None to hide.
    pub detail_right: Option<String>,
}

pub fn draw_header(parent: &NSView, y_top: f64, model: &HeaderModel, theme: &Theme, mtm: MainThreadMarker) {
    let y = y_top - HEADER_HEIGHT;
    add_line(parent, 0.0, y, widgets::POPOVER_WIDTH, theme.line, mtm);

    let Some(disk) = model.disk else {
        label(
            parent, "Disk unavailable", PAD_X, y_top - 28.0, CONTENT_WIDTH, 20.0, 14.0, false,
            theme.ink_2, false, mtm,
        );
        return;
    };

    let free = format!("{} free", human_size(disk.free_bytes));
    label(parent, &free, PAD_X, y_top - 30.0, 220.0, 22.0, 20.0, true, theme.ink, false, mtm);
    let trailing = model.trailing.clone().unwrap_or_else(|| format!("of {}", human_size(disk.total_bytes)));
    label_right(
        parent, &trailing, PAD_X + 200.0, y_top - 26.0, CONTENT_WIDTH - 200.0, 16.0, 11.5,
        theme.ink_3, true, mtm,
    );

    draw_gauge(parent, y_top - 42.0, disk, if model.sizing { 0 } else { model.reclaimable }, theme, mtm);

    let left = model.detail_left.clone().unwrap_or_else(|| {
        if model.sizing {
            "Measuring build artifacts\u{2026}".into()
        } else {
            let bound = if model.approximate { "up to " } else { "" };
            format!("{bound}{} reclaimable", human_size(model.reclaimable))
        }
    });
    add_fill(parent, PAD_X, y_top - 62.0, 7.0, 7.0, theme.accent, 1.0, 2.0, mtm);
    label(parent, &left, PAD_X + 14.0, y_top - 66.0, 200.0, 16.0, 13.0, false, theme.ink, false, mtm);
    if let Some(right) = &model.detail_right {
        label_right(
            parent, right, PAD_X + 180.0, y_top - 66.0, CONTENT_WIDTH - 180.0, 16.0, 11.5,
            theme.ink_3, true, mtm,
        );
    } else if !model.sizing && model.reclaimable > 0 && model.detail_left.is_none() {
        let after = disk.free_bytes.saturating_add(model.reclaimable).min(disk.total_bytes);
        let right = format!("\u{2192} {} free", human_size(after));
        label_right(
            parent, &right, PAD_X + 180.0, y_top - 66.0, CONTENT_WIDTH - 180.0, 16.0, 11.5,
            theme.ink_3, true, mtm,
        );
    }
}

pub fn draw_scan_header(
    parent: &NSView,
    y_top: f64,
    disk: Option<DiskSpace>,
    reclaimable: u64,
    sizing: bool,
    theme: &Theme,
    mtm: MainThreadMarker,
) {
    let y = y_top - SCAN_HEADER_HEIGHT;
    add_line(parent, 0.0, y, widgets::POPOVER_WIDTH, theme.line, mtm);
    let Some(disk) = disk else {
        label(parent, "Disk unavailable", PAD_X, y_top - 38.0, CONTENT_WIDTH, 24.0, 14.0, false, theme.ink_2, false, mtm);
        return;
    };

    let free = format!("{} free", scan_size(disk.free_bytes));
    label_tracked(parent, &free, PAD_X, y_top - 38.0, 220.0, 24.0, 20.0, true, theme.ink, false, -0.28, mtm);
    label_right(parent, &format!("of {}", scan_size(disk.total_bytes)), PAD_X + 200.0, y_top - 35.0, CONTENT_WIDTH - 200.0, 16.0, 11.5, theme.ink_3, true, mtm);

    draw_scan_gauge(parent, y_top - 59.0, disk, reclaimable, theme, mtm);
    let used = disk.total_bytes.saturating_sub(disk.free_bytes).min(disk.total_bytes);
    let reclaimable = reclaimable.min(used);
    draw_legend_row(
        parent,
        y_top - 86.0,
        [
            ("Reclaimable", wd40::scanner::human_size(reclaimable), theme.accent),
            ("In use", wd40::scanner::human_size(used.saturating_sub(reclaimable)), theme.ink_4),
            ("Free", wd40::scanner::human_size(disk.free_bytes), theme.pos),
        ],
        theme,
        mtm,
    );

    let after = if sizing {
        disk.free_bytes
    } else {
        disk.free_bytes.saturating_add(reclaimable).min(disk.total_bytes)
    };
    label(parent, &format!("\u{2192} {} free after cleaning", scan_size(after)), PAD_X, y_top - 112.0, CONTENT_WIDTH, 16.0, 11.5, false, theme.ink_3, true, mtm);
}

fn draw_scan_gauge(
    parent: &NSView,
    y: f64,
    disk: DiskSpace,
    reclaimable: u64,
    theme: &Theme,
    mtm: MainThreadMarker,
) {
    let parts = gauge_fractions(disk, reclaimable);
    let w = CONTENT_WIDTH;
    add_fill(parent, PAD_X, y, w, 11.0, theme.pos, 0.26, 3.0, mtm);
    let used_w = w * parts.used;
    let art_w = w * parts.artifacts;
    add_fill(parent, PAD_X, y, used_w, 11.0, theme.ink_4, 1.0, 0.0, mtm);
    add_fill(parent, PAD_X + used_w, y, art_w, 11.0, theme.accent, 1.0, 0.0, mtm);
    add_fill(parent, PAD_X + used_w + art_w, y, 1.0, 11.0, theme.pos, 0.55, 0.0, mtm);
}

/// Lay the three legend items out at their natural widths and spread the slack
/// between them, as the mock's space-between does. Fixed columns clipped
/// "Reclaimable" into the figure beside it.
fn draw_legend_row(
    parent: &NSView,
    y: f64,
    items: [(&str, String, (f64, f64, f64)); 3],
    theme: &Theme,
    mtm: MainThreadMarker,
) {
    const SWATCH: f64 = 8.0;
    const AFTER_SWATCH: f64 = 6.0;
    const BEFORE_VALUE: f64 = 6.0;
    let widths: Vec<(f64, f64)> = items
        .iter()
        .map(|(title, value, _)| {
            (
                widgets::fitted_width(title, 12.5, false, mtm),
                widgets::fitted_width(value, 12.0, true, mtm),
            )
        })
        .collect();
    let natural: f64 = widths
        .iter()
        .map(|(t, v)| SWATCH + AFTER_SWATCH + t + BEFORE_VALUE + v)
        .sum();
    let gap = ((CONTENT_WIDTH - natural) / 2.0).max(8.0);

    let mut x = PAD_X;
    for ((title, value, swatch), (title_w, value_w)) in items.iter().zip(widths) {
        add_fill(parent, x, y + 4.0, SWATCH, SWATCH, *swatch, 1.0, 2.0, mtm);
        let title_x = x + SWATCH + AFTER_SWATCH;
        label(parent, title, title_x, y, title_w + 2.0, 17.0, 12.5, false, theme.ink, false, mtm);
        let value_x = title_x + title_w + BEFORE_VALUE;
        label(parent, value, value_x, y, value_w + 2.0, 17.0, 12.0, false, theme.ink_2, true, mtm);
        x = value_x + value_w + gap;
    }
}

pub fn scan_size(bytes: u64) -> String {
    wd40::scanner::human_size(bytes)
        .replace('T', " TB")
        .replace('G', " GB")
        .replace('M', " MB")
        .replace('K', " KB")
}

fn draw_gauge(
    parent: &NSView,
    y: f64,
    disk: DiskSpace,
    reclaimable: u64,
    theme: &Theme,
    mtm: MainThreadMarker,
) {
    let parts = gauge_fractions(disk, reclaimable);
    let w = CONTENT_WIDTH;
    add_fill(parent, PAD_X, y, w, 7.0, theme.surface_2, 1.0, 3.5, mtm);
    let used_w = w * parts.used;
    let art_w = w * parts.artifacts;
    add_fill(parent, PAD_X, y, used_w, 7.0, theme.ink_4, 1.0, 3.5, mtm);
    add_fill(parent, PAD_X + used_w, y, art_w.max(0.0), 7.0, theme.accent, 1.0, 3.5, mtm);
}
