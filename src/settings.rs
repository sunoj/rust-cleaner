// Settings submenu for WD-40: auto-clean cadence, age threshold, login item, updates.
// Exports: `build_settings_menu`.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::{autostart, menu}.

use crate::menu::{add_caption, new_item};
use crate::{autostart, AppState};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadOnly};
use objc2_app_kit::{NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSMenuItem};
use objc2_foundation::{ns_string, MainThreadMarker, NSString};
use wd40::scanner::ArtifactGroup;

const INTERVALS: &[(u64, &str)] = &[
    (0, "Off"),
    (1, "Every Hour"),
    (6, "Every 6 Hours"),
    (12, "Every 12 Hours"),
    (24, "Every Day"),
];
const AGES: &[(u64, &str)] = &[(3, "3 Days"), (7, "7 Days"), (14, "14 Days"), (30, "30 Days")];

pub fn build_settings_menu(
    state: &AppState,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Settings"));
    menu.setAutoenablesItems(false);

    add_caption(&menu, "Auto Clean", mtm);
    for &(hours, label) in INTERVALS {
        add_choice(&menu, label, hours, state.config.auto_clean_hours, sel!(handleSetAutoInterval:), target, mtm);
    }

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    add_caption(&menu, "Clean Artifacts Older Than", mtm);
    for &(days, label) in AGES {
        add_choice(&menu, label, days, state.config.max_age_days, sel!(handleSetMaxAge:), target, mtm);
    }

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let login = add_toggle(&menu, "Launch at Login", autostart::is_enabled(), sel!(handleToggleLoginItem:), target, mtm);
    login.setTag(0);
    if let Some(updater) = state.updater.as_ref() {
        add_toggle(&menu, "Check for Updates Automatically", updater.automatic_checks(), sel!(handleToggleAutoUpdate:), target, mtm);
    }

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let rules = new_item(ns_string!("Scan Rules"), None, mtm);
    rules.setSubmenu(Some(&build_rules_menu(target, mtm)));
    menu.addItem(&rules);

    menu
}

fn build_rules_menu(target: &AnyObject, mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Scan Rules"));
    menu.setAutoenablesItems(false);
    for &group in ArtifactGroup::ALL {
        let item = new_item(
            &NSString::from_str(group.label()),
            Some(sel!(handleGroupInfo:)),
            mtm,
        );
        item.setTag(group.tag());
        unsafe { item.setTarget(Some(target)) };
        menu.addItem(&item);
    }
    menu
}

/// Radio-style row: checked when `value` is the active setting.
fn add_choice(
    menu: &NSMenu,
    label: &str,
    value: u64,
    active: u64,
    action: Sel,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    let item = new_item(&NSString::from_str(label), Some(action), mtm);
    item.setTag(value as isize);
    item.setState(if value == active { NSControlStateValueOn } else { NSControlStateValueOff });
    unsafe { item.setTarget(Some(target)) };
    menu.addItem(&item);
}

fn add_toggle(
    menu: &NSMenu,
    label: &str,
    on: bool,
    action: Sel,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> Retained<NSMenuItem> {
    let item = new_item(&NSString::from_str(label), Some(action), mtm);
    item.setState(if on { NSControlStateValueOn } else { NSControlStateValueOff });
    unsafe { item.setTarget(Some(target)) };
    menu.addItem(&item);
    item
}
