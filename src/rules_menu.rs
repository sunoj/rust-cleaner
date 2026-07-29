// "Scan Rules" submenu: explains what each artifact group matches.
// Informational only — every adjustable setting lives in the Settings window.
// Exports: `build_rules_menu`.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::menu.

use crate::menu::new_item;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{sel, MainThreadOnly};
use objc2_app_kit::NSMenu;
use objc2_foundation::{ns_string, MainThreadMarker, NSString};
use wd40::scanner::ArtifactGroup;

pub fn build_rules_menu(target: &AnyObject, mtm: MainThreadMarker) -> Retained<NSMenu> {
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
