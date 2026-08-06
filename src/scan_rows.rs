// The scan list's two kinds of line: a group header and a target row. Both hand
// back the handles that let a size arrive, or a tick land, without the list
// being rebuilt underneath.
// Exports: `ROW_H`, `GROUP_H`, `draw_group_header`, `draw_row`, `show_size`,
// `show_group_size`. Deps: crate::{checkbox, header, hover_row, live, widgets}.

use crate::checkbox::checkbox;
use crate::header::scan_size;
use crate::hover_row::hover_row;
use crate::live::{Group, Row};
use crate::names::age_short;
use crate::selection::{is_recent, GroupSelection};
use crate::theme::Theme;
use crate::widgets::{self, label, label_right, label_tracked, PAD_X, POPOVER_WIDTH};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2_app_kit::{NSBox, NSTextField, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use wd40::scanner::{ArtifactGroup, TargetDir};

pub const ROW_H: f64 = 40.0;
pub const GROUP_H: f64 = 33.0;
/// Shown in place of a figure that is not known yet.
const UNKNOWN: &str = "\u{2026}";
/// Marks a total that is still being added to.
const FLOOR: &str = "\u{2265} ";

pub struct GroupModel {
    pub group: ArtifactGroup,
    pub count: usize,
    pub size: u64,
    /// Every target in the group has a settled size.
    pub settled: bool,
    pub selection: GroupSelection,
}

pub struct RowModel<'a> {
    pub index: usize,
    pub name: &'a str,
    pub target: &'a TargetDir,
    pub measured: bool,
    /// Largest settled size on screen, which the wash is drawn against.
    pub max_size: u64,
    pub on: bool,
    pub max_age_days: u64,
}

/// A figure, or the mark that says there is not one yet.
pub fn show_size(field: &NSTextField, bytes: Option<u64>) {
    let text = bytes.map_or_else(|| UNKNOWN.to_string(), scan_size);
    field.setStringValue(&NSString::from_str(&text));
}

/// A group total. Until every member has been measured it is a floor, and it
/// is drawn as one rather than as an answer that happens to be low.
pub fn show_group_size(field: &NSTextField, bytes: u64, settled: bool) {
    let prefix = if settled { "" } else { FLOOR };
    field.setStringValue(&NSString::from_str(&format!("{prefix}{}", scan_size(bytes))));
}

/// How wide the size wash is for `bytes` against the largest row on screen.
pub fn wash_frame(bytes: u64, max_size: u64) -> NSRect {
    let fraction = bytes as f64 / max_size.max(1) as f64;
    let width = (POPOVER_WIDTH * fraction.clamp(0.0, 1.0)).max(0.5);
    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, ROW_H))
}

pub fn draw_group_header(
    parent: &NSView,
    y_top: f64,
    model: &GroupModel,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> (f64, Group) {
    let y = y_top - GROUP_H;
    // The mock draws no checkbox here, but selecting a whole group at once is a
    // requirement of its own; the header is the only place it can live.
    let index = ArtifactGroup::ALL.iter().position(|g| *g == model.group).unwrap_or(0);
    let check = checkbox(
        parent, model.selection, PAD_X, y + 6.0, sel!(handleToggleGroup:), target,
        crate::scan_view::TAG_GROUP_BASE + index as isize, theme, mtm,
    );
    widgets::symbol_view(parent, model.group.symbol(), PAD_X + 23.0, y + 6.0, 15.0, theme.ink_3, mtm);
    // Boxes centre on y + 13.5; all-caps text carries descender space it never
    // uses, so its frame sits a point lower to land on the same optical line.
    label_tracked(parent, title(model.group), PAD_X + 46.0, y + 4.0, 120.0, 17.0, 11.5, false, theme.ink_3, true, 0.69, mtm);
    widgets::add_fill(parent, PAD_X + 166.0, y + 5.0, 28.0, 17.0, theme.surface_2, 1.0, 4.0, mtm);
    label(parent, &model.count.to_string(), PAD_X + 171.0, y + 6.0, 18.0, 15.0, 10.5, false, theme.ink_3, true, mtm);
    let size = label_right(parent, "", PAD_X + 203.0, y + 4.0, widgets::CONTENT_WIDTH - 203.0, 17.0, 12.0, theme.ink_2, true, mtm);
    show_group_size(&size, model.size, model.settled);
    (y, Group { group: model.group, size, check })
}

fn title(group: ArtifactGroup) -> &'static str {
    match group {
        ArtifactGroup::Rust => "RUST TARGETS",
        ArtifactGroup::NodeModules => "NODE MODULES",
        ArtifactGroup::BuildOutput => "BUILD OUTPUT",
        ArtifactGroup::Caches => "CACHES",
    }
}

pub fn draw_row(
    parent: &NSView,
    y_top: f64,
    model: &RowModel<'_>,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> (f64, Row) {
    let y = y_top - ROW_H;
    let tag = crate::scan_view::TAG_ITEM_BASE + model.index as isize;
    let row = hover_row(parent, y, ROW_H, theme.surface_2, tag, mtm);
    // Full row height, not inset. The mock insets it because nothing else sits
    // behind the row; here the hover tint does, and a shorter wash on top of it
    // banded every row into three tones.
    let wash = draw_wash(&row, model, theme, mtm);
    let check = checkbox(&row, GroupSelection::from(model.on), PAD_X, 13.0, sel!(handleToggleItem:), target, tag, theme, mtm);
    label(&row, model.name, PAD_X + 26.0, 18.0, 220.0, 16.0, 13.5, false, theme.ink, false, mtm);
    label(&row, &meta(model.target), PAD_X + 26.0, 4.0, 150.0, 14.0, 11.0, false, theme.ink_3, true, mtm);
    if is_recent(model.target, model.max_age_days) {
        draw_recent_badge(&row, theme, mtm);
    }
    let size = label_right(&row, "", PAD_X + 250.0, 12.0, widgets::CONTENT_WIDTH - 250.0, 16.0, 13.0, theme.ink, true, mtm);
    show_size(&size, model.measured.then_some(model.target.size_bytes));
    (y, Row { index: model.index, size, check, wash })
}

fn draw_wash(
    row: &NSView,
    model: &RowModel<'_>,
    theme: &Theme,
    mtm: MainThreadMarker,
) -> Retained<NSBox> {
    let frame = match model.measured {
        true => wash_frame(model.target.size_bytes, model.max_size),
        false => NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.5, ROW_H)),
    };
    widgets::add_fill(row, 0.0, 0.0, frame.size.width, ROW_H, theme.accent, 0.09, 0.0, mtm)
}

fn meta(target: &TargetDir) -> String {
    if target.kind.group() == ArtifactGroup::Caches {
        return format!("{} \u{00b7} network downloads to rebuild", target.kind.label());
    }
    format!("{} \u{00b7} {}", target.kind.label(), age_short(target.last_modified))
}

fn draw_recent_badge(row: &NSView, theme: &Theme, mtm: MainThreadMarker) {
    let badge = widgets::add_fill(row, PAD_X + 182.0, 3.0, 50.0, 17.0, theme.surface, 1.0, 3.0, mtm);
    badge.setBorderWidth(1.0);
    badge.setBorderColor(&Theme::color_alpha(theme.warn, 0.35));
    label_tracked(row, "RECENT", PAD_X + 186.0, 5.0, 54.0, 13.0, 10.0, false, theme.warn, true, 0.4, mtm);
}
