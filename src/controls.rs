// Interactive AppKit controls for the popover (buttons, checkbox, pill, slider).
// Exports: button/checkbox/switch/slider helpers used by views.
// Deps: crate::{theme, widgets}, objc2 AppKit.

use crate::theme::Theme;
use crate::widgets::add_fill;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::MainThreadOnly;
use objc2_app_kit::{
    NSButton, NSButtonType, NSColor, NSEventModifierFlags, NSFont, NSFontAttributeName,
    NSForegroundColorAttributeName, NSUnderlineStyleAttributeName, NSSlider,
};
use objc2_foundation::{
    MainThreadMarker, NSAttributedString, NSDictionary, NSPoint, NSRect, NSSize, NSNumber, NSString,
};

fn attributed(text: &str, rgb: (f64, f64, f64), size: f64, bold: bool) -> Retained<NSAttributedString> {
    let font = if bold {
        NSFont::boldSystemFontOfSize(size)
    } else {
        NSFont::systemFontOfSize(size)
    };
    let color = Theme::color(rgb);
    let font_obj: &AnyObject = unsafe { &*(&*font as *const NSFont as *const AnyObject) };
    let color_obj: &AnyObject = unsafe { &*(&*color as *const NSColor as *const AnyObject) };
    let attrs = NSDictionary::<NSString, AnyObject>::from_slices::<NSString>(
        &[unsafe { NSForegroundColorAttributeName }, unsafe { NSFontAttributeName }],
        &[color_obj, font_obj],
    );
    unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(text), &attrs) }
}

#[allow(clippy::too_many_arguments)]
pub fn text_button(
    parent: &objc2_app_kit::NSView,
    title: &str,
    x: f64,
    y: f64,
    w: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Retained<NSButton> {
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(action),
            mtm,
        )
    };
    button.setBordered(false);
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(w, 22.0)));
    button.setTag(tag);
    button.setAttributedTitle(&attributed(title, color, 12.5, false));
    parent.addSubview(&button);
    button
}

pub fn text_button_underlined(
    parent: &objc2_app_kit::NSView,
    title: &str,
    x: f64,
    y: f64,
    w: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Retained<NSButton> {
    let button = text_button(parent, title, x, y, w, action, target, tag, color, mtm);
    let font = NSFont::systemFontOfSize(12.5);
    let tint = Theme::color(color);
    let font_obj: &AnyObject = unsafe { &*(&*font as *const NSFont as *const AnyObject) };
    let tint_obj: &AnyObject = unsafe { &*(&*tint as *const NSColor as *const AnyObject) };
    let underline = NSNumber::numberWithDouble(1.0);
    let underline_obj: &AnyObject = unsafe { &*(&*underline as *const NSNumber as *const AnyObject) };
    let attrs = NSDictionary::<NSString, AnyObject>::from_slices::<NSString>(
        &[unsafe { NSForegroundColorAttributeName }, unsafe { NSFontAttributeName }, unsafe { NSUnderlineStyleAttributeName }],
        &[tint_obj, font_obj, underline_obj],
    );
    let title = unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(title), &attrs) };
    button.setAttributedTitle(&title);
    button
}

/// Text button plus a faint ⌘ shortcut hint drawn beside it.
#[allow(clippy::too_many_arguments)]
pub fn text_button_hint(
    parent: &objc2_app_kit::NSView,
    title: &str,
    hint: &str,
    x: f64,
    y: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    color: (f64, f64, f64),
    hint_color: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Retained<NSButton> {
    let button = text_button(parent, title, x, y, 58.0, action, target, tag, color, mtm);
    crate::widgets::label(parent, hint, x + 58.0, y + 2.0, 28.0, 16.0, 11.0, false, hint_color, true, mtm);
    button
}

#[allow(clippy::too_many_arguments)]
pub fn filled_button(
    parent: &objc2_app_kit::NSView,
    title: &str,
    x: f64,
    y: f64,
    w: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    fill: (f64, f64, f64),
    ink: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Retained<NSButton> {
    add_fill(parent, x, y, w, 34.0, fill, 1.0, 8.0, mtm);
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(action),
            mtm,
        )
    };
    button.setBordered(false);
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(w, 34.0)));
    button.setTag(tag);
    button.setAttributedTitle(&attributed(title, ink, 13.5, true));
    parent.addSubview(&button);
    button
}

