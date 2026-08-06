// Scan-results popover body: grouped selectable targets and the clean CTA.
// Exports: `build`, `draw_footer`, `header_model`, the screen's tags.
// Deps: crate::{controls, header, live, menu_rows, scan_rows, state, widgets}.

use crate::controls::{clean_button, set_cmd_key, text_button_hint, text_button_underlined, CleanCta};
use crate::header::{self, scan_size, ScanHeader};
use crate::live::{self, Group, Row, Scan, Zone};
use crate::menu_rows::{plan_groups, GroupPlan};
use crate::names::display_names;
use crate::scan_rows::{self, GroupModel, RowModel, GROUP_H, ROW_H};
use crate::selection::group_selection;
use crate::state::AppState;
use crate::theme::Theme;
use crate::widgets::{
    self, add_line, label, CONTENT_WIDTH, PAD_X, POPOVER_WIDTH,
};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use wd40::disk::sum_bytes;
use wd40::scanner::sizes_may_overlap;

pub const TAG_CLEAN: isize = 2000;
pub const TAG_RESCAN: isize = 2001;
pub const TAG_SETTINGS: isize = 2002;
pub const TAG_SHOW_MORE: isize = 2006;
pub const TAG_ITEM_BASE: isize = 1000;
pub const TAG_GROUP_BASE: isize = 3000;

/// Match the mock: show the largest few, offer a link for the rest.
const VISIBLE_CAP: usize = 6;
/// Button, shortcuts and padding. The caveat line adds its own height only
/// when there is a caveat.
const FOOTER_BASE_H: f64 = 95.0;
const CAVEAT_H: f64 = 20.0;

fn footer_h(state: &AppState) -> f64 {
    FOOTER_BASE_H + if footer_caveat(state).is_some() { CAVEAT_H } else { 0.0 }
}
const MAX_HEIGHT: f64 = 540.0;

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let plans = plan_groups(&state.targets, if state.show_all { usize::MAX } else { VISIBLE_CAP });
    let list_h = measure_list(&plans);
    let body_h = (MAX_HEIGHT - header::SCAN_HEADER_HEIGHT - footer_h(state)).min(list_h);
    let height = header::SCAN_HEADER_HEIGHT + body_h + footer_h(state);
    let root = widgets::root_view(height, theme.surface, mtm);

    // Header shows everything found; only the clean button counts the selection.
    let header_zone = Zone::new(&root, height - header::SCAN_HEADER_HEIGHT, header::SCAN_HEADER_HEIGHT, mtm);
    header::draw_scan_header(header_zone.view(), header_zone.top(), &header_model(state), theme, mtm);

    let list = crate::scrolling::scroll_document_view(&root, 0.0, footer_h(state), POPOVER_WIDTH, body_h, list_h, mtm);
    let (rows, groups) = draw_list(&list, state, &plans, list_h, theme, target, mtm);

    let footer_zone = Zone::new(&root, 0.0, footer_h(state) + 1.0, mtm);
    draw_footer(footer_zone.view(), state, theme, target, mtm);

    live::install_scan(Scan { theme: *theme, header: header_zone, footer: footer_zone, rows, groups });
    (root, height)
}

