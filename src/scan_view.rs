// Scan-results popover body: grouped selectable targets and clean CTA.
// Exports: `build` → (root view, height).
// Deps: crate::{header, menu_rows, names, selection, state, theme, widgets}.

use crate::header;
use crate::hover_row::hover_row;
use crate::menu_rows::plan_groups;
use crate::names::{age_short, display_names};
use crate::selection::is_recent;
use crate::state::AppState;
use crate::theme::Theme;
use crate::controls::{checkbox, clean_button, set_cmd_key, text_button_hint, text_button_underlined};
use crate::widgets::{
    self, add_line, add_size_wash, label, label_right, label_tracked, label_wrap, CONTENT_WIDTH, PAD_X, POPOVER_WIDTH,
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
pub const TAG_GROUP_BASE: isize = 3000;

const ROW_H: f64 = 40.0;
const GROUP_H: f64 = 33.0;
/// Match the mock: show the largest few, offer a link for the rest.
const VISIBLE_CAP: usize = 6;
const FOOTER_H: f64 = 128.0;
const MAX_HEIGHT: f64 = 540.0;

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let sizing = state.total_size() == 0 && !state.targets.is_empty();
    let paths: Vec<_> = state.targets.iter().map(|t| t.path.clone()).collect();
    let overlap = sizes_may_overlap(&paths);
    let plans = plan_groups(&state.targets, if state.show_all { usize::MAX } else { VISIBLE_CAP });
    let list_h = measure_list(&plans);
    let body_h = (MAX_HEIGHT - header::SCAN_HEADER_HEIGHT - FOOTER_H).min(list_h);
    let height = header::SCAN_HEADER_HEIGHT + body_h + FOOTER_H;
    let root = widgets::root_view(height, theme.surface, mtm);

    // Header shows everything found; only the clean button counts the selection.
    header::draw_scan_header(&root, height, state.disk_stats(), state.total_size(), sizing, theme, mtm);

    let list = widgets::scroll_document_view(&root, 0.0, FOOTER_H, POPOVER_WIDTH, body_h, list_h, mtm);
    draw_list(&list, state, &plans, sizing, list_h, theme, target, mtm);
    draw_footer(&root, state, sizing, overlap, theme, target, mtm);
    (root, height)
}

