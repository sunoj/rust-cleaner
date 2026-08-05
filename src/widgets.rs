// Frame-based AppKit primitives for the 380pt popover (labels, fills, buttons).
// Exports: layout helpers used by scan/clean/done/settings views.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::theme::Theme.

use crate::theme::Theme;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::MainThreadOnly;
use objc2_app_kit::{
    NSBox, NSBoxType, NSButton, NSButtonType, NSColor, NSFont, NSFontAttributeName,
    NSForegroundColorAttributeName, NSTextAlignment, NSTextField, NSView,
};
use objc2_foundation::{
    MainThreadMarker, NSAttributedString, NSDictionary, NSPoint, NSRect, NSSize, NSString,
};

pub const POPOVER_WIDTH: f64 = 380.0;
pub const PAD_X: f64 = 16.0;
pub const CONTENT_WIDTH: f64 = POPOVER_WIDTH - PAD_X * 2.0;

pub fn root_view(height: f64, fill: (f64, f64, f64), mtm: MainThreadMarker) -> Retained<NSView> {
    let view = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(POPOVER_WIDTH, height)),
    );
    add_fill(&view, 0.0, 0.0, POPOVER_WIDTH, height, fill, 1.0, mtm);
    view
}

#[allow(clippy::too_many_arguments)]
pub fn add_fill(
    parent: &NSView,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    rgb: (f64, f64, f64),
    alpha: f64,
    mtm: MainThreadMarker,
) -> Retained<NSBox> {
    let box_ = NSBox::initWithFrame(
        NSBox::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(w.max(0.5), h.max(0.5))),
    );
    box_.setBoxType(NSBoxType::Custom);
    box_.setBorderWidth(0.0);
    box_.setFillColor(&Theme::color_alpha(rgb, alpha));
    parent.addSubview(&box_);
    box_
}

pub fn add_line(
    parent: &NSView,
    x: f64,
    y: f64,
    w: f64,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) {
    add_fill(parent, x, y, w, 1.0, color, 1.0, mtm);
}

#[allow(clippy::too_many_arguments)]
pub fn label(
    parent: &NSView,
    text: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    size: f64,
    bold: bool,
    color: (f64, f64, f64),
    mono: bool,
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    let field = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)),
    );
    field.setStringValue(&NSString::from_str(text));
    field.setBezeled(false);
    field.setDrawsBackground(false);
    field.setEditable(false);
    field.setSelectable(false);
    field.setTextColor(Some(&Theme::color(color)));
    let font = if mono {
        NSFont::monospacedSystemFontOfSize_weight(size, 0.23)
    } else if bold {
        NSFont::boldSystemFontOfSize(size)
    } else {
        NSFont::systemFontOfSize(size)
    };
    field.setFont(Some(&font));
    parent.addSubview(&field);
    field
}

#[allow(clippy::too_many_arguments)]
pub fn label_right(
    parent: &NSView,
    text: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    size: f64,
    color: (f64, f64, f64),
    mono: bool,
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    let field = label(parent, text, x, y, w, h, size, false, color, mono, mtm);
    field.setAlignment(NSTextAlignment::Right);
    field
}

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
    parent: &NSView,
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

#[allow(clippy::too_many_arguments)]
pub fn filled_button(
    parent: &NSView,
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
    add_fill(parent, x, y, w, 34.0, fill, 1.0, mtm);
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
pub fn checkbox(
    parent: &NSView,
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
        add_fill(parent, x, y, 15.0, 15.0, theme.ink, 1.0, mtm);
    } else {
        let box_ = add_fill(parent, x, y, 15.0, 15.0, theme.surface, 1.0, mtm);
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

/// Soft accent wash behind a row (design: rgba(149,96,74,.09)).
pub fn add_size_wash(
    parent: &NSView,
    x: f64,
    y: f64,
    max_w: f64,
    h: f64,
    fraction: f64,
    theme: &Theme,
    mtm: MainThreadMarker,
) {
    let w = (max_w * fraction.clamp(0.03, 1.0)).max(4.0);
    add_fill(parent, x, y, w, h, theme.accent, 0.09, mtm);
}
