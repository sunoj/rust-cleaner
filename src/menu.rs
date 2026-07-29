// Menu bar UI for WD-40: status button, grouped scan results, settings, updates.
// Exports: `refresh_menu`, `project_name`.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::{icon, settings, style}.

use crate::icon::{rust_text_color, rusty_icon};
use crate::settings::build_settings_menu;
use crate::style::{caption_font, menu_font, symbol_image, tinted, truncate, Row};
use crate::{AppState, HANDLER};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadOnly};
use objc2_app_kit::{NSColor, NSFont, NSMenu, NSMenuItem};
use objc2_foundation::{ns_string, MainThreadMarker, NSString};
use wd40::disk::sum_bytes;
use wd40::scanner::{human_size, ArtifactGroup, ArtifactKind, TargetDir};

const INFO_LIMIT: usize = 15;
const MAX_BAR: usize = 8;
const NAME_CHARS: usize = 28;

pub fn refresh_menu(state: &mut AppState, mtm: MainThreadMarker) {
    update_status_button(state, mtm);

    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("WD-40"));
    menu.setAutoenablesItems(false);

    HANDLER.with(|cell| {
        let handler = cell.borrow();
        let Some(handler) = handler.as_ref() else { return };
        let target: &AnyObject = unsafe { &*(handler.as_ref() as *const _ as *const AnyObject) };
        build_menu(&menu, state, target, mtm);
    });

    state.status_item.setMenu(Some(&menu));
}

fn update_status_button(state: &AppState, mtm: MainThreadMarker) {
    let total = state.total_size();
    let Some(button) = state.status_item.button(mtm) else { return };
    if let Some(image) = rusty_icon(total) {
        button.setImage(Some(&image));
    }
    if total < 1024 * 1024 * 1024 {
        button.setTitle(ns_string!(""));
        return;
    }
    let title = format!(" {}", human_size(total));
    let font = NSFont::menuBarFontOfSize(0.0);
    button.setAttributedTitle(&tinted(&title, &font, &rust_text_color(total)));
}

fn build_menu(menu: &NSMenu, state: &AppState, target: &AnyObject, mtm: MainThreadMarker) {
    let total = state.total_size();
    let sizing = total == 0 && !state.targets.is_empty();

    add_header(menu, state, total, sizing, mtm);
    menu.addItem(&NSMenuItem::separatorItem(mtm));

    if state.targets.is_empty() {
        add_caption(menu, "Nothing to clean \u{2014} everything is tidy", mtm);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
    } else {
        add_groups(menu, state, target, sizing, mtm);
    }

    if !sizing {
        let label = format!("Clean All \u{2014} {}", human_size(total));
        add_action(menu, &label, sel!(handleCleanAll:), target, mtm);
    }
    let old_label = format!("Clean Older Than {} Days", state.config.max_age_days);
    add_action(menu, &old_label, sel!(handleCleanOld:), target, mtm);
    let rescan = add_action(menu, "Rescan", sel!(handleRescan:), target, mtm);
    rescan.setKeyEquivalent(ns_string!("r"));

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let settings = new_item(ns_string!("Settings"), None, mtm);
    settings.setSubmenu(Some(&build_settings_menu(state, target, mtm)));
    menu.addItem(&settings);
    add_updates_item(menu, state, mtm);

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let quit = add_action(menu, "Quit WD-40", sel!(quit:), target, mtm);
    quit.setKeyEquivalent(ns_string!("q"));
}

fn add_header(menu: &NSMenu, state: &AppState, total: u64, sizing: bool, mtm: MainThreadMarker) {
    let mut row = Row::new();
    row.push("WD-40", None);
    row.tab();
    if sizing {
        row.push("scanning\u{2026}", Some(NSColor::secondaryLabelColor()));
    } else {
        row.push(&human_size(total), Some(rust_text_color(total)));
    }
    let item = new_item(&NSString::from_str(""), None, mtm);
    item.setAttributedTitle(Some(&row.build(&menu_font())));
    item.setEnabled(false);
    menu.addItem(&item);

    let mut caption = format!("v{}", crate::updater::bundle_version());
    if let Some(disk) = state.disk_stats() {
        caption.push_str(&format!(
            "  \u{2022}  {} free of {}",
            human_size(disk.free_bytes),
            human_size(disk.total_bytes)
        ));
    }
    add_caption(menu, &caption, mtm);
}

fn add_groups(
    menu: &NSMenu,
    state: &AppState,
    target: &AnyObject,
    sizing: bool,
    mtm: MainThreadMarker,
) {
    let max_size = state.targets.iter().map(|t| t.size_bytes).max().unwrap_or(1).max(1);
    let mut shown = 0usize;

    for &group in ArtifactGroup::ALL {
        let items: Vec<(usize, &TargetDir)> = state
            .targets
            .iter()
            .enumerate()
            .filter(|(_, td)| td.kind.group() == group)
            .collect();
        if items.is_empty() {
            continue;
        }

        let group_size = sum_bytes(items.iter().map(|(_, td)| td.size_bytes));
        add_group_header(menu, group, items.len(), group_size, sizing, mtm);

        // The kind only adds information where a group mixes kinds (target vs
        // tmp-target); elsewhere it just repeats the header and steals width.
        let first_kind = items[0].1.kind.label();
        let mixed = items.iter().any(|(_, td)| td.kind.label() != first_kind);

        let visible = items.len().min(INFO_LIMIT.saturating_sub(shown));
        for &(index, td) in items.iter().take(visible) {
            let item = new_item(&NSString::from_str(""), Some(sel!(handleCleanProject:)), mtm);
            let row = project_row(td, max_size, sizing, mixed);
            item.setAttributedTitle(Some(&row.build(&menu_font())));
            item.setTag(index as isize);
            unsafe { item.setTarget(Some(target)) };
            menu.addItem(&item);
            shown += 1;
        }
        if items.len() > visible {
            add_caption(menu, &format!("{} more not shown", items.len() - visible), mtm);
        }

        if !sizing {
            let label = format!("Clean {} \u{2014} {}", group.label(), human_size(group_size));
            let clean = add_action(menu, &label, sel!(handleCleanGroup:), target, mtm);
            clean.setTag(group.tag());
            if let Some(image) = symbol_image("trash", 13.0) {
                clean.setImage(Some(&image));
            }
        }
        menu.addItem(&NSMenuItem::separatorItem(mtm));
    }
}

