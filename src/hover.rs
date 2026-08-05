// Hover behaviour for project rows: the pointer reveals a row's full path in
// place, since a menu row only has width for a short name.
// Exports: `reset`, `register`, `attach`.
// Deps: objc2, objc2_app_kit, objc2_foundation.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, sel, MainThreadOnly, Message};
use objc2_app_kit::{NSMenu, NSMenuDelegate, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSAttributedString, NSObject, NSObjectProtocol};
use std::cell::RefCell;
use std::collections::HashMap;

/// The two faces of one project row.
struct Titles {
    normal: Retained<NSAttributedString>,
    path: Retained<NSAttributedString>,
}

thread_local! {
    static TITLES: RefCell<HashMap<isize, Titles>> = RefCell::new(HashMap::new());
    static DELEGATE: RefCell<Option<Retained<HoverDelegate>>> = const { RefCell::new(None) };
    static SHOWING: RefCell<Option<Retained<NSMenuItem>>> = const { RefCell::new(None) };
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "HoverDelegate"]
    pub struct HoverDelegate;

    unsafe impl NSObjectProtocol for HoverDelegate {}

    unsafe impl NSMenuDelegate for HoverDelegate {
        #[unsafe(method(menu:willHighlightItem:))]
        fn will_highlight(&self, _menu: &NSMenu, item: Option<&NSMenuItem>) {
            restore();
            if let Some(item) = item {
                show_path(item);
            }
        }

        /// Closing while a row is highlighted would leave the path showing when
        /// the menu opens again.
        #[unsafe(method(menuDidClose:))]
        fn menu_did_close(&self, _menu: &NSMenu) {
            restore();
        }
    }
);

impl HoverDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// Forget the previous menu's rows. Call before rebuilding.
pub fn reset() {
    SHOWING.with(|cell| *cell.borrow_mut() = None);
    TITLES.with(|cell| cell.borrow_mut().clear());
}

/// Record what a row shows normally and what it shows under the pointer.
pub fn register(
    tag: isize,
    normal: Retained<NSAttributedString>,
    path: Retained<NSAttributedString>,
) {
    TITLES.with(|cell| cell.borrow_mut().insert(tag, Titles { normal, path }));
}

pub fn attach(menu: &NSMenu, mtm: MainThreadMarker) {
    let delegate = DELEGATE.with(|cell| {
        let mut slot = cell.borrow_mut();
        slot.get_or_insert_with(|| HoverDelegate::new(mtm)).clone()
    });
    // The menu holds its delegate weakly, so the thread-local above owns it.
    menu.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
}

fn restore() {
    let Some(item) = SHOWING.with(|cell| cell.borrow_mut().take()) else { return };
    TITLES.with(|cell| {
        if let Some(titles) = cell.borrow().get(&item.tag()) {
            item.setAttributedTitle(Some(&titles.normal));
        }
    });
}

fn show_path(item: &NSMenuItem) {
    // Group and action items carry tags of their own; only project rows swap.
    if item.action() != Some(sel!(handleCleanProject:)) {
        return;
    }
    let swapped = TITLES.with(|cell| {
        cell.borrow().get(&item.tag()).map(|titles| item.setAttributedTitle(Some(&titles.path)))
    });
    if swapped.is_some() {
        SHOWING.with(|cell| *cell.borrow_mut() = Some(item.retain()));
    }
}
