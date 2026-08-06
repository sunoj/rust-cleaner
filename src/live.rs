// Handles on the few views a small state change actually touches, so a size
// arriving or a tile coming off the plate repaints those instead of rebuilding
// the popover. The old full rebuild also threw away the scroll position, the
// hover and the spray in flight, which is why those states jumped.
// Exports: `Zone`, `Scan`, `Clean`, `install_*`, `clear`, the `*_changed` hooks.
// Deps: crate view modules + state + theme; objc2 AppKit.

use crate::checkbox::CheckBox;
use crate::plate::PlateView;
use crate::state::{with_state_ret, AppState, UiScreen};
use crate::theme::Theme;
use crate::{MenuHandler, HANDLER};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::MainThreadOnly;
use objc2_app_kit::{NSBox, NSTextField, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use std::cell::RefCell;
use wd40::scanner::ArtifactGroup;

thread_local! {
    static SCAN: RefCell<Option<Scan>> = const { RefCell::new(None) };
    static CLEAN: RefCell<Option<Clean>> = const { RefCell::new(None) };
}

/// A slice of the popover that is cheap to draw whole — a dozen views, against
/// several hundred for the screen. Its frame is its own region, so the drawing
/// code inside it keeps the coordinates it already used.
pub struct Zone {
    view: Retained<NSView>,
    top: f64,
}

impl Zone {
    pub fn new(parent: &NSView, y: f64, height: f64, mtm: MainThreadMarker) -> Self {
        let width = crate::widgets::POPOVER_WIDTH;
        let view = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, y), NSSize::new(width, height)),
        );
        parent.addSubview(&view);
        Self { view, top: height }
    }

    /// The zone's own top edge, which is what its drawing function counts down from.
    pub fn top(&self) -> f64 {
        self.top
    }

    pub fn view(&self) -> &NSView {
        &self.view
    }

    /// Empty it ready to be drawn again.
    pub fn clear(&self) {
        for view in self.view.subviews().to_vec() {
            view.removeFromSuperview();
        }
    }
}

pub struct Row {
    pub index: usize,
    pub size: Retained<NSTextField>,
    pub check: CheckBox,
    /// Accent bar behind the row, drawn to the size's share of the largest row.
    pub wash: Retained<NSBox>,
}

pub struct Group {
    pub group: ArtifactGroup,
    pub size: Retained<NSTextField>,
    pub check: CheckBox,
}

pub struct Scan {
    pub theme: Theme,
    pub header: Zone,
    pub footer: Zone,
    pub rows: Vec<Row>,
    pub groups: Vec<Group>,
}

pub struct Clean {
    pub theme: Theme,
    pub header: Zone,
    pub strip: Zone,
    pub footer: Zone,
    /// The line above the path, which stops saying "removing" when nothing is.
    pub caption: Retained<NSTextField>,
    pub path: Retained<NSTextField>,
    pub plate: Retained<PlateView>,
    /// Region of the plate the crust may cover, fixed for the run.
    pub crust: NSRect,
}

pub fn install_scan(scan: Scan) {
    clear();
    SCAN.with(|cell| *cell.borrow_mut() = Some(scan));
}

pub fn install_clean(clean: Clean) {
    clear();
    CLEAN.with(|cell| *cell.borrow_mut() = Some(clean));
}

/// Drop every handle. Called before each rebuild — the views they point at are
/// about to leave the tree.
pub fn clear() {
    SCAN.with(|cell| *cell.borrow_mut() = None);
    CLEAN.with(|cell| *cell.borrow_mut() = None);
}

