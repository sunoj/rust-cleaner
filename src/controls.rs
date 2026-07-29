// AppKit control primitives for the Settings window: build them, then read
// them back by tag. Geometry is passed in; layout decisions live in the caller.
// Exports: `add_label`, `add_checkbox_at`, `add_button`, `add_popup`, accessors.
// Deps: objc2, objc2_app_kit, objc2_foundation.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::MainThreadOnly;
use objc2_app_kit::{
    NSBezelStyle, NSButton, NSButtonType, NSColor, NSControlStateValueOff, NSControlStateValueOn,
    NSFont, NSPopUpButton, NSTextField, NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};


pub fn add_label(root: &NSView, text: &str, x: f64, y: f64, width: f64, secondary: bool, mtm: MainThreadMarker) {
    let label = label_field(text, x, y, width, secondary, mtm);
    root.addSubview(&label);
}

pub fn label_field(text: &str, x: f64, y: f64, width: f64, secondary: bool, mtm: MainThreadMarker) -> Retained<NSTextField> {
    let field = {
        NSTextField::initWithFrame(
            NSTextField::alloc(mtm),
            NSRect::new(NSPoint::new(x, y), NSSize::new(width, 18.0)),
        )
    };
    field.setStringValue(&NSString::from_str(text));
    field.setBezeled(false);
    field.setDrawsBackground(false);
    field.setEditable(false);
    field.setSelectable(false);
    if secondary {
        field.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        field.setTextColor(Some(&NSColor::secondaryLabelColor()));
    }
    field
}

#[allow(clippy::too_many_arguments)]
pub fn add_checkbox_at(root: &NSView, title: &str, tag: isize, on: bool, action: Sel, target: &AnyObject, x: f64, y: f64, width: f64, mtm: MainThreadMarker) {
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(&NSString::from_str(title), Some(target), Some(action), mtm)
    };
    button.setButtonType(NSButtonType::Switch);
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, 22.0)));
    button.setTag(tag);
    button.setState(if on { NSControlStateValueOn } else { NSControlStateValueOff });
    root.addSubview(&button);
}

pub fn add_button(root: &NSView, title: &str, action: Sel, target: &AnyObject, x: f64, y: f64, mtm: MainThreadMarker) {
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(&NSString::from_str(title), Some(target), Some(action), mtm)
    };
    button.setBezelStyle(NSBezelStyle::Push);
    button.setFrame(NSRect::new(NSPoint::new(x, y - 2.0), NSSize::new(120.0, 26.0)));
    root.addSubview(&button);
}

#[allow(clippy::too_many_arguments)]
pub fn add_popup(
    root: &NSView,
    title: &str,
    tag: isize,
    options: &[(u64, &str)],
    active: u64,
    action: Sel,
    target: &AnyObject,
    x: f64,
    y: f64,
    mtm: MainThreadMarker,
) {
    add_label(root, title, x, y + 3.0, 130.0, false, mtm);
    let popup = {
        NSPopUpButton::initWithFrame_pullsDown(
            NSPopUpButton::alloc(mtm),
            NSRect::new(NSPoint::new(x + 140.0, y), NSSize::new(180.0, 25.0)),
            false,
        )
    };
    for &(value, label) in options {
        popup.addItemWithTitle(&NSString::from_str(label));
        if let Some(item) = popup.lastItem() {
            item.setTag(value as isize);
        }
    }
    popup.setTag(tag);
    unsafe {
        popup.setTarget(Some(target));
        popup.setAction(Some(action));
    }
    select_value(&popup, active as isize);
    root.addSubview(&popup);
}

pub fn select_value(popup: &NSPopUpButton, tag: isize) {
    let count = popup.numberOfItems();
    for index in 0..count {
        let item = popup.itemAtIndex(index);
        if let Some(item) = item {
            if item.tag() == tag {
                popup.selectItemAtIndex(index);
                return;
            }
        }
    }
}

pub fn find(root: &NSView, tag: isize) -> Option<Retained<NSView>> {
    root.viewWithTag(tag)
}

pub fn set_checkbox(root: &NSView, tag: isize, on: bool) {
    if let Some(view) = find(root, tag) {
        let button: &NSButton = unsafe { &*(&*view as *const NSView as *const NSButton) };
        button.setState(if on { NSControlStateValueOn } else { NSControlStateValueOff });
    }
}

pub fn checkbox_is_on(root: &NSView, tag: isize) -> bool {
    find(root, tag).is_some_and(|view| {
        let button: &NSButton = unsafe { &*(&*view as *const NSView as *const NSButton) };
        button.state() == NSControlStateValueOn
    })
}

pub fn select_tag(root: &NSView, tag: isize, value: isize) {
    if let Some(view) = find(root, tag) {
        let popup: &NSPopUpButton = unsafe { &*(&*view as *const NSView as *const NSPopUpButton) };
        select_value(popup, value);
    }
}

pub fn selected_tag(root: &NSView, tag: isize) -> Option<isize> {
    let view = find(root, tag)?;
    let popup: &NSPopUpButton = unsafe { &*(&*view as *const NSView as *const NSPopUpButton) };
    popup.selectedItem().map(|item| item.tag())
}
