// NSPopover shell for WD-40: toggle from the status item, rebuild per screen.
// Exports: `attach`, `toggle`, `refresh`, `close`.
// Deps: objc2 AppKit; crate view modules + state + theme.

use crate::clean_view;
use crate::done_view;
use crate::icon::{rust_text_color, rusty_icon};
use crate::scan_view;
use crate::settings_view;
use crate::state::{with_state_ret, AppState, UiScreen};
use crate::style::tinted;
use crate::theme::Theme;
use crate::widgets::POPOVER_WIDTH;
use crate::{MenuHandler, HANDLER};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{sel, MainThreadOnly};
use objc2_app_kit::{NSEventMask, NSFont, NSPopover, NSPopoverBehavior, NSView, NSViewController};
use objc2_foundation::{ns_string, MainThreadMarker, NSPoint, NSRect, NSRectEdge, NSSize, NSString};
use std::cell::RefCell;
use wd40::scanner::human_size;

thread_local! {
    static POPOVER: RefCell<Option<Retained<NSPopover>>> = const { RefCell::new(None) };
    static CONTROLLER: RefCell<Option<Retained<NSViewController>>> = const { RefCell::new(None) };
}

/// Wire the status button to toggle the popover instead of showing an NSMenu.
pub fn attach(mtm: MainThreadMarker) {
    let handler = HANDLER.with(|cell| cell.borrow().clone());
    let button = with_state_ret(|state| state.status_item.button(mtm)).flatten();
    if let (Some(handler), Some(button)) = (handler, button) {
        let target: &AnyObject = unsafe { &*(handler.as_ref() as *const _ as *const AnyObject) };
        unsafe {
            button.setTarget(Some(target));
            button.setAction(Some(sel!(togglePopover:)));
        }
        // Fire on mouse-down, not mouse-up. A transient popover dismisses itself
        // the moment you press outside it — including on this button — so by
        // mouse-up it is already closed and the toggle would just reopen it,
        // which reads as a popover that cannot be clicked shut.
        button.sendActionOn(NSEventMask::LeftMouseDown);
        button.setToolTip(Some(&NSString::from_str("WD-40")));
    }
    ensure_popover(mtm);
    refresh(mtm);
}

pub fn toggle(mtm: MainThreadMarker) {
    if POPOVER.with(|cell| cell.borrow().as_ref().is_some_and(|p| p.isShown())) {
        close();
        return;
    }
    refresh(mtm);
    show(mtm);
}

pub fn close() {
    POPOVER.with(|cell| {
        if let Some(popover) = cell.borrow().as_ref() {
            popover.close();
        }
    });
}

/// Open the popover if it is not already shown.
#[cfg(debug_assertions)]
pub fn ensure_open(mtm: MainThreadMarker) {
    let shown = POPOVER.with(|cell| cell.borrow().as_ref().is_some_and(|p| p.isShown()));
    if !shown {
        show(mtm);
    }
}

/// Content view currently hosted by the popover, for screenshot capture.
#[cfg(debug_assertions)]
pub fn content_view(mtm: MainThreadMarker) -> Option<Retained<NSView>> {
    let _ = mtm;
    CONTROLLER.with(|cell| cell.borrow().as_ref().map(|c| c.view()))
}

/// Rebuild popover content from current `AppState` (keeps it open if shown).
pub fn refresh(mtm: MainThreadMarker) {
    ensure_popover(mtm);
    crate::widgets::remember_scroll();
    let was_shown = POPOVER.with(|cell| cell.borrow().as_ref().is_some_and(|p| p.isShown()));
    let (view, height) = with_state_ret(|state| {
        update_status_button(state, mtm);
        build_for_state(state, mtm)
    })
    .unwrap_or_else(|| empty_view(mtm));

    CONTROLLER.with(|cell| {
        if let Some(controller) = cell.borrow().as_ref() {
            controller.setView(&view);
        }
    });
    POPOVER.with(|cell| {
        if let Some(popover) = cell.borrow().as_ref() {
            popover.setContentSize(NSSize::new(POPOVER_WIDTH, height));
        }
    });
    if was_shown {
        show(mtm);
    }
}

fn build_for_state(state: &AppState, mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let Some(handler) = HANDLER.with(|cell| cell.borrow().clone()) else {
        return empty_view(mtm);
    };
    let target: &AnyObject =
        unsafe { &*(handler.as_ref() as *const MenuHandler as *const AnyObject) };
    let theme = status_theme(state, mtm);
    match state.screen {
        UiScreen::Scan => scan_view::build(state, &theme, target, mtm),
        UiScreen::Cleaning => clean_view::build(state, &theme, target, mtm),
        UiScreen::Done => done_view::build(state, &theme, target, mtm),
        UiScreen::Settings => settings_view::build(state, &theme, target, mtm),
    }
}

fn empty_view(mtm: MainThreadMarker) -> (Retained<NSView>, f64) {
    let view = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(POPOVER_WIDTH, 80.0)),
    );
    (view, 80.0)
}

fn ensure_popover(mtm: MainThreadMarker) {
    if POPOVER.with(|cell| cell.borrow().is_some()) {
        return;
    }
    let controller =
        NSViewController::initWithNibName_bundle(NSViewController::alloc(mtm), None, None);
    let popover = NSPopover::new(mtm);
    let behavior = {
        #[cfg(debug_assertions)]
        {
            if crate::screenshot::enabled() {
                NSPopoverBehavior::ApplicationDefined
            } else {
                NSPopoverBehavior::Transient
            }
        }
        #[cfg(not(debug_assertions))]
        {
            NSPopoverBehavior::Transient
        }
    };
    popover.setBehavior(behavior);
    popover.setAnimates(false);
    popover.setContentViewController(Some(&controller));
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(controller));
    POPOVER.with(|cell| *cell.borrow_mut() = Some(popover));
}

fn show(mtm: MainThreadMarker) {
    let button = with_state_ret(|state| state.status_item.button(mtm)).flatten();
    let Some(button) = button else { return };
    let popover = POPOVER.with(|cell| cell.borrow().clone());
    if let Some(popover) = popover {
        popover.showRelativeToRect_ofView_preferredEdge(
            button.bounds(),
            &button,
            NSRectEdge::MaxY,
        );
    }
}

fn status_theme(state: &AppState, mtm: MainThreadMarker) -> Theme {
    if let Ok(value) = std::env::var("WD40_APPEARANCE") {
        if value.eq_ignore_ascii_case("light") {
            return Theme::light();
        }
        if value.eq_ignore_ascii_case("dark") {
            return Theme::dark_theme();
        }
    }
    let _ = state;
    Theme::current_app(mtm)
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
