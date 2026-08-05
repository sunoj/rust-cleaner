// Attributed-string styling for WD-40 menu rows: aligned columns and tinted runs.
// Exports: `Row`, `Columns`, font helpers, `symbol_image`, `text_width`, `fit_width`.
// Deps: objc2, objc2_app_kit, objc2_foundation.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::AnyThread;
use objc2_app_kit::{
    NSColor, NSFont, NSFontAttributeName, NSForegroundColorAttributeName, NSImage,
    NSMutableParagraphStyle, NSParagraphStyleAttributeName, NSStringDrawing, NSTextAlignment,
    NSTextTab,
};
use objc2_foundation::{
    NSArray, NSAttributedString, NSDictionary, NSMutableAttributedString, NSRange, NSSize, NSString,
};

/// Gap between the name column and the size column, in points.
const NAME_GAP: f64 = 18.0;
/// Width reserved for the right-aligned size column, in points.
const SIZE_WIDTH: f64 = 54.0;
/// Gap between the size column and the usage bar, in points.
const BAR_GAP: f64 = 12.0;
/// Narrowest name column, so a menu of short names still looks deliberate.
pub const MIN_NAME_WIDTH: f64 = 150.0;
/// Widest name column. Longer names are elided from the left; the full path
/// stays reachable as the row's tooltip.
pub const MAX_NAME_WIDTH: f64 = 320.0;

/// Tab stops shared by every row of one menu, so columns line up across groups.
#[derive(Clone, Copy)]
pub struct Columns {
    size_end: f64,
    bar_start: f64,
}

impl Columns {
    /// Columns sized to the widest name the menu will actually draw.
    pub fn for_name_width(name_width: f64) -> Self {
        let name = name_width.clamp(MIN_NAME_WIDTH, MAX_NAME_WIDTH);
        Self::ending_at(name + NAME_GAP + SIZE_WIDTH)
    }

    /// Columns whose right-aligned field ends `x` points from the row start.
    pub fn ending_at(x: f64) -> Self {
        Self { size_end: x, bar_start: x + BAR_GAP }
    }

    /// Where the trailing usage bar begins, in points from the row start.
    pub fn bar_start(&self) -> f64 {
        self.bar_start
    }
}

impl Default for Columns {
    fn default() -> Self {
        Self::for_name_width(MIN_NAME_WIDTH)
    }
}

pub fn menu_font() -> Retained<NSFont> {
    NSFont::menuFontOfSize(0.0)
}

/// Small secondary font used for group headers and footnotes.
pub fn caption_font() -> Retained<NSFont> {
    NSFont::menuFontOfSize(11.0)
}

/// Large font for the one number the menu is built around: free disk space.
pub fn headline_font() -> Retained<NSFont> {
    NSFont::boldSystemFontOfSize(16.0)
}

