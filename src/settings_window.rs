// Dedicated Settings window for WD-40, laid out with explicit frames.
// Exports: `show`, `refresh`, `read_controls`, tag constants.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::{autostart, style}.

use crate::controls::{
    add_button, add_checkbox_at, add_custom_choice, add_label, add_popup, checkbox_is_on,
    label_field, select_tag, selected_tag, set_checkbox,
};
use crate::style::symbol_image;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSFont, NSImageView, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSPoint, NSRect, NSSize};
use std::cell::RefCell;
use wd40::config::{Config, ARTIFACT_DIRS};

/// Control tags, also used to find controls again when reading values back.
pub const TAG_LAUNCH_AT_LOGIN: isize = 100;
pub const TAG_AUTO_UPDATE: isize = 101;
pub const TAG_INTERVAL: isize = 102;
pub const TAG_MAX_AGE: isize = 103;
/// Artifact-type checkboxes occupy TAG_ARTIFACT_BASE + index.
pub const TAG_ARTIFACT_BASE: isize = 200;

const WIDTH: f64 = 440.0;
const HEIGHT: f64 = 486.0;
const MARGIN: f64 = 20.0;
const ROW: f64 = 28.0;

pub const INTERVALS: &[(u64, &str)] = &[
    (0, "Off"),
    (1, "Every hour"),
    (6, "Every 6 hours"),
    (12, "Every 12 hours"),
    (24, "Every day"),
];
pub const AGES: &[(u64, &str)] = &[(3, "3 days"), (7, "7 days"), (14, "14 days"), (30, "30 days")];

thread_local! {
    static WINDOW: RefCell<Option<Retained<NSWindow>>> = const { RefCell::new(None) };
}

/// Build the window on first use, then bring it to the front.
pub fn show(config: &Config, auto_update: Option<bool>, version: &str, target: &AnyObject, mtm: MainThreadMarker) {
    let existing = WINDOW.with(|cell| cell.borrow().clone());
    let window = match existing {
        Some(window) => {
            refresh(&window, config, auto_update);
            window
        }
        None => {
            let window = build(config, auto_update, version, target, mtm);
            WINDOW.with(|cell| *cell.borrow_mut() = Some(window.clone()));
            window
        }
    };
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    window.makeKeyAndOrderFront(None);
}

/// Push current config back into the controls (config can change from the menu).
pub fn refresh(window: &NSWindow, config: &Config, auto_update: Option<bool>) {
    let Some(root) = window.contentView() else { return };
    set_checkbox(&root, TAG_LAUNCH_AT_LOGIN, crate::autostart::is_enabled());
    if let Some(enabled) = auto_update {
        set_checkbox(&root, TAG_AUTO_UPDATE, enabled);
    }
    ensure_selectable(&root, TAG_INTERVAL, config.auto_clean_hours);
    ensure_selectable(&root, TAG_MAX_AGE, config.max_age_days);
    for (index, name) in ARTIFACT_DIRS.iter().enumerate() {
        let on = config.artifact_types.iter().any(|value| value == name);
        set_checkbox(&root, TAG_ARTIFACT_BASE + index as isize, on);
    }
}

/// What a single control edit means. Reading the whole window instead would
/// let one edit overwrite a field whose control cannot represent the stored
/// value (e.g. auto_clean_hours = 2 is not one of the five offered choices).
pub enum Change {
    Interval(u64),
    MaxAge(u64),
    Artifacts(Vec<String>),
}

/// Map the control that fired to the single field it owns.
pub fn read_change(sender_tag: isize) -> Option<Change> {
    let window = WINDOW.with(|cell| cell.borrow().clone())?;
    let root = window.contentView()?;
    match sender_tag {
        TAG_INTERVAL => Some(Change::Interval(selected_tag(&root, TAG_INTERVAL)? as u64)),
        TAG_MAX_AGE => Some(Change::MaxAge(selected_tag(&root, TAG_MAX_AGE)? as u64)),
        tag if is_artifact_tag(tag) => Some(Change::Artifacts(
            ARTIFACT_DIRS
                .iter()
                .enumerate()
                .filter(|(index, _)| checkbox_is_on(&root, TAG_ARTIFACT_BASE + *index as isize))
                .map(|(_, name)| (*name).to_string())
                .collect(),
        )),
        _ => None,
    }
}

/// Select `value`, adding a choice for it first when the popup lacks one.
fn ensure_selectable(root: &objc2_app_kit::NSView, tag: isize, value: u64) {
    if !select_tag(root, tag, value as isize) {
        add_custom_choice(root, tag, value);
        select_tag(root, tag, value as isize);
    }
}

pub fn is_artifact_tag(tag: isize) -> bool {
    (TAG_ARTIFACT_BASE..TAG_ARTIFACT_BASE + ARTIFACT_DIRS.len() as isize).contains(&tag)
}

