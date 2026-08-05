// Scan-results popover body: grouped selectable targets and clean CTA.
// Exports: `build` → (root view, height).
// Deps: crate::{header, menu_rows, names, selection, state, theme, widgets}.

use crate::header::{self, HeaderModel};
use crate::menu_rows::plan_groups;
use crate::names::{age_short, display_names};
use crate::selection::is_recent;
use crate::state::AppState;
use crate::theme::Theme;
use crate::widgets::{
    self, add_line, add_size_wash, checkbox, filled_button, label, label_right, text_button,
    CONTENT_WIDTH, PAD_X, POPOVER_WIDTH,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use wd40::disk::sum_bytes;
use wd40::scanner::{human_size, sizes_may_overlap, ArtifactGroup, TargetDir};

pub const TAG_CLEAN: isize = 2000;
pub const TAG_RESCAN: isize = 2001;
pub const TAG_SETTINGS: isize = 2002;
pub const TAG_SHOW_MORE: isize = 2006;
pub const TAG_ITEM_BASE: isize = 1000;

const ROW_H: f64 = 40.0;
const VISIBLE_CAP: usize = 12;
const FOOTER_H: f64 = 100.0;

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let sizing = state.total_size() == 0 && !state.targets.is_empty();
    let paths: Vec<_> = state.targets.iter().map(|t| t.path.clone()).collect();
    let overlap = sizes_may_overlap(&paths);
    let plans = plan_groups(&state.targets, if state.show_all { usize::MAX } else { VISIBLE_CAP });
    let list_h = measure_list(&plans, state).min(360.0);
    let height = header::HEADER_HEIGHT + list_h + FOOTER_H;
    let root = widgets::root_view(height, theme.surface, mtm);

    header::draw_header(
        &root,
        height,
        &HeaderModel {
            disk: state.disk_stats(),
            reclaimable: if sizing { 0 } else { state.selected_size() },
            sizing,
            approximate: overlap,
            trailing: None,
            detail_left: None,
            detail_right: None,
        },
        theme,
        mtm,
    );

    let mut y = height - header::HEADER_HEIGHT;
    let list_bottom = FOOTER_H + 4.0;
    let names = display_names(&state.targets);
    let max_size = state.targets.iter().map(|t| t.size_bytes).max().unwrap_or(1).max(1);

    for plan in &plans {
        if y <= list_bottom + ROW_H {
            break;
        }
        y = draw_group_header(&root, y, plan.group, plan.count, plan.size, sizing, theme, mtm);
        for row in &plan.rows {
            if y <= list_bottom + ROW_H {
                break;
            }
            y = draw_row(
                &root, y, row.index, &names[row.index], row.target, max_size, sizing,
                state.selected.contains(&row.index),
                state.config.max_age_days, theme, target, mtm,
            );
        }
        if plan.hidden > 0 && y > list_bottom + 28.0 {
            let hidden_size = sum_bytes(
                state.targets.iter().enumerate()
                    .filter(|(i, td)| td.kind.group() == plan.group && !plan.rows.iter().any(|r| r.index == *i))
                    .map(|(_, td)| td.size_bytes),
            );
            let title = format!("Show {} smaller \u{00b7} {}", plan.hidden, human_size(hidden_size));
            text_button(
                &root, &title, PAD_X, y - 22.0, CONTENT_WIDTH, sel!(handleShowMore:), target,
                TAG_SHOW_MORE, theme.ink_2, mtm,
            );
            y -= 28.0;
        }
        if y > list_bottom + 8.0 {
            add_line(&root, PAD_X, y - 4.0, CONTENT_WIDTH, theme.line, mtm);
            y -= 8.0;
        }
    }

    if plans.is_empty() && !sizing {
        label(
            &root, "Nothing to clean \u{2014} everything is tidy", PAD_X, list_bottom + 20.0,
            CONTENT_WIDTH, 20.0, 13.0, false, theme.ink_2, false, mtm,
        );
    }

    draw_footer(&root, state, sizing, overlap, theme, target, mtm);
    let _ = y;
    (root, height)
}

