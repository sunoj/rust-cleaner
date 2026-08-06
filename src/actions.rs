// Settings and popover action helpers invoked from MenuHandler.
// Exports: free functions that mutate AppState / spawn work.
// Deps: crate::{autostart, popover, settings_view, state, tasks}, wd40::config.

use crate::popover;
use crate::settings_view;
use crate::state::{with_state, UiScreen};
use crate::tasks;
use objc2_app_kit::{NSAlert, NSAlertStyle, NSApplication, NSButton};
use objc2_foundation::{MainThreadMarker, NSString};
use wd40::config::{merge_artifact_types, ARTIFACT_DIRS};

pub fn toggle_item(tag: isize, mtm: MainThreadMarker) {
    let index = (tag - crate::scan_view::TAG_ITEM_BASE) as usize;
    // Ticking a row is the smallest state change the app has; it repaints the
    // box and the button, and leaves the list exactly where it was.
    with_state(|state| state.toggle_selected(index));
    if !crate::live::selection_changed(mtm) {
        popover::refresh(mtm);
    }
}

pub fn done_ack(mtm: MainThreadMarker) {
    with_state(|state| {
        state.screen = UiScreen::Scan;
        state.done = None;
    });
    popover::refresh(mtm);
    tasks::start_scan(false);
}

pub fn show_more(mtm: MainThreadMarker) {
    with_state(|state| state.show_all = true);
    popover::refresh(mtm);
}

pub fn open_settings(mtm: MainThreadMarker) {
    with_state(|state| state.screen = UiScreen::Settings);
    popover::refresh(mtm);
}

pub fn cycle_interval(mtm: MainThreadMarker) {
    let hours = with_state_ret_hours();
    if hours > 0 {
        tasks::start_auto_clean(hours);
    } else {
        tasks::stop_auto_clean();
    }
    popover::refresh(mtm);
}

fn with_state_ret_hours() -> u64 {
    let mut hours = 0;
    with_state(|state| {
        state.config.auto_clean_hours = settings_view::next_interval(state.config.auto_clean_hours);
        state.config.save();
        hours = state.config.auto_clean_hours;
    });
    hours
}

pub fn set_max_age(days: u64, mtm: MainThreadMarker) {
    with_state(|state| {
        state.config.max_age_days = days.min(30);
        state.config.save();
        state.reset_selection();
    });
    popover::refresh(mtm);
}

pub fn cycle_depth(mtm: MainThreadMarker) {
    with_state(|state| {
        state.config.max_depth = settings_view::next_depth(state.config.max_depth);
        state.config.save();
    });
    popover::refresh(mtm);
    tasks::start_scan(false);
}

pub fn toggle_artifact(sender: &NSButton, mtm: MainThreadMarker) {
    let index = (sender.tag() - settings_view::TAG_ARTIFACT_BASE) as usize;
    let Some(name) = ARTIFACT_DIRS.get(index).copied() else { return };
    with_state(|state| {
        let mut selected: Vec<String> = state
            .config
            .artifact_types
            .iter()
            .filter(|v| ARTIFACT_DIRS.iter().any(|k| k == &v.as_str()))
            .cloned()
            .collect();
        if let Some(pos) = selected.iter().position(|v| v == name) {
            selected.remove(pos);
        } else {
            selected.push(name.to_string());
        }
        state.config.artifact_types = merge_artifact_types(&state.config.artifact_types, &selected);
        state.config.save();
    });
    popover::refresh(mtm);
    tasks::start_scan(false);
}

pub fn toggle_login(mtm: MainThreadMarker) {
    let enable = !crate::autostart::is_enabled();
    if let Err(err) = crate::autostart::set_enabled(enable) {
        show_alert(mtm, "Launch at Login", &err);
    }
    popover::refresh(mtm);
}

pub fn toggle_auto_update(mtm: MainThreadMarker) {
    with_state(|state| {
        if let Some(updater) = state.updater.as_ref() {
            updater.set_automatic_checks(!updater.automatic_checks());
        }
    });
    popover::refresh(mtm);
}

pub fn check_updates() {
    with_state(|state| {
        if let Some(updater) = state.updater.as_ref() {
            updater.check_now();
        }
    });
}

pub fn show_alert(mtm: MainThreadMarker, title: &str, body: &str) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(body));
    alert.setAlertStyle(NSAlertStyle::Informational);
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    alert.runModal();
}
