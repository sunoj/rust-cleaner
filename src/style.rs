// Attributed-string helpers: the status-item title tint, and the button titles
// AppKit will not colour for us.
// Exports: `tinted`, `attributed`.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::theme.

use crate::theme::Theme;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName};
use objc2_foundation::{NSAttributedString, NSDictionary, NSString};

/// Button title in one colour, weight and size.
pub fn attributed(
    text: &str,
    rgb: (f64, f64, f64),
    size: f64,
    bold: bool,
) -> Retained<NSAttributedString> {
    let font = if bold {
        NSFont::boldSystemFontOfSize(size)
    } else {
        NSFont::systemFontOfSize(size)
    };
    tinted(text, &font, &Theme::color(rgb))
}

/// Simple single-attribute string for the status bar button title.
pub fn tinted(text: &str, font: &NSFont, color: &NSColor) -> Retained<NSAttributedString> {
    let font_obj: &AnyObject = unsafe { &*(font as *const NSFont as *const AnyObject) };
    let color_obj: &AnyObject = unsafe { &*(color as *const NSColor as *const AnyObject) };
    let attrs = NSDictionary::<NSString, AnyObject>::from_slices::<NSString>(
        &[unsafe { NSForegroundColorAttributeName }, unsafe { NSFontAttributeName }],
        &[color_obj, font_obj],
    );
    unsafe { NSAttributedString::new_with_attributes(&NSString::from_str(text), &attrs) }
}
