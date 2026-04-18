// Menu bar UI construction for WD-40. Builds the NSMenu with scan results and actions.
// Exports: `refresh_menu`, `project_name`.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::icon, crate::scanner.

use crate::icon::{rust_text_color, rusty_icon};
use crate::{AppState, HANDLER};
use rust_cleaner::disk::sum_bytes;
use rust_cleaner::scanner::{human_size, ArtifactGroup, ArtifactKind, TargetDir};
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, MainThreadOnly};
use objc2_app_kit::{
    NSMenu, NSMenuItem, NSFont, NSFontAttributeName, NSForegroundColorAttributeName,
};
use objc2_foundation::{ns_string, MainThreadMarker, NSDictionary, NSString, NSAttributedString};

const INFO_LIMIT: usize = 15;
const MAX_BAR: usize = 12;

pub fn refresh_menu(state: &mut AppState, mtm: MainThreadMarker) {
    let total = state.total_size();

    if let Some(button) = state.status_item.button(mtm) {
        if let Some(img) = rusty_icon(total) {
            button.setImage(Some(&img));
        }
        if total < 1024 * 1024 * 1024 {
            button.setTitle(ns_string!(""));
        } else {
            let title_text = format!(" {}", human_size(total));
            let color = rust_text_color(total);
            let font = NSFont::menuBarFontOfSize(0.0);
            let color_obj: &AnyObject = unsafe { &*(&*color as *const _ as *const AnyObject) };
            let font_obj: &AnyObject = unsafe { &*(&*font as *const _ as *const AnyObject) };
            let fg_key: &NSString = unsafe { NSForegroundColorAttributeName };
            let font_key: &NSString = unsafe { NSFontAttributeName };
            let attrs = NSDictionary::<NSString, AnyObject>::from_slices::<NSString>(
                &[fg_key, font_key],
                &[color_obj, font_obj],
            );
            let attr_str = unsafe {
                NSAttributedString::new_with_attributes(&NSString::from_str(&title_text), &attrs)
            };
            button.setAttributedTitle(&attr_str);
        }
    }

    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("WD-40"));
    menu.setAutoenablesItems(false);

    add_disabled(&menu, "WD-40", mtm);
    menu.addItem(&NSMenuItem::separatorItem(mtm));

    HANDLER.with(|cell| {
        let handler = cell.borrow();
        let handler = match handler.as_ref() {
            Some(h) => h,
            None => return,
        };
        let target: &AnyObject = unsafe { &*(handler.as_ref() as *const _ as *const AnyObject) };

        let sizing = total == 0 && !state.targets.is_empty();

        if state.targets.is_empty() {
            add_disabled(&menu, "No targets found", mtm);
        } else {
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
                let header = if sizing {
                    format!("{} — {} projects", group.label(), items.len())
                } else {
                    format!("{} — {}", group.label(), human_size(group_size))
                };
                add_disabled(&menu, &header, mtm);

                let info_item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str("  \u{24d8} Scan rules"),
                        Some(sel!(handleGroupInfo:)),
                        ns_string!(""),
                    )
                };
                info_item.setTag(group.tag());
                unsafe { info_item.setTarget(Some(target)) };
                menu.addItem(&info_item);

                let budget = INFO_LIMIT.saturating_sub(shown);
                let show_count = items.len().min(budget);
                for &(i, td) in items.iter().take(show_count) {
                    let title = if sizing {
                        format!("  {}  [{}]  —  ...", project_name(td), td.kind.label())
                    } else {
                        let bar_len = ((td.size_bytes as f64 / max_size as f64) * MAX_BAR as f64)
                            .ceil()
                            .max(1.0) as usize;
                        format!(
                            "  {}  [{}]  —  {}  {}",
                            project_name(td), td.kind.label(), human_size(td.size_bytes), "█".repeat(bar_len),
                        )
                    };

                    let item = unsafe {
                        NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(mtm),
                            &NSString::from_str(&title),
                            Some(sel!(handleCleanProject:)),
                            ns_string!(""),
                        )
                    };
                    item.setTag(i as isize);
                    unsafe { item.setTarget(Some(target)) };
                    menu.addItem(&item);
                    shown += 1;
                }
                if items.len() > show_count {
                    add_disabled(&menu, &format!("  ... {} more", items.len() - show_count), mtm);
                }

                if !sizing {
                    let clean_label = format!("  Clean {} ({})", group.label(), human_size(group_size));
                    let clean_item = unsafe {
                        NSMenuItem::initWithTitle_action_keyEquivalent(
                            NSMenuItem::alloc(mtm),
                            &NSString::from_str(&clean_label),
                            Some(sel!(handleCleanGroup:)),
                            ns_string!(""),
                        )
                    };
                    clean_item.setTag(group.tag());
                    unsafe { clean_item.setTarget(Some(target)) };
                    menu.addItem(&clean_item);
                }

                menu.addItem(&NSMenuItem::separatorItem(mtm));
            }
        }

        menu.addItem(&NSMenuItem::separatorItem(mtm));
        if sizing {
            add_disabled(&menu, &format!("{} projects — computing sizes...", state.targets.len()), mtm);
        } else {
            add_disabled(&menu, &format!("Total: {} in {} projects", human_size(total), state.targets.len()), mtm);
            if let Some(free_bytes) = state.remaining_disk_space() {
                add_disabled(&menu, &format!("Remaining Disk Space: {}", human_size(free_bytes)), mtm);
            }
        }
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        if !sizing {
            add_action(&menu, &format!("Clean All ({})", human_size(total)), sel!(handleCleanAll:), target, mtm);
        }
        add_action(&menu, &format!("Clean Old (>{}d)", state.config.max_age_days), sel!(handleCleanOld:), target, mtm);
        add_action(&menu, "Rescan", sel!(handleRescan:), target, mtm);

        // Auto Clean submenu
        let auto_menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), ns_string!("Auto Clean"));
        auto_menu.setAutoenablesItems(false);

        add_disabled(&auto_menu, "Interval", mtm);
        let intervals: &[(u64, &str)] = &[(0, "Off"), (1, "Every 1h"), (6, "Every 6h"), (12, "Every 12h"), (24, "Every 24h")];
        for &(hours, label) in intervals {
            let text = if hours == state.config.auto_clean_hours {
                format!("{} ✓", label)
            } else {
                label.to_string()
            };
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm), &NSString::from_str(&text), Some(sel!(handleSetAutoInterval:)), ns_string!(""),
                )
            };
            item.setTag(hours as isize);
            unsafe { item.setTarget(Some(target)) };
            auto_menu.addItem(&item);
        }

        auto_menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_disabled(&auto_menu, "Clean older than", mtm);
        let ages: &[(u64, &str)] = &[(3, "3 days"), (7, "7 days"), (14, "14 days"), (30, "30 days")];
        for &(days, label) in ages {
            let text = if days == state.config.max_age_days {
                format!("{} ✓", label)
            } else {
                label.to_string()
            };
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm), &NSString::from_str(&text), Some(sel!(handleSetMaxAge:)), ns_string!(""),
                )
            };
            item.setTag(days as isize);
            unsafe { item.setTarget(Some(target)) };
            auto_menu.addItem(&item);
        }

        let auto_item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(NSMenuItem::alloc(mtm), ns_string!("Auto Clean"), None, ns_string!(""))
        };
        auto_item.setSubmenu(Some(&auto_menu));
        menu.addItem(&auto_item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_action(&menu, "Quit", sel!(quit:), target, mtm);
    });

    state.status_item.setMenu(Some(&menu));
}

fn add_disabled(menu: &NSMenu, text: &str, mtm: MainThreadMarker) {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm), &NSString::from_str(text), None, ns_string!("")
        )
    };
    item.setEnabled(false);
    menu.addItem(&item);
}

fn add_action(menu: &NSMenu, text: &str, action: Sel, target: &AnyObject, mtm: MainThreadMarker) {
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm), &NSString::from_str(text), Some(action), ns_string!("")
        )
    };
    unsafe { item.setTarget(Some(target)) };
    menu.addItem(&item);
}

pub fn project_name(td: &TargetDir) -> String {
    let name = match td.kind {
        ArtifactKind::CcTarget => {
            let dir_name = td.path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
            return dir_name.strip_prefix("cc-target-").unwrap_or(dir_name).to_string();
        }
        _ => td.path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or("unknown"),
    };
    name.to_string()
}
