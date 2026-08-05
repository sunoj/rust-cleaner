// Settings screen inside the 380pt popover (replaces the old NSWindow).
// Exports: `build`. Maps real config fields; skips unknowable design toggles.
// Deps: crate::{autostart, state, theme, updater, widgets}, wd40::config.

use crate::state::AppState;
use crate::theme::Theme;
use crate::controls::{days_slider, pill_switch, text_button};
use crate::widgets::{
    self, add_fill, add_line, label, label_right, CONTENT_WIDTH, PAD_X, POPOVER_WIDTH,
};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::sel;
use objc2_app_kit::NSView;
use objc2_foundation::MainThreadMarker;
use wd40::config::ARTIFACT_DIRS;

pub const TAG_QUIT: isize = 2005;
pub const TAG_LOGIN: isize = 2100;
pub const TAG_AUTO_UPDATE: isize = 2101;
pub const TAG_INTERVAL: isize = 2102;
pub const TAG_MAX_AGE: isize = 2103;
pub const TAG_DEPTH: isize = 2104;
pub const TAG_ARTIFACT_BASE: isize = 2200;
pub const TAG_CHECK_UPDATES: isize = 2105;

const INTERVALS: &[(u64, &str)] = &[
    (0, "Off"),
    (1, "1h"),
    (6, "6h"),
    (12, "12h"),
    (24, "1d"),
];

pub fn build(state: &AppState, theme: &Theme, target: &AnyObject, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let artifact_rows = ARTIFACT_DIRS.len();
    let height = 86.0 + 210.0 + 90.0 + artifact_rows as f64 * 36.0 + 80.0 + 48.0;
    let root = widgets::root_view(height, theme.surface, mtm);
    let mut y = height;
    y = draw_brand(&root, y, theme, mtm);
    y = draw_cleaning(&root, y, state, theme, target, mtm);
    y = draw_artifacts(&root, y, state, theme, target, mtm);
    y = draw_general(&root, y, state, theme, target, mtm);
    y = draw_roots(&root, y, state, theme, mtm);
    add_line(&root, 0.0, 36.0, POPOVER_WIDTH, theme.line, mtm);
    text_button(&root, "Back", PAD_X, 8.0, 50.0, sel!(handleDoneAck:), target, TAG_QUIT - 1, theme.ink_2, mtm);
    text_button(
        &root, "Quit", POPOVER_WIDTH - PAD_X - 50.0, 8.0, 50.0, sel!(quit:), target, TAG_QUIT,
        theme.ink_2, mtm,
    );
    let _ = y;
    (root, height)
}

fn draw_cleaning(
    root: &NSView,
    mut y: f64,
    state: &AppState,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    y = section(root, y, "cleaning", theme, mtm);
    y = cycle_row(
        root, y, "Auto clean", interval_label(state.config.auto_clean_hours),
        sel!(settingsCycleInterval:), TAG_INTERVAL, theme, target, mtm,
    );
    y = keep_days_slider(root, y, state.config.max_age_days, theme, target, mtm);
    let depth = format!("{} levels", state.config.max_depth);
    cycle_row(
        root, y, "Scan depth", &depth, sel!(settingsCycleDepth:), TAG_DEPTH, theme, target, mtm,
    )
}

fn draw_artifacts(
    root: &NSView,
    mut y: f64,
    state: &AppState,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    y = section(root, y, "artifact types", theme, mtm);
    for (index, name) in ARTIFACT_DIRS.iter().enumerate() {
        let on = state.config.artifact_types.iter().any(|v| v == name);
        y = switch_row(
            root, y, name, None, on, sel!(settingsToggleArtifact:),
            TAG_ARTIFACT_BASE + index as isize, theme, target, mtm,
        );
    }
    y
}

fn draw_general(
    root: &NSView,
    mut y: f64,
    state: &AppState,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    y = section(root, y, "general", theme, mtm);
    y = switch_row(
        root, y, "Launch at Login", None, crate::autostart::is_enabled(),
        sel!(settingsToggleLoginItem:), TAG_LOGIN, theme, target, mtm,
    );
    if let Some(updater) = state.updater.as_ref() {
        y = switch_row(
            root, y, "Automatic updates", None, updater.automatic_checks(),
            sel!(settingsToggleAutoUpdate:), TAG_AUTO_UPDATE, theme, target, mtm,
        );
        text_button(
            root, "Check for Updates\u{2026}", PAD_X, y - 28.0, 160.0,
            sel!(settingsCheckForUpdates:), target, TAG_CHECK_UPDATES, theme.ink_2, mtm,
        );
        y -= 36.0;
    }
    y
}