fn add_group_header(
    menu: &NSMenu,
    group: ArtifactGroup,
    count: usize,
    group_size: u64,
    sizing: bool,
    mtm: MainThreadMarker,
) {
    let mut row = Row::new();
    row.push(group.label(), Some(NSColor::secondaryLabelColor()));
    row.push(&format!("  {count}"), Some(NSColor::tertiaryLabelColor()));
    row.tab();
    if !sizing {
        row.push(&human_size(group_size), Some(NSColor::secondaryLabelColor()));
    }
    let item = new_item(&NSString::from_str(""), None, mtm);
    item.setAttributedTitle(Some(&row.build(&caption_font())));
    item.setEnabled(false);
    if let Some(image) = symbol_image(group.symbol(), 12.0) {
        item.setImage(Some(&image));
    }
    menu.addItem(&item);
}

fn project_row(td: &TargetDir, max_size: u64, sizing: bool, show_kind: bool) -> Row {
    let mut row = Row::new();
    // Name and kind share one column; overflowing it would shove the size past
    // its tab stop and wrap the bar onto a second line.
    let kind = if show_kind { td.kind.label() } else { "" };
    let budget = NAME_CHARS.saturating_sub(if kind.is_empty() { 0 } else { kind.chars().count() + 2 });
    row.push(&truncate(&project_name(td), budget), None);
    if !kind.is_empty() {
        row.push(&format!("  {kind}"), Some(NSColor::tertiaryLabelColor()));
    }
    row.tab();
    if sizing {
        row.push("\u{2026}", Some(NSColor::tertiaryLabelColor()));
        return row;
    }
    row.push(&human_size(td.size_bytes), Some(NSColor::secondaryLabelColor()));
    row.tab();
    let ratio = td.size_bytes as f64 / max_size as f64;
    let filled = (ratio * MAX_BAR as f64).ceil().max(1.0) as usize;
    row.push(&"\u{2588}".repeat(filled.min(MAX_BAR)), Some(NSColor::tertiaryLabelColor()));
    row
}

fn add_updates_item(menu: &NSMenu, state: &AppState, mtm: MainThreadMarker) {
    let Some(updater) = state.updater.as_ref() else { return };
    let item = new_item(ns_string!("Check for Updates\u{2026}"), Some(sel!(checkForUpdates:)), mtm);
    unsafe { item.setTarget(Some(updater.target())) };
    item.setEnabled(updater.can_check());
    menu.addItem(&item);
}

pub(crate) fn new_item(
    title: &NSString,
    action: Option<Sel>,
    mtm: MainThreadMarker,
) -> Retained<NSMenuItem> {
    unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            title,
            action,
            ns_string!(""),
        )
    }
}

pub(crate) fn add_caption(menu: &NSMenu, text: &str, mtm: MainThreadMarker) {
    let mut row = Row::new();
    row.push(text, Some(NSColor::secondaryLabelColor()));
    let item = new_item(&NSString::from_str(""), None, mtm);
    item.setAttributedTitle(Some(&row.build(&caption_font())));
    item.setEnabled(false);
    menu.addItem(&item);
}

pub(crate) fn add_action(
    menu: &NSMenu,
    text: &str,
    action: Sel,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> Retained<NSMenuItem> {
    let item = new_item(&NSString::from_str(text), Some(action), mtm);
    unsafe { item.setTarget(Some(target)) };
    menu.addItem(&item);
    item
}

pub fn project_name(td: &TargetDir) -> String {
    match td.kind {
        ArtifactKind::TmpTarget => {
            let dir_name = td.path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            // Strip the cc-target- prefix when present; otherwise show as-is so
            // names like "smart-router-target" stay recognizable.
            dir_name.strip_prefix("cc-target-").unwrap_or(dir_name).to_string()
        }
        _ => {
            if let Some(home) = dirs::home_dir() {
                // Under ~/.cargo-target/<project>[/<session>]/, show path relative to the root.
                let shared_root = home.join(".cargo-target");
                if let Ok(rel) = td.path.strip_prefix(&shared_root) {
                    return rel.to_string_lossy().into_owned();
                }
                // Under ~/.aid/worktrees/<repo>/<branch>/target, show "<repo>/<branch>".
                let aid_root = home.join(".aid").join("worktrees");
                if let Ok(rel) = td.path.strip_prefix(&aid_root) {
                    if let Some(parent) = rel.parent() {
                        let name = parent.to_string_lossy();
                        if !name.is_empty() {
                            return name.into_owned();
                        }
                    }
                }
            }
            // Standard target/node_modules/.next — show the containing project dir name.
            td.path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        }
    }
}