pub fn header_model(state: &AppState) -> ScanHeader {
    ScanHeader {
        disk: state.disk_stats(),
        reclaimable: state.total_size(),
        measured: state.measured.len(),
        total: state.targets.len(),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_list(
    root: &NSView,
    state: &AppState,
    plans: &[GroupPlan<'_>],
    mut y: f64,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> (Vec<Row>, Vec<Group>) {
    let list_bottom = 4.0;
    let names = display_names(&state.targets);
    let max_size = state.measured.iter().filter_map(|i| state.targets.get(*i)).map(|t| t.size_bytes).max().unwrap_or(1).max(1);
    let (mut rows, mut groups) = (Vec::new(), Vec::new());
    for plan in plans {
        if y <= list_bottom + ROW_H {
            break;
        }
        let model = GroupModel {
            group: plan.group,
            count: plan.count,
            size: plan.size,
            settled: state.group_settled(plan.group),
            selection: group_selection(&state.targets, &state.selected, plan.group),
        };
        let (next, group) = scan_rows::draw_group_header(root, y, &model, theme, target, mtm);
        y = next;
        groups.push(group);
        for row in &plan.rows {
            if y <= list_bottom + ROW_H {
                break;
            }
            let model = row_model(state, row, &names, max_size);
            let (next, drawn) = scan_rows::draw_row(root, y, &model, theme, target, mtm);
            y = next;
            rows.push(drawn);
        }
        y = draw_show_more(root, state, plan, y, list_bottom, theme, target, mtm);
    }
    if plans.is_empty() && !state.sizing() {
        label(
            root, "Nothing to clean \u{2014} everything is tidy", PAD_X, list_bottom + 20.0,
            CONTENT_WIDTH, 20.0, 13.0, false, theme.ink_2, false, mtm,
        );
    }
    (rows, groups)
}

fn row_model<'a>(
    state: &'a AppState,
    row: &'a crate::menu_rows::RowPlan<'a>,
    names: &'a [String],
    max_size: u64,
) -> RowModel<'a> {
    RowModel {
        index: row.index,
        name: &names[row.index],
        target: row.target,
        measured: state.measured.contains(&row.index),
        max_size,
        on: state.selected.contains(&row.index),
        max_age_days: state.config.max_age_days,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_show_more(
    root: &NSView,
    state: &AppState,
    plan: &GroupPlan<'_>,
    mut y: f64,
    list_bottom: f64,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    if plan.hidden > 0 && y > list_bottom + 40.0 {
        text_button_underlined(
            root, &show_more_title(state, plan), PAD_X, y - 28.0, CONTENT_WIDTH,
            sel!(handleShowMore:), target, TAG_SHOW_MORE, theme.ink_2, mtm,
        );
        y -= 40.0;
    }
    if y > list_bottom + 8.0 {
        add_line(root, PAD_X, y - 4.0, CONTENT_WIDTH, theme.line, mtm);
        y -= 8.0;
    }
    y
}

/// Until every size is in, the hidden rows are neither known to be the smaller
/// ones nor known to add up to anything, so the link claims neither.
fn show_more_title(state: &AppState, plan: &GroupPlan<'_>) -> String {
    if !state.group_settled(plan.group) {
        return format!("Show {} more", plan.hidden);
    }
    let hidden_size = sum_bytes(
        state.targets.iter().enumerate()
            .filter(|(i, td)| td.kind.group() == plan.group && !plan.rows.iter().any(|r| r.index == *i))
            .map(|(_, td)| td.size_bytes),
    );
    format!("Show {} smaller targets \u{00b7} {}", plan.hidden, scan_size(hidden_size))
}

fn measure_list(plans: &[GroupPlan<'_>]) -> f64 {
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

pub fn draw_footer(
    parent: &NSView,
    state: &AppState,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    add_line(parent, 0.0, footer_h(state), POPOVER_WIDTH, theme.line, mtm);
    clean_button(parent, &clean_cta(state), PAD_X, footer_h(state) - 47.0, CONTENT_WIDTH, sel!(handleCleanSelected:), target, TAG_CLEAN, theme, mtm);
    if let Some(caveat) = footer_caveat(state) {
        label(parent, &caveat, PAD_X, 46.0, CONTENT_WIDTH, 16.0, 11.5, false, theme.ink_3, false, mtm);
    }
    let rescan = text_button_hint(parent, "Rescan", "\u{2318}R", PAD_X, 13.0, sel!(handleRescan:), target, TAG_RESCAN, theme.ink_2, theme.ink_4, mtm);
    set_cmd_key(&rescan, "r");
    let settings = text_button_hint(parent, "Settings", "\u{2318},", POPOVER_WIDTH - PAD_X - 96.0, 13.0, sel!(openSettings:), target, TAG_SETTINGS, theme.ink_2, theme.ink_4, mtm);
    set_cmd_key(&settings, ",");
}

fn clean_cta(state: &AppState) -> CleanCta {
    if state.sizing() {
        return CleanCta::Measuring;
    }
    if state.selected.is_empty() {
        return CleanCta::Empty;
    }
    CleanCta::Ready {
        count: state.selected.len(),
        size: scan_size(state.selected_size()),
    }
}

/// The only thing the footer can say that the rows do not already show. Empty
/// checkboxes, RECENT badges and a disabled button state the rest of it
/// themselves; repeating them in prose was noise on every open.
fn footer_caveat(state: &AppState) -> Option<String> {
    let paths: Vec<_> = state.targets.iter().map(|t| t.path.clone()).collect();
    sizes_may_overlap(&paths)
        .then(|| "Sizes overlap where builds share APFS clones.".to_string())
}