#[allow(clippy::too_many_arguments)]
fn draw_list(
    root: &NSView,
    state: &AppState,
    plans: &[crate::menu_rows::GroupPlan<'_>],
    sizing: bool,
    mut y: f64,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    let list_bottom = 4.0;
    let names = display_names(&state.targets);
    let max_size = state.targets.iter().map(|t| t.size_bytes).max().unwrap_or(1).max(1);
    for plan in plans {
        if y <= list_bottom + ROW_H {
            break;
        }
        y = draw_group_header(root, y, plan.group, plan.count, plan.size, sizing, theme, mtm);
        for row in &plan.rows {
            if y <= list_bottom + ROW_H {
                break;
            }
            y = draw_row(
                root, y, row.index, &names[row.index], row.target, max_size, sizing,
                state.selected.contains(&row.index),
                state.config.max_age_days, theme, target, mtm,
            );
        }
        y = draw_show_more(root, state, plan, y, list_bottom, theme, target, mtm);
    }
    if plans.is_empty() && !sizing {
        label(
            root, "Nothing to clean \u{2014} everything is tidy", PAD_X, list_bottom + 20.0,
            CONTENT_WIDTH, 20.0, 13.0, false, theme.ink_2, false, mtm,
        );
    }
}

fn draw_show_more(
    root: &NSView,
    state: &AppState,
    plan: &crate::menu_rows::GroupPlan<'_>,
    mut y: f64,
    list_bottom: f64,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    if plan.hidden > 0 && y > list_bottom + 40.0 {
        let hidden_size = sum_bytes(
            state.targets.iter().enumerate()
                .filter(|(i, td)| td.kind.group() == plan.group && !plan.rows.iter().any(|r| r.index == *i))
                .map(|(_, td)| td.size_bytes),
        );
        let title = format!(
            "Show {} smaller targets \u{00b7} {}",
            plan.hidden,
            human_size(hidden_size)
        );
        text_button_underlined(
            root, &title, PAD_X, y - 28.0, CONTENT_WIDTH, sel!(handleShowMore:), target,
            TAG_SHOW_MORE, theme.ink_2, mtm,
        );
        y -= 40.0;
    }
    if y > list_bottom + 8.0 {
        add_line(root, PAD_X, y - 4.0, CONTENT_WIDTH, theme.line, mtm);
        y -= 8.0;
    }
    y
}

fn measure_list(plans: &[crate::menu_rows::GroupPlan<'_>]) -> f64 {
    if plans.is_empty() {
        return 48.0;
    }
    let mut h = 8.0;
    for plan in plans {
        h += GROUP_H + plan.rows.len() as f64 * ROW_H;
        if plan.hidden > 0 {
            h += 40.0;
        }
        h += 8.0;
    }
    h
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
    let y = y_top - GROUP_H;
    widgets::symbol_view(parent, group.symbol(), PAD_X, y + 6.0, 15.0, theme.ink_3, mtm);
    let title = match group {
        ArtifactGroup::Rust => "RUST TARGETS",
        ArtifactGroup::NodeModules => "NODE MODULES",
        ArtifactGroup::BuildOutput => "BUILD OUTPUT",
        ArtifactGroup::Caches => "CACHES",
    };
    label_tracked(parent, title, PAD_X + 23.0, y + 7.0, 120.0, 17.0, 11.5, false, theme.ink_3, true, 0.69, mtm);
    widgets::add_fill(parent, PAD_X + 143.0, y + 5.0, 28.0, 17.0, theme.surface_2, 1.0, 4.0, mtm);
    label(parent, &count.to_string(), PAD_X + 148.0, y + 6.0, 18.0, 15.0, 10.5, false, theme.ink_3, true, mtm);
    if !sizing {
        label_right(parent, &header::scan_size(size), PAD_X + 180.0, y + 7.0, CONTENT_WIDTH - 180.0, 17.0, 12.0, theme.ink_2, true, mtm);
    }
    y_top - GROUP_H
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
    let row = hover_row(parent, y, ROW_H, theme.surface_2, mtm);
    let frac = td.size_bytes as f64 / max_size as f64;
    // Size bars for every row (mock), not only the checked ones.
    if !sizing {
        add_size_wash(&row, 0.0, 4.0, POPOVER_WIDTH, ROW_H - 8.0, frac, theme, mtm);
    }
    checkbox(&row, on, PAD_X, 12.0, sel!(handleToggleItem:), target, TAG_ITEM_BASE + index as isize, theme, mtm);
    label(&row, name, PAD_X + 26.0, 18.0, 220.0, 16.0, 13.5, false, theme.ink, false, mtm);
    let meta = if td.kind.group() == ArtifactGroup::Caches {
        format!("{} \u{00b7} network downloads to rebuild", td.kind.label())
    } else {
        format!("{} \u{00b7} {}", td.kind.label(), age_short(td.last_modified))
    };
    label(&row, &meta, PAD_X + 26.0, 4.0, 150.0, 14.0, 11.0, false, theme.ink_3, true, mtm);
    if is_recent(td, max_age_days) {
        let badge = widgets::add_fill(&row, PAD_X + 182.0, 3.0, 50.0, 17.0, theme.surface, 1.0, 3.0, mtm);
        badge.setBorderWidth(1.0);
        badge.setBorderColor(&Theme::color_alpha(theme.warn, 0.35));
        label_tracked(&row, "RECENT", PAD_X + 186.0, 5.0, 42.0, 13.0, 10.0, false, theme.warn, true, 0.4, mtm);
    }
    let size_text = if sizing { "\u{2026}".into() } else { header::scan_size(td.size_bytes) };
    label_right(&row, &size_text, PAD_X + 250.0, 12.0, CONTENT_WIDTH - 250.0, 16.0, 13.0, theme.ink, true, mtm);
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
    add_line(parent, 0.0, FOOTER_H, POPOVER_WIDTH, theme.line, mtm);
    let (title, size) = clean_cta(state, sizing);
    clean_button(parent, &title, &size, PAD_X, 82.0, CONTENT_WIDTH, sel!(handleCleanSelected:), target, TAG_CLEAN, theme, mtm);
    label_wrap(parent, &footer_note(state, overlap), PAD_X, 40.0, CONTENT_WIDTH, 33.0, 11.5, theme.ink_3, mtm);
    let rescan = text_button_hint(parent, "Rescan", "\u{2318}R", PAD_X, 13.0, sel!(handleRescan:), target, TAG_RESCAN, theme.ink_2, theme.ink_4, mtm);
    set_cmd_key(&rescan, "r");
    let settings = text_button_hint(parent, "Settings", "\u{2318},", POPOVER_WIDTH - PAD_X - 96.0, 13.0, sel!(openSettings:), target, TAG_SETTINGS, theme.ink_2, theme.ink_4, mtm);
    set_cmd_key(&settings, ",");
}

fn clean_cta(state: &AppState, sizing: bool) -> (String, String) {
    let title = if sizing { "Measuring\u{2026}".into() } else { format!("Clean {} selected", state.selected.len()) };
    let size = if sizing { "\u{2026}".into() } else { header::scan_size(state.selected_size()) };
    (title, size)
}

fn footer_note(state: &AppState, overlap: bool) -> String {
    let recent = state
        .targets
        .iter()
        .filter(|td| is_recent(td, state.config.max_age_days))
        .count();
    let mut note = if recent > 0 {
        format!("{recent} recent build{} excluded. Everything here is regenerated by your next build; ", if recent == 1 { "" } else { "s" })
    } else {
        "No recent builds excluded. Everything here is regenerated by your next build; ".to_string()
    };
    if overlap { note.push_str("sizes overlap where builds share APFS clones."); } else { note.push_str("sizes are reported from this scan."); }
    note
}