/// Font for block-glyph gauges — small enough that the blocks read as a bar.
pub fn gauge_font() -> Retained<NSFont> {
    NSFont::menuFontOfSize(9.0)
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

/// Rendered width of `text` in `font`, in points.
pub fn text_width(text: &str, font: &NSFont) -> f64 {
    let font_obj: &AnyObject = unsafe { &*(font as *const NSFont as *const AnyObject) };
    let attrs = NSDictionary::<NSString, AnyObject>::from_slices::<NSString>(
        &[unsafe { NSFontAttributeName }],
        &[font_obj],
    );
    let string = NSString::from_str(text);
    unsafe { string.sizeWithAttributes(Some(&attrs)) }.width
}

/// One styled span of a row. `None` keeps the row's own color or font.
struct Run {
    start: usize,
    len: usize,
    color: Option<Retained<NSColor>>,
    font: Option<Retained<NSFont>>,
}

/// A menu row assembled from styled runs, laid out against shared tab stops.
pub struct Row {
    text: String,
    runs: Vec<Run>,
    len: usize,
}

impl Row {
    pub fn new() -> Self {
        Self { text: String::new(), runs: Vec::new(), len: 0 }
    }

    /// Append `text`; `color` of `None` keeps the default label color.
    pub fn push(&mut self, text: &str, color: Option<Retained<NSColor>>) -> &mut Self {
        self.push_styled(text, color, None)
    }

    /// Append `text` in a font of its own — lets one row mix a headline number
    /// with a caption beside it.
    pub fn push_styled(
        &mut self,
        text: &str,
        color: Option<Retained<NSColor>>,
        font: Option<Retained<NSFont>>,
    ) -> &mut Self {
        let units = text.encode_utf16().count();
        if color.is_some() || font.is_some() {
            self.runs.push(Run { start: self.len, len: units, color, font });
        }
        self.text.push_str(text);
        self.len += units;
        self
    }

    /// Move to the next column.
    pub fn tab(&mut self) -> &mut Self {
        self.push("\t", None)
    }

    pub fn build(&self, font: &NSFont, columns: Columns) -> Retained<NSAttributedString> {
        let string = NSString::from_str(&self.text);
        let attributed =
            NSMutableAttributedString::initWithString(NSMutableAttributedString::alloc(), &string);
        let whole = NSRange::new(0, self.len);
        let paragraph = column_paragraph_style(columns);

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
            for run in &self.runs {
                let range = NSRange::new(run.start, run.len);
                if let Some(color) = &run.color {
                    attributed.addAttribute_value_range(
                        NSForegroundColorAttributeName,
                        &*(&**color as *const NSColor as *const AnyObject),
                        range,
                    );
                }
                if let Some(font) = &run.font {
                    attributed.addAttribute_value_range(
                        NSFontAttributeName,
                        &*(&**font as *const NSFont as *const AnyObject),
                        range,
                    );
                }
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

fn column_paragraph_style(columns: Columns) -> Retained<NSMutableParagraphStyle> {
    let paragraph = NSMutableParagraphStyle::new();
    let empty = NSDictionary::new();
    let size_tab = unsafe {
        NSTextTab::initWithTextAlignment_location_options(
            NSTextTab::alloc(),
            NSTextAlignment::Right,
            columns.size_end,
            &empty,
        )
    };
    let bar_tab = unsafe {
        NSTextTab::initWithTextAlignment_location_options(
            NSTextTab::alloc(),
            NSTextAlignment::Left,
            columns.bar_start,
            &empty,
        )
    };
    paragraph.setTabStops(Some(&NSArray::from_retained_slice(&[size_tab, bar_tab])));
    paragraph
}

/// Elide `name` from the left until it fits `max_width`, keeping the tail —
/// the end of a path is what tells two projects apart.
pub fn fit<F: Fn(&str) -> f64>(name: &str, max_width: f64, width_of: F) -> String {
    if width_of(name) <= max_width {
        return name.to_string();
    }
    let chars: Vec<char> = name.chars().collect();
    for skip in 1..chars.len() {
        let candidate: String =
            std::iter::once('\u{2026}').chain(chars[skip..].iter().copied()).collect();
        if width_of(&candidate) <= max_width {
            return candidate;
        }
    }
    "\u{2026}".to_string()
}

/// `fit` measured against a real font.
pub fn fit_width(name: &str, font: &NSFont, max_width: f64) -> String {
    fit(name, max_width, |text| text_width(text, font))
}

#[cfg(test)]
mod tests {
    use super::fit;

    /// Stand-in metric: every character is one unit wide.
    fn per_char(text: &str) -> f64 {
        text.chars().count() as f64
    }

    #[test]
    fn fit_keeps_names_that_already_fit() {
        assert_eq!(fit("smart-router", 20.0, per_char), "smart-router");
    }

    #[test]
    fn fit_elides_from_the_left() {
        assert_eq!(fit("aaaaabbbbbccccc", 6.0, per_char), "\u{2026}ccccc");
    }

    #[test]
    fn fit_degrades_to_an_ellipsis_when_nothing_fits() {
        assert_eq!(fit("abcdef", 0.5, per_char), "\u{2026}");
    }
}
