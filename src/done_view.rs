// Done-summary popover body after a clean finishes.
// Exports: `build`. Omits rebuild-time estimates the app cannot know.
// Deps: crate::{state, theme, widgets}, wd40.

use crate::state::{AppState, DoneSummary};
use crate::theme::Theme;
use crate::widgets::{
    self, add_fill, add_line, filled_button, label, label_right, text_button, CONTENT_WIDTH, PAD_X,
    POPOVER_WIDTH,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use wd40::scanner::{human_size, ArtifactGroup};

pub const TAG_DONE: isize = 2003;
const FOOTER_H: f64 = 70.0;

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let summary = state.done.as_ref();
    let rows = summary.map(|s| s.removed.len() + usize::from(s.skipped_count > 0)).unwrap_or(0);
    let height = 110.0 + rows as f64 * 40.0 + FOOTER_H;
    let root = widgets::root_view(height, theme.surface, mtm);

    if let Some(s) = summary {
        draw_summary(&root, height, s, theme, mtm);
    }

    add_line(&root, 0.0, FOOTER_H, POPOVER_WIDTH, theme.line, mtm);
    filled_button(
        &root, "Done", PAD_X, 28.0, CONTENT_WIDTH, sel!(handleDoneAck:), target, TAG_DONE,
        theme.ink, theme.surface, mtm,
    );
    text_button(
        &root, "Rescan", PAD_X, 4.0, 60.0, sel!(handleRescan:), target, crate::scan_view::TAG_RESCAN,
        theme.ink_2, mtm,
    );
    (root, height)
}

fn draw_summary(parent: &NSView, height: f64, s: &DoneSummary, theme: &Theme, mtm: MainThreadMarker) {
    let freed = human_size(s.freed_bytes);
    label(parent, &freed, PAD_X, height - 42.0, 160.0, 32.0, 30.0, true, theme.ink, false, mtm);
    label(parent, "freed", PAD_X + 170.0, height - 36.0, 60.0, 20.0, 15.0, false, theme.ink_2, false, mtm);
    let secs = s.duration.as_secs().max(1);
    label_right(
        parent, &format!("in {secs}s"), PAD_X + 220.0, height - 34.0, CONTENT_WIDTH - 220.0, 16.0,
        11.5, theme.ink_3, true, mtm,
    );

    // Simple after-clean gauge: used vs free (no artifact slice).
    if s.total_bytes > 0 {
        let free_frac = s.free_after as f64 / s.total_bytes as f64;
        let used_w = CONTENT_WIDTH * (1.0 - free_frac).clamp(0.0, 1.0);
        add_fill(parent, PAD_X, height - 58.0, CONTENT_WIDTH, 7.0, theme.surface_2, 1.0, mtm);
        add_fill(parent, PAD_X, height - 58.0, used_w, 7.0, theme.ink_4, 1.0, mtm);
    }

    label(
        parent, &format!("{} free", human_size(s.free_after)), PAD_X, height - 82.0, 160.0, 16.0,
        13.0, false, theme.ink, false, mtm,
    );
    label_right(
        parent, &format!("was {}", human_size(s.free_before)), PAD_X + 180.0, height - 82.0,
        CONTENT_WIDTH - 180.0, 16.0, 11.5, theme.ink_3, true, mtm,
    );
    add_line(parent, 0.0, height - 96.0, POPOVER_WIDTH, theme.line, mtm);

    let mut y = height - 110.0;
    for row in &s.removed {
        y = draw_stat_row(parent, y, &row.count.to_string(), &group_label(row.group), &human_size(row.bytes), theme, false, mtm);
    }
    if s.skipped_count > 0 {
        draw_stat_row(
            parent, y, &s.skipped_count.to_string(), "Skipped \u{2014} recent",
            &human_size(s.skipped_bytes), theme, true, mtm,
        );
    }
    // No "rebuilding costs N min" card — that figure is not knowable here.
}

fn group_label(group: ArtifactGroup) -> String {
    match group {
        ArtifactGroup::Rust => "Rust targets removed".into(),
        ArtifactGroup::NodeModules => "Node modules removed".into(),
        ArtifactGroup::BuildOutput => "Build output removed".into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_stat_row(
    parent: &NSView,
    y_top: f64,
    count: &str,
    title: &str,
    size: &str,
    theme: &Theme,
    warn: bool,
    mtm: MainThreadMarker,
) -> f64 {
    let y = y_top - 40.0;
    let bg = if warn { theme.accent_weak } else { theme.surface_2 };
    let fg = if warn { theme.warn } else { theme.ink_2 };
    add_fill(parent, PAD_X, y + 8.0, 22.0, 22.0, bg, 1.0, mtm);
    label(parent, count, PAD_X + 2.0, y + 11.0, 18.0, 16.0, 11.0, false, fg, true, mtm);
    label(parent, title, PAD_X + 32.0, y + 12.0, 220.0, 16.0, 13.5, false, theme.ink, false, mtm);
    label_right(
        parent, size, PAD_X + 240.0, y + 12.0, CONTENT_WIDTH - 240.0, 16.0, 13.0,
        if warn { theme.ink_3 } else { theme.ink }, true, mtm,
    );
    y
}
