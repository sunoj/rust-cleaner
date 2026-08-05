// Attributed-string helper for the status-item title tint.
// Exports: `tinted`. Kept small after the menu UI was replaced by the popover.
// Deps: objc2, objc2_app_kit, objc2_foundation.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName};
use objc2_foundation::{NSAttributedString, NSDictionary, NSString};

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
