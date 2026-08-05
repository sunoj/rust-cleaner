// Menu bar UI for WD-40: status button, disk panel, grouped scan results, actions.
// Exports: `refresh_menu`, `new_item`, `add_caption`, `add_action`.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::{disk_panel, hover, menu_rows, style}.

use crate::disk_panel::{add_disk_panel, DiskPanel};
use crate::hover;
use crate::icon::{rust_text_color, rusty_icon};
use crate::menu_rows::{path_row, plan_groups, project_row, row_width, widest_label, GroupPlan};
use crate::rules_menu::build_rules_menu;
use crate::style::{caption_font, menu_font, symbol_image, tinted, Columns, Row};
use crate::{AppState, HANDLER};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadOnly};
use objc2_app_kit::{NSColor, NSFont, NSMenu, NSMenuItem};
use objc2_foundation::{ns_string, MainThreadMarker, NSString};
use wd40::scanner::{human_size, sizes_may_overlap, TargetDir};

/// How many project rows the menu lists before it starts summarizing.
const INFO_LIMIT: usize = 15;

pub fn refresh_menu(state: &mut AppState, mtm: MainThreadMarker) {
    update_status_button(state, mtm);

    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("WD-40"));
    menu.setAutoenablesItems(false);
    hover::reset();

    HANDLER.with(|cell| {
        let handler = cell.borrow();
        let Some(handler) = handler.as_ref() else { return };
        let target: &AnyObject = unsafe { &*(handler.as_ref() as *const _ as *const AnyObject) };
        build_menu(&menu, state, target, mtm);
    });

    hover::attach(&menu, mtm);
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
    let paths: Vec<_> = state.targets.iter().map(|t| t.path.clone()).collect();
    let overlap = sizes_may_overlap(&paths);

    // Columns come first: the disk panel spans the width the rows below need.
    let font = menu_font();
    let plans = plan_groups(&state.targets, INFO_LIMIT);
    let columns = Columns::for_name_width(widest_label(&plans, &font));
    let width = row_width(columns, &font);

    add_disk_panel(
        menu,
        &DiskPanel {
            disk: state.disk_stats(),
            reclaimable: total,
            sizing,
            approximate: overlap,
        },
        width,
        mtm,
    );
    menu.addItem(&NSMenuItem::separatorItem(mtm));

    if plans.is_empty() {
        add_caption(menu, "Nothing to clean \u{2014} everything is tidy", mtm);
        menu.addItem(&NSMenuItem::separatorItem(mtm));
    } else {
        add_groups(menu, &plans, &state.targets, target, sizing, columns, width, mtm);
    }

    add_clean_actions(menu, state, target, total, sizing, overlap, mtm);

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let settings = add_action(menu, "Settings\u{2026}", sel!(openSettings:), target, mtm);
    settings.setKeyEquivalent(ns_string!(","));
    let rules = new_item(ns_string!("Scan Rules"), None, mtm);
    rules.setSubmenu(Some(&build_rules_menu(target, mtm)));
    menu.addItem(&rules);
    add_updates_item(menu, state, mtm);
    add_caption(menu, &format!("WD-40 v{}", crate::updater::bundle_version()), mtm);

    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let quit = add_action(menu, "Quit WD-40", sel!(quit:), target, mtm);
    quit.setKeyEquivalent(ns_string!("q"));
}

fn add_clean_actions(
    menu: &NSMenu,
    state: &AppState,
    target: &AnyObject,
    total: u64,
    sizing: bool,
    overlap: bool,
    mtm: MainThreadMarker,
) {
    if !sizing {
        // Never state a flat figure when clone sharing can inflate it.
        let label = if overlap {
            format!("Clean All \u{2014} up to {}", human_size(total))
        } else {
            format!("Clean All \u{2014} {}", human_size(total))
        };
        add_action(menu, &label, sel!(handleCleanAll:), target, mtm);
        if overlap {
            add_caption(menu, "sizes overlap where builds share APFS clones", mtm);
        }
    }
    let old_label = format!("Clean Older Than {} Days", state.config.max_age_days);
    add_action(menu, &old_label, sel!(handleCleanOld:), target, mtm);
    let rescan = add_action(menu, "Rescan", sel!(handleRescan:), target, mtm);
    rescan.setKeyEquivalent(ns_string!("r"));
}

fn add_groups(
    menu: &NSMenu,
    plans: &[GroupPlan],
    targets: &[TargetDir],
    target: &AnyObject,
    sizing: bool,
    columns: Columns,
    width: f64,
    mtm: MainThreadMarker,
) {
    let font = menu_font();
    let max_size = targets.iter().map(|td| td.size_bytes).max().unwrap_or(1).max(1);

    for plan in plans {
        add_group_header(menu, plan, sizing, columns, mtm);

        for row in &plan.rows {
            let item = new_item(&NSString::from_str(""), Some(sel!(handleCleanProject:)), mtm);
            let title = project_row(row, max_size, sizing, &font).build(&font, columns);
            item.setAttributedTitle(Some(&title));
            item.setTag(row.index as isize);
            unsafe { item.setTarget(Some(target)) };
            menu.addItem(&item);
            // A row has width for a short name; the pointer is what reveals the
            // path it stands for.
            let path = path_row(row.target, &font, width).build(&font, Columns::default());
            hover::register(row.index as isize, title, path);
        }
        if plan.hidden > 0 {
            add_caption(menu, &format!("{} more not shown", plan.hidden), mtm);
        }

        if !sizing {
            let label =
                format!("Clean {} \u{2014} {}", plan.group.label(), human_size(plan.size));
            let clean = add_action(menu, &label, sel!(handleCleanGroup:), target, mtm);
            clean.setTag(plan.group.tag());
            if let Some(image) = symbol_image("trash", 13.0) {
                clean.setImage(Some(&image));
            }
        }
        menu.addItem(&NSMenuItem::separatorItem(mtm));
    }
}

fn add_group_header(
    menu: &NSMenu,
    plan: &GroupPlan,
    sizing: bool,
    columns: Columns,
    mtm: MainThreadMarker,
) {
    let mut row = Row::new();
    row.push(plan.group.label(), Some(NSColor::secondaryLabelColor()));
    row.push(&format!("  {}", plan.count), Some(NSColor::tertiaryLabelColor()));
    row.tab();
    if !sizing {
        row.push(&human_size(plan.size), Some(NSColor::secondaryLabelColor()));
    }
    let item = new_item(&NSString::from_str(""), None, mtm);
    item.setAttributedTitle(Some(&row.build(&caption_font(), columns)));
    item.setEnabled(false);
    if let Some(image) = symbol_image(plan.group.symbol(), 12.0) {
        item.setImage(Some(&image));
    }
    menu.addItem(&item);
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
    item.setAttributedTitle(Some(&row.build(&caption_font(), Columns::default())));
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