#[allow(clippy::too_many_arguments)]
pub fn clean_button(
    parent: &objc2_app_kit::NSView,
    title: &str,
    size: &str,
    x: f64,
    y: f64,
    w: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    theme: &Theme,
    mtm: MainThreadMarker,
) {
    add_fill(parent, x, y, w, 34.0, theme.ink, 1.0, 8.0, mtm);
    crate::widgets::symbol_view(parent, "trash", x + 76.0, y + 10.0, 13.0, theme.surface, mtm);
    crate::widgets::label(parent, title, x + 97.0, y + 8.0, 132.0, 18.0, 13.5, false, theme.surface, false, mtm);
    let amount = crate::widgets::label(parent, size, x + 232.0, y + 9.0, 54.0, 16.0, 12.5, false, theme.surface, true, mtm);
    amount.setAlphaValue(0.62);
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(&NSString::from_str(""), Some(target), Some(action), mtm)
    };
    button.setBordered(false);
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(w, 34.0)));
    button.setTag(tag);
    parent.addSubview(&button);
}

#[allow(clippy::too_many_arguments)]
pub fn checkbox(
    parent: &objc2_app_kit::NSView,
    on: bool,
    x: f64,
    y: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    theme: &Theme,
    mtm: MainThreadMarker,
) -> Retained<NSButton> {
    if on {
        add_fill(parent, x, y, 15.0, 15.0, theme.ink, 1.0, 4.0, mtm);
    } else {
        let box_ = add_fill(parent, x, y, 15.0, 15.0, theme.surface_2, 0.78, 4.0, mtm);
        box_.setBorderWidth(1.0);
        box_.setBorderColor(&Theme::color(theme.line_2));
    }
    let title = if on { "\u{2713}" } else { "" };
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(action),
            mtm,
        )
    };
    button.setButtonType(NSButtonType::MomentaryChange);
    button.setBordered(false);
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(15.0, 15.0)));
    button.setTag(tag);
    if on {
        button.setAttributedTitle(&attributed("\u{2713}", theme.surface, 10.0, true));
    }
    parent.addSubview(&button);
    button
}

/// Pill-shaped toggle track + knob (mock: 38×22, radius 99).
#[allow(clippy::too_many_arguments)]
pub fn pill_switch(
    parent: &objc2_app_kit::NSView,
    on: bool,
    x: f64,
    y: f64,
    action: Sel,
    tag: isize,
    theme: &Theme,
    target: &AnyObject,
    mtm: MainThreadMarker,
) {
    let track = if on { theme.ink } else { theme.surface_3 };
    add_fill(parent, x, y, 38.0, 22.0, track, 1.0, 11.0, mtm);
    let knob_x = if on { x + 18.0 } else { x + 2.0 };
    add_fill(parent, knob_x, y + 2.0, 18.0, 18.0, theme.surface, 1.0, 9.0, mtm);
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(&NSString::from_str(""), Some(target), Some(action), mtm)
    };
    button.setBordered(false);
    button.setButtonType(NSButtonType::MomentaryChange);
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(38.0, 22.0)));
    button.setTag(tag);
    parent.addSubview(&button);
}

/// Keep-days NSSlider (0–30) matching the mock safety control.
#[allow(clippy::too_many_arguments)]
pub fn days_slider(
    parent: &objc2_app_kit::NSView,
    days: u64,
    x: f64,
    y: f64,
    w: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    mtm: MainThreadMarker,
) -> Retained<NSSlider> {
    let slider = NSSlider::initWithFrame(
        NSSlider::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, 22.0)),
    );
    slider.setMinValue(0.0);
    slider.setMaxValue(30.0);
    slider.setDoubleValue(days.min(30) as f64);
    slider.setContinuous(false);
    slider.setTag(tag);
    unsafe {
        slider.setTarget(Some(target));
        slider.setAction(Some(action));
    }
    parent.addSubview(&slider);
    slider
}

/// Attach ⌘ + key as the button's key equivalent.
pub fn set_cmd_key(button: &NSButton, key: &str) {
    button.setKeyEquivalent(&NSString::from_str(key));
    button.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
}