fn measure_list(plans: &[crate::menu_rows::GroupPlan<'_>], state: &AppState) -> f64 {
    if plans.is_empty() {
        return 48.0;
    }
    let mut h = 8.0;
    for plan in plans {
        h += 28.0 + plan.rows.len() as f64 * ROW_H;
        if plan.hidden > 0 {
            h += 28.0;
        }
        h += 8.0;
    }
    let _ = state;
    h.min(400.0)
}

#[allow(clippy::too_many_arguments)]
fn draw_group_header(
    parent: &NSView,
    y_top: f64,
    group: ArtifactGroup,
    count: usize,
    size: u64,
    sizing: bool,
    theme: &Theme,
    mtm: MainThreadMarker,
) -> f64 {
    let y = y_top - 24.0;
    let title = match group {
        ArtifactGroup::Rust => "rust targets",
        ArtifactGroup::NodeModules => "node modules",
        ArtifactGroup::BuildOutput => "build output",
    };
    label(parent, title, PAD_X, y, 160.0, 14.0, 11.5, false, theme.ink_3, true, mtm);
    label(parent, &count.to_string(), PAD_X + 130.0, y, 30.0, 14.0, 10.5, false, theme.ink_3, true, mtm);
    if !sizing {
        label_right(
            parent, &human_size(size), PAD_X + 180.0, y, CONTENT_WIDTH - 180.0, 14.0, 12.0,
            theme.ink_2, true, mtm,
        );
    }
    y_top - 28.0
}

#[allow(clippy::too_many_arguments)]
fn draw_row(
    parent: &NSView,
    y_top: f64,
    index: usize,
    name: &str,
    td: &TargetDir,
    max_size: u64,
    sizing: bool,
    on: bool,
    max_age_days: u64,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    let y = y_top - ROW_H;
    let frac = td.size_bytes as f64 / max_size as f64;
    if on && !sizing {
        add_size_wash(parent, 0.0, y + 4.0, POPOVER_WIDTH, ROW_H - 8.0, frac, theme, mtm);
    }
    checkbox(parent, on, PAD_X, y + 12.0, sel!(handleToggleItem:), target, TAG_ITEM_BASE + index as isize, theme, mtm);
    label(parent, name, PAD_X + 26.0, y + 18.0, 220.0, 16.0, 13.5, false, theme.ink, false, mtm);
    let meta = format!("{} \u{00b7} {}", td.kind.label(), age_short(td.last_modified));
    label(parent, &meta, PAD_X + 26.0, y + 4.0, 180.0, 14.0, 11.0, false, theme.ink_3, true, mtm);
    if is_recent(td, max_age_days) {
        label(
            parent, "recent", PAD_X + 210.0, y + 4.0, 50.0, 14.0, 10.0, false, theme.warn, true, mtm,
        );
    }
    let size_text = if sizing { "\u{2026}".into() } else { human_size(td.size_bytes) };
    label_right(
        parent, &size_text, PAD_X + 250.0, y + 12.0, CONTENT_WIDTH - 250.0, 16.0, 13.0,
        theme.ink, true, mtm,
    );
    y
}

fn draw_footer(
    parent: &NSView,
    state: &AppState,
    sizing: bool,
    overlap: bool,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    use crate::widgets::add_fill;
    add_fill(parent, 0.0, 0.0, POPOVER_WIDTH, FOOTER_H, theme.surface, 1.0, mtm);
    add_line(parent, 0.0, FOOTER_H, POPOVER_WIDTH, theme.line, mtm);
    let count = state.selected.len();
    let title = if sizing {
        "Measuring\u{2026}".into()
    } else {
        format!("Clean {count} selected  {}", human_size(state.selected_size()))
    };
    filled_button(
        parent, &title, PAD_X, 52.0, CONTENT_WIDTH, sel!(handleCleanSelected:), target, TAG_CLEAN,
        theme.ink, theme.surface, mtm,
    );
    let recent = state
        .targets
        .iter()
        .filter(|td| is_recent(td, state.config.max_age_days))
        .count();
    let mut note = if recent > 0 {
        format!("{recent} recent build{} excluded. ", if recent == 1 { "" } else { "s" })
    } else {
        String::new()
    };
    note.push_str("Removed targets rebuild on the next cargo build");
    if overlap {
        note.push_str("; sizes may overlap (APFS clones)");
    }
    note.push('.');
    label(parent, &note, PAD_X, 28.0, CONTENT_WIDTH, 20.0, 11.0, false, theme.ink_3, false, mtm);
    text_button(parent, "Rescan", PAD_X, 4.0, 60.0, sel!(handleRescan:), target, TAG_RESCAN, theme.ink_2, mtm);
    text_button(
        parent, "Settings", POPOVER_WIDTH - PAD_X - 70.0, 4.0, 70.0, sel!(openSettings:), target,
        TAG_SETTINGS, theme.ink_2, mtm,
    );
}
