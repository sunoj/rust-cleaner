// Cleaning-progress popover body: live list of pending/active/done removals.
// Exports: `build`. Omits ETA — the app cannot know remaining time.
// Deps: crate::{header, state, theme, widgets}, wd40.

use crate::header::{self, HeaderModel};
use crate::state::{AppState, CleanItemStatus, CleanProgress};
use crate::theme::Theme;
use crate::widgets::{
    self, add_line, filled_button, label, label_right, CONTENT_WIDTH, PAD_X, POPOVER_WIDTH,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use wd40::scanner::human_size;

pub const TAG_STOP: isize = 2004;
const ROW_H: f64 = 32.0;
const FOOTER_H: f64 = 78.0;
const PATH_H: f64 = 44.0;

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let progress = state.cleaning.as_ref();
    let list_h = progress.map(|p| (p.items.len() as f64 * ROW_H).min(360.0)).unwrap_or(40.0);
    let height = header::HEADER_HEIGHT + PATH_H + list_h + FOOTER_H;
    let root = widgets::root_view(height, theme.surface, mtm);

    let (freed, done, total, current) = match progress {
        Some(p) => (p.freed_so_far, p.done_count, p.total_count, p.current_path.clone()),
        None => (0, 0, 0, String::new()),
    };

    header::draw_header(
        &root,
        height,
        &HeaderModel {
            disk: state.disk_stats(),
            reclaimable: freed,
            sizing: false,
            approximate: false,
            trailing: Some(format!("{done} of {total}")),
            detail_left: Some(format!("{} freed so far", human_size(freed))),
            // No "~9s left" — we do not estimate remaining time.
            detail_right: None,
        },
        theme,
        mtm,
    );

    let mut y = height - header::HEADER_HEIGHT;
    add_line(&root, 0.0, y, POPOVER_WIDTH, theme.line, mtm);
    y -= 8.0;
    label(&root, "removing", PAD_X, y - 14.0, CONTENT_WIDTH, 14.0, 11.0, false, theme.ink_3, true, mtm);
    let path = if current.is_empty() { "\u{2026}".into() } else { ellipsize(&current, 48) };
    label(&root, &path, PAD_X, y - 32.0, CONTENT_WIDTH, 16.0, 12.0, false, theme.ink, true, mtm);
    y -= PATH_H;
    add_line(&root, 0.0, y, POPOVER_WIDTH, theme.line, mtm);

    if let Some(p) = progress {
        y = draw_items(&root, y, p, theme, mtm);
    }

    add_line(&root, 0.0, FOOTER_H, POPOVER_WIDTH, theme.line, mtm);
    filled_button(
        &root, "Stop after this target", PAD_X, 36.0, CONTENT_WIDTH, sel!(handleStopClean:),
        target, TAG_STOP, theme.surface_2, theme.ink, mtm,
    );
    label(
        &root,
        "Already-removed directories stay removed. Nothing goes to the Trash.",
        PAD_X, 10.0, CONTENT_WIDTH, 28.0, 11.5, false, theme.ink_3, false, mtm,
    );
    let _ = y;
    (root, height)
}

fn draw_items(
    parent: &NSView,
    mut y: f64,
    progress: &CleanProgress,
    theme: &Theme,
    mtm: MainThreadMarker,
) -> f64 {
    for item in &progress.items {
        y -= ROW_H;
        let (mark, color, _strike) = match item.status {
            CleanItemStatus::Done => ("\u{2713}", theme.ink_3, true),
            CleanItemStatus::Active => ("\u{25cb}", theme.ink, false),
            CleanItemStatus::Pending => ("\u{00b7}", theme.ink_4, false),
            CleanItemStatus::Skipped => ("\u{2013}", theme.ink_4, false),
        };
        label(parent, mark, PAD_X, y + 8.0, 16.0, 16.0, 13.0, false, color, false, mtm);
        label(parent, &item.name, PAD_X + 22.0, y + 8.0, 220.0, 16.0, 13.5, false, color, false, mtm);
        label_right(
            parent, &human_size(item.size_bytes), PAD_X + 240.0, y + 8.0, CONTENT_WIDTH - 240.0,
            16.0, 13.0, color, true, mtm,
        );
    }
    y
}

fn ellipsize(path: &str, max_chars: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max_chars {
        return path.to_string();
    }
    let tail: String = chars[chars.len() - (max_chars - 1)..].iter().collect();
    format!("\u{2026}{tail}")
}