/// Sizes that have just settled. Nothing here changes the layout, so no row
/// moves while the user is reading it.
///
/// False means there is no live scan screen to patch and the caller should
/// rebuild instead.
pub fn sizes_arrived(arrived: &[usize], mtm: MainThreadMarker) -> bool {
    patch(UiScreen::Scan, |state, target| {
        SCAN.with(|cell| {
            let held = cell.borrow();
            let Some(scan) = held.as_ref() else { return false };
            let max_size = largest(state);
            for row in &scan.rows {
                let Some(bytes) = state.measured.contains(&row.index).then(|| {
                    state.targets.get(row.index).map_or(0, |t| t.size_bytes)
                }) else {
                    continue;
                };
                if arrived.contains(&row.index) {
                    crate::scan_rows::show_size(&row.size, Some(bytes));
                    crate::motion::arrive(&row.size);
                }
                // Every wash is drawn against the largest row, so one arrival
                // can rescale the rest; they glide rather than snap.
                crate::motion::glide(&row.wash, crate::scan_rows::wash_frame(bytes, max_size));
            }
            for group in &scan.groups {
                crate::scan_rows::show_group_size(
                    &group.size,
                    state.group_size(group.group),
                    state.group_settled(group.group),
                );
            }
            redraw_scan_frame(scan, state, target, mtm);
            true
        })
    })
}

/// A row or group was ticked: the boxes and the button, and nothing else.
pub fn selection_changed(mtm: MainThreadMarker) -> bool {
    patch(UiScreen::Scan, |state, target| {
        SCAN.with(|cell| {
            let held = cell.borrow();
            let Some(scan) = held.as_ref() else { return false };
            for row in &scan.rows {
                let on = state.selected.contains(&row.index);
                row.check.set(crate::selection::GroupSelection::from(on), &scan.theme);
            }
            let empty = state.empty_targets();
            for group in &scan.groups {
                let selection = crate::selection::group_selection(
                    &state.targets, &state.selected, group.group, &empty,
                );
                group.check.set(selection, &scan.theme);
            }
            scan.footer.clear();
            crate::scan_view::draw_footer(scan.footer.view(), state, &scan.theme, target, mtm);
            true
        })
    })
}

/// One tick of a clean: the strip, the header, the path and the plate's tiles.
pub fn progress_changed(mtm: MainThreadMarker) -> bool {
    patch(UiScreen::Cleaning, |state, target| {
        CLEAN.with(|cell| {
            let held = cell.borrow();
            let Some(live) = held.as_ref() else { return false };
            let Some(progress) = state.cleaning.clone() else { return false };
            live.header.clear();
            live.strip.clear();
            live.footer.clear();
            crate::clean_view::draw_disk_header(
                live.header.view(), live.header.top(), state, &progress, &live.theme, mtm,
            );
            crate::clean_view::draw_strip(
                live.strip.view(), live.strip.top(), &progress, &live.theme, mtm,
            );
            crate::clean_view::draw_footer(
                live.footer.view(), &progress, &live.theme, target, mtm,
            );
            crate::clean_view::show_path(&live.caption, &live.path, &progress);
            live.plate.update(&progress, live.crust);
            true
        })
    })
}

/// Largest settled size on screen — nothing is drawn to scale against a figure
/// that has not arrived.
fn largest(state: &AppState) -> u64 {
    state
        .measured
        .iter()
        .filter_map(|index| state.targets.get(*index))
        .map(|target| target.size_bytes)
        .max()
        .unwrap_or(1)
        .max(1)
}

fn redraw_scan_frame(scan: &Scan, state: &AppState, target: &AnyObject, mtm: MainThreadMarker) {
    scan.header.clear();
    crate::header::draw_scan_header(
        scan.header.view(),
        scan.header.top(),
        &crate::scan_view::header_model(state),
        &scan.theme,
        mtm,
    );
    scan.footer.clear();
    crate::scan_view::draw_footer(scan.footer.view(), state, &scan.theme, target, mtm);
}

/// Run `body` only while `screen` is the one on show, with the state and the
/// action target it needs.
fn patch(screen: UiScreen, body: impl FnOnce(&AppState, &AnyObject) -> bool) -> bool {
    let Some(handler) = HANDLER.with(|cell| cell.borrow().clone()) else { return false };
    let target: &AnyObject =
        unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
    with_state_ret(|state| match state.screen == screen {
        true => body(state, target),
        false => false,
    })
    .unwrap_or(false)
}
