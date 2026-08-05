// Frame-based AppKit primitives for the 380pt popover (labels, fills, lines).
// Exports: layout helpers; interactive controls live in `controls`.
// Deps: objc2, objc2_app_kit, objc2_foundation, crate::theme::Theme.

use crate::theme::Theme;
use objc2::rc::Retained;
use objc2::MainThreadOnly;
use objc2_app_kit::{
    NSBorderType, NSBox, NSBoxType, NSFont, NSImage, NSImageSymbolConfiguration,
    NSImageSymbolScale, NSImageView, NSLineBreakMode, NSScrollView, NSTextAlignment, NSTextField,
    NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};

pub const POPOVER_WIDTH: f64 = 380.0;
pub const PAD_X: f64 = 16.0;
pub const CONTENT_WIDTH: f64 = POPOVER_WIDTH - PAD_X * 2.0;

/// The panel is an opaque surface, as the mock draws it. NSVisualEffectView was
/// tried and dropped: behind-window vibrancy let the desktop through and made
/// the design's flat mid-greys illegible, and the material had not settled the
/// first time the popover was shown, so it looked different before and after
/// the first click. A plain fill renders the same every time.
pub fn root_view(height: f64, fill: (f64, f64, f64), mtm: MainThreadMarker) -> Retained<NSView> {
    let view = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(POPOVER_WIDTH, height)),
    );
    add_fill(&view, 0.0, 0.0, POPOVER_WIDTH, height, fill, 1.0, 0.0, mtm);
    view
}

#[allow(clippy::too_many_arguments)]
pub fn scroll_document_view(
    parent: &NSView,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    document_h: f64,
    mtm: MainThreadMarker,
) -> Retained<NSView> {
    let scroll = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)),
    );
    scroll.setBorderType(NSBorderType::NoBorder);
    scroll.setHasHorizontalScroller(false);
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setDrawsBackground(false);
    let document = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, document_h.max(h))),
    );
    scroll.setDocumentView(Some(&document));
    parent.addSubview(&scroll);
    document
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
    radius: f64,
    mtm: MainThreadMarker,
) -> Retained<NSBox> {
    let box_ = NSBox::initWithFrame(
        NSBox::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(w.max(0.5), h.max(0.5))),
    );
    box_.setBoxType(NSBoxType::Custom);
    box_.setBorderWidth(0.0);
    box_.setFillColor(&Theme::color_alpha(rgb, alpha));
    if radius > 0.0 {
        box_.setCornerRadius(radius);
    }
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
    add_fill(parent, x, y, w, 1.0, color, 1.0, 0.0, mtm);
}

#[allow(deprecated)]
#[allow(clippy::too_many_arguments)]
pub fn symbol_view(
    parent: &NSView,
    symbol: &str,
    x: f64,
    y: f64,
    size: f64,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) {
    let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        &NSString::from_str(symbol),
        None,
    ) else {
        return;
    };
    image.setTemplate(true);
    let view = NSImageView::imageViewWithImage(&image, mtm);
    view.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(size, size)));
    view.setContentTintColor(Some(&Theme::color(color)));
    view.setSymbolConfiguration(Some(&NSImageSymbolConfiguration::configurationWithScale(
        NSImageSymbolScale::Medium,
    )));
    parent.addSubview(&view);
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
    make_label(parent, text, x, y, w, h, size, bold, color, mono, false, mtm)
}

/// Multiline label that wraps at word boundaries (footer copy, notes).
#[allow(clippy::too_many_arguments)]
pub fn label_wrap(
    parent: &NSView,
    text: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    size: f64,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    make_label(parent, text, x, y, w, h, size, false, color, false, true, mtm)
}

#[allow(clippy::too_many_arguments)]
fn make_label(
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
    wrap: bool,
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
    if wrap {
        field.setUsesSingleLineMode(false);
        field.setMaximumNumberOfLines(0);
        field.setLineBreakMode(NSLineBreakMode::ByWordWrapping);
        field.setPreferredMaxLayoutWidth(w);
        if let Some(cell) = field.cell() {
            cell.setWraps(true);
        }
    }
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

/// Soft accent wash behind a row (design: rgba(149,96,74,.09)) — size bar.
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
    add_fill(parent, x, y, w, h, theme.accent, 0.14, 0.0, mtm);
}
