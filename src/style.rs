// Attributed-string styling for WD-40 menu rows: aligned columns and tinted runs.
// Exports: `Row`, `menu_font`, `symbol_image`, `SIZE_TAB`.
// Deps: objc2, objc2_app_kit, objc2_foundation.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_app_kit::{
    NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSImage,
    NSMutableParagraphStyle, NSParagraphStyleAttributeName, NSTextAlignment, NSTextTab,
};
use objc2_foundation::{
    NSArray, NSAttributedString, NSDictionary, NSMutableAttributedString, NSRange, NSSize, NSString,
};

/// Right-aligned tab stop where the size column ends, in points.
const SIZE_TAB: f64 = 276.0;
/// Left-aligned tab stop where the usage bar starts, in points.
const BAR_TAB: f64 = 288.0;

pub fn menu_font() -> Retained<NSFont> {
    NSFont::menuFontOfSize(0.0)
}

/// Small secondary font used for group headers and footnotes.
pub fn caption_font() -> Retained<NSFont> {
    NSFont::menuFontOfSize(11.0)
}

pub fn symbol_image(name: &str, points: f64) -> Option<Retained<NSImage>> {
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str(name),
        None,
    )?;
    image.setSize(NSSize::new(points, points));
    image.setTemplate(true);
    Some(image)
}

/// A menu row assembled from colored runs, laid out against shared tab stops.
pub struct Row {
    text: String,
    runs: Vec<(usize, usize, Retained<NSColor>)>,
    len: usize,
}

impl Row {
    pub fn new() -> Self {
        Self { text: String::new(), runs: Vec::new(), len: 0 }
    }

    /// Append `text`; `color` of `None` keeps the default label color.
    pub fn push(&mut self, text: &str, color: Option<Retained<NSColor>>) -> &mut Self {
        let units = text.encode_utf16().count();
        if let Some(color) = color {
            self.runs.push((self.len, units, color));
        }
        self.text.push_str(text);
        self.len += units;
        self
    }

    /// Move to the size column (right-aligned).
    pub fn tab(&mut self) -> &mut Self {
        self.push("\t", None)
    }

    pub fn build(&self, font: &NSFont) -> Retained<NSAttributedString> {
        let string = NSString::from_str(&self.text);
        let attributed =
            NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &string);
        let whole = NSRange::new(0, self.len);
        let paragraph = column_paragraph_style();

        unsafe {
            attributed.addAttribute_value_range(
                NSFontAttributeName,
                &*(font as *const NSFont as *const AnyObject),
                whole,
            );
            attributed.addAttribute_value_range(
                NSParagraphStyleAttributeName,
                &*(&*paragraph as *const NSMutableParagraphStyle as *const AnyObject),
                whole,
            );
            for (start, units, color) in &self.runs {
                attributed.addAttribute_value_range(
                    NSForegroundColorAttributeName,
                    &*(&**color as *const NSColor as *const AnyObject),
                    NSRange::new(*start, *units),
                );
            }
        }

        Retained::into_super(attributed)
    }
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

fn column_paragraph_style() -> Retained<NSMutableParagraphStyle> {
    let paragraph = NSMutableParagraphStyle::new();
    let empty = NSDictionary::new();
    let size_tab = unsafe {
        NSTextTab::initWithTextAlignment_location_options(
            NSTextTab::alloc(),
            NSTextAlignment::Right,
            SIZE_TAB,
            &empty,
        )
    };
    let bar_tab = unsafe {
        NSTextTab::initWithTextAlignment_location_options(
            NSTextTab::alloc(),
            NSTextAlignment::Left,
            BAR_TAB,
            &empty,
        )
    };
    paragraph.setTabStops(Some(&NSArray::from_retained_slice(&[size_tab, bar_tab])));
    paragraph
}

/// Trim a project name so rows keep a predictable width.
pub fn truncate(name: &str, max_chars: usize) -> String {
    let count = name.chars().count();
    if count <= max_chars {
        return name.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let tail: String = name.chars().skip(count - keep).collect();
    format!("\u{2026}{tail}")
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn truncate_keeps_short_names() {
        assert_eq!(truncate("smart-router", 20), "smart-router");
    }

    #[test]
    fn truncate_elides_from_the_left() {
        assert_eq!(truncate("aaaaabbbbbccccc", 6), "\u{2026}ccccc");
    }
}