/// Re-sync the visible controls with reality, used after a failed toggle.
pub fn resync(config: &Config, auto_update: Option<bool>) {
    if let Some(window) = WINDOW.with(|cell| cell.borrow().clone()) {
        refresh(&window, config, auto_update);
    }
}

fn build(
    config: &Config,
    auto_update: Option<bool>,
    version: &str,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> Retained<NSWindow> {
    let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Miniaturizable;
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, HEIGHT)),
            style,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(ns_string!("WD-40 Settings"));
    unsafe { window.setReleasedWhenClosed(false) };
    window.center();

    let root = {
        NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(WIDTH, HEIGHT)),
        )
    };

    let mut y = HEIGHT - MARGIN - 24.0;
    add_header(&root, "WD-40", &format!("v{version}"), y, mtm);
    y -= 40.0;

    y = add_section(&root, "General", y, mtm);
    add_checkbox(&root, "Launch at Login", TAG_LAUNCH_AT_LOGIN, crate::autostart::is_enabled(), sel!(settingsToggleLoginItem:), target, y, mtm);
    y -= ROW + 12.0;

    y = add_section(&root, "Cleaning", y, mtm);
    add_popup(&root, "Auto clean", TAG_INTERVAL, INTERVALS, config.auto_clean_hours, sel!(settingsChanged:), target, MARGIN, y, mtm);
    y -= ROW + 6.0;
    add_popup(&root, "Clean older than", TAG_MAX_AGE, AGES, config.max_age_days, sel!(settingsChanged:), target, MARGIN, y, mtm);
    y -= ROW + 12.0;

    y = add_section(&root, "Artifact Types to Scan", y, mtm);
    for (index, name) in ARTIFACT_DIRS.iter().enumerate() {
        let on = config.artifact_types.iter().any(|value| value == name);
        let column = (index % 2) as f64;
        let x = MARGIN + column * 200.0;
        let row_y = y - (index / 2) as f64 * ROW;
        add_checkbox_at(&root, name, TAG_ARTIFACT_BASE + index as isize, on, sel!(settingsChanged:), target, x, row_y, 180.0, mtm);
    }
    y -= ((ARTIFACT_DIRS.len() + 1) / 2) as f64 * ROW + 12.0;

    y = add_section(&root, "Updates", y, mtm);
    match auto_update {
        Some(enabled) => {
            add_checkbox(&root, "Check for updates automatically", TAG_AUTO_UPDATE, enabled, sel!(settingsToggleAutoUpdate:), target, y, mtm);
            y -= ROW + 4.0;
            add_button(&root, "Check Now", sel!(settingsCheckForUpdates:), target, MARGIN, y, mtm);
        }
        None => {
            add_label(&root, "Updates unavailable \u{2014} run the bundled app", MARGIN, y + 4.0, WIDTH - MARGIN * 2.0, true, mtm);
            y -= ROW + 4.0;
        }
    }
    y -= ROW + 10.0;

    add_label(&root, "Scan roots and depth live in ~/.config/wd-40/config.toml", MARGIN, y, WIDTH - MARGIN * 2.0, true, mtm);

    window.setContentView(Some(&root));
    window
}

fn add_header(root: &NSView, title: &str, version: &str, y: f64, mtm: MainThreadMarker) {
    if let Some(image) = symbol_image("wrench.and.screwdriver", 22.0) {
        let view = {
            NSImageView::initWithFrame(
                NSImageView::alloc(mtm),
                NSRect::new(NSPoint::new(MARGIN, y - 2.0), NSSize::new(24.0, 24.0)),
            )
        };
        view.setImage(Some(&image));
        root.addSubview(&view);
    }
    let label = label_field(title, MARGIN + 32.0, y, 200.0, false, mtm);
    label.setFont(Some(&NSFont::boldSystemFontOfSize(17.0)));
    root.addSubview(&label);
    let sub = label_field(version, WIDTH - MARGIN - 90.0, y + 3.0, 90.0, true, mtm);
    root.addSubview(&sub);
}

fn add_section(root: &NSView, title: &str, y: f64, mtm: MainThreadMarker) -> f64 {
    let label = label_field(title, MARGIN, y, WIDTH - MARGIN * 2.0, true, mtm);
    label.setFont(Some(&NSFont::boldSystemFontOfSize(11.0)));
    root.addSubview(&label);
    y - 24.0
}

/// Full-width row: wide enough that long titles are never truncated.
#[allow(clippy::too_many_arguments)]
fn add_checkbox(root: &NSView, title: &str, tag: isize, on: bool, action: Sel, target: &AnyObject, y: f64, mtm: MainThreadMarker) {
    add_checkbox_at(root, title, tag, on, action, target, MARGIN, y, WIDTH - MARGIN * 2.0, mtm);
}