fn draw_roots(root: &NSView, mut y: f64, state: &AppState, theme: &Theme, mtm: MainThreadMarker) -> f64 {
    y = section(root, y, "roots", theme, mtm);
    for dir in &state.config.scan_dirs {
        let text = crate::names::display_path(dir);
        label(root, &text, PAD_X, y - 22.0, CONTENT_WIDTH, 16.0, 12.0, false, theme.ink, true, mtm);
        y -= 28.0;
    }
    label(
        root, "Edit scan roots in ~/.config/wd-40/config.toml", PAD_X, y - 18.0,
        CONTENT_WIDTH, 14.0, 11.5, false, theme.ink_3, false, mtm,
    );
    y - 28.0
}

fn draw_brand(parent: &NSView, y_top: f64, theme: &Theme, mtm: MainThreadMarker) -> f64 {
    add_fill(parent, PAD_X, y_top - 44.0, 30.0, 30.0, theme.ink, 1.0, 8.0, mtm);
    label(parent, "40", PAD_X + 6.0, y_top - 38.0, 20.0, 18.0, 11.0, true, theme.surface, true, mtm);
    label(parent, "WD-40", PAD_X + 42.0, y_top - 32.0, 120.0, 18.0, 14.5, true, theme.ink, false, mtm);
    let ver = format!("v{}", crate::updater::bundle_version());
    label(parent, &ver, PAD_X + 42.0, y_top - 48.0, 160.0, 14.0, 11.0, false, theme.ink_3, true, mtm);
    add_line(parent, 0.0, y_top - 62.0, POPOVER_WIDTH, theme.line, mtm);
    y_top - 70.0
}

fn section(parent: &NSView, y_top: f64, title: &str, theme: &Theme, mtm: MainThreadMarker) -> f64 {
    label(parent, title, PAD_X, y_top - 18.0, CONTENT_WIDTH, 14.0, 11.0, false, theme.ink_3, true, mtm);
    y_top - 28.0
}

fn keep_days_slider(
    parent: &NSView,
    y_top: f64,
    days: u64,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    let y = y_top - 72.0;
    label(parent, "Keep builds newer than", PAD_X, y + 50.0, 220.0, 16.0, 13.5, false, theme.ink, false, mtm);
    let age = if days == 1 {
        "1 day".into()
    } else {
        format!("{days} days")
    };
    label_right(parent, &age, PAD_X + 200.0, y + 50.0, CONTENT_WIDTH - 200.0, 16.0, 12.5, theme.ink, true, mtm);
    days_slider(
        parent, days, PAD_X, y + 22.0, CONTENT_WIDTH, sel!(settingsSetMaxAge:), target, TAG_MAX_AGE, mtm,
    );
    label(parent, "0d", PAD_X, y + 2.0, 40.0, 14.0, 10.5, false, theme.ink_4, true, mtm);
    label_right(parent, "30d", PAD_X + CONTENT_WIDTH - 40.0, y + 2.0, 40.0, 14.0, 10.5, theme.ink_4, true, mtm);
    y
}

#[allow(clippy::too_many_arguments)]
fn switch_row(
    parent: &NSView,
    y_top: f64,
    title: &str,
    hint: Option<&str>,
    on: bool,
    action: Sel,
    tag: isize,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    let h = if hint.is_some() { 44.0 } else { 36.0 };
    let y = y_top - h;
    label(parent, title, PAD_X, y + h - 22.0, 240.0, 16.0, 13.5, false, theme.ink, false, mtm);
    if let Some(hint) = hint {
        label(parent, hint, PAD_X, y + 4.0, 240.0, 14.0, 11.5, false, theme.ink_3, false, mtm);
    }
    pill_switch(
        parent, on, POPOVER_WIDTH - PAD_X - 38.0, y + (h - 22.0) / 2.0, action, tag, theme, target, mtm,
    );
    y
}

#[allow(clippy::too_many_arguments)]
fn cycle_row(
    parent: &NSView,
    y_top: f64,
    title: &str,
    value: &str,
    action: Sel,
    tag: isize,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> f64 {
    let y = y_top - 36.0;
    label(parent, title, PAD_X, y + 10.0, 200.0, 16.0, 13.5, false, theme.ink, false, mtm);
    text_button(
        parent, value, POPOVER_WIDTH - PAD_X - 70.0, y + 8.0, 70.0, action, target, tag,
        theme.ink_2, mtm,
    );
    y
}

fn interval_label(hours: u64) -> &'static str {
    INTERVALS.iter().find(|(h, _)| *h == hours).map(|(_, l)| *l).unwrap_or("custom")
}

pub fn next_interval(current: u64) -> u64 {
    let idx = INTERVALS.iter().position(|(h, _)| *h == current).unwrap_or(0);
    INTERVALS[(idx + 1) % INTERVALS.len()].0
}

pub fn next_depth(current: usize) -> usize {
    match current {
        0..=2 => 3,
        3 => 4,
        4 => 5,
        5 => 6,
        _ => 3,
    }
}
