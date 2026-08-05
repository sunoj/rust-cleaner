// Disk header shared by scan and cleaning screens (free / gauge / reclaimable).
// Exports: `header_height`, `draw_header`.
// Deps: crate::{disk_gauge, theme, widgets}, wd40.

use crate::disk_gauge::gauge_fractions;
use crate::theme::Theme;
use crate::widgets::{self, add_fill, add_line, label, label_right, CONTENT_WIDTH, PAD_X};
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use wd40::disk::DiskSpace;
use wd40::scanner::human_size;

pub const HEADER_HEIGHT: f64 = 78.0;

pub struct HeaderModel {
    pub disk: Option<DiskSpace>,
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
    // Accent swatch
    add_fill(parent, PAD_X, y_top - 62.0, 7.0, 7.0, theme.accent, 1.0, mtm);
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
    add_fill(parent, PAD_X, y, w, 7.0, theme.surface_2, 1.0, mtm);
    let used_w = w * parts.used;
    let art_w = w * parts.artifacts;
    add_fill(parent, PAD_X, y, used_w, 7.0, theme.ink_4, 1.0, mtm);
    add_fill(parent, PAD_X + used_w, y, art_w.max(0.0), 7.0, theme.accent, 1.0, mtm);
}
