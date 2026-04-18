// Icon rendering for Rust Cleaner. Generates rusty SF Symbol icons based on disk usage.
// Exports: `rusty_icon`, `rust_text_color`.
// Deps: objc2, objc2_app_kit, objc2_foundation.

use objc2::{AnyThread, rc::Retained};
use objc2_app_kit::{NSBezierPath, NSColor, NSCompositingOperation, NSImage, NSImageSymbolConfiguration};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize, NSString};

const ICON_NORMAL: &str = "internaldrive";

#[allow(deprecated)]
pub fn rusty_icon(total_bytes: u64) -> Option<Retained<NSImage>> {
    let gb = total_bytes / (1024 * 1024 * 1024);
    let s = NSString::from_str(ICON_NORMAL);
    let base = NSImage::imageWithSystemSymbolName_accessibilityDescription(&s, None)?;

    if gb < 5 {
        base.setTemplate(true);
        return Some(base);
    }

    // Style B: two-tone palette colors based on rust severity
    let (c1, c2, spot_rgba, spot_count) = match gb {
        5..=19 => (
            (0.80, 0.50, 0.20), (0.90, 0.70, 0.40),  // light rust palette
            (0.75, 0.40, 0.12, 0.7), 10u32,
        ),
        20..=49 => (
            (0.70, 0.30, 0.10), (0.85, 0.50, 0.20),  // medium rust palette
            (0.60, 0.20, 0.06, 0.8), 20,
        ),
        _ => (
            (0.55, 0.15, 0.05), (0.75, 0.30, 0.10),  // heavy rust palette
            (0.45, 0.10, 0.03, 0.95), 35,
        ),
    };

    let color1 = NSColor::colorWithSRGBRed_green_blue_alpha(c1.0, c1.1, c1.2, 1.0);
    let color2 = NSColor::colorWithSRGBRed_green_blue_alpha(c2.0, c2.1, c2.2, 1.0);
    let palette = NSArray::from_retained_slice(&[color1, color2]);
    let config = NSImageSymbolConfiguration::configurationWithPaletteColors(&palette);
    let tinted = base.imageWithSymbolConfiguration(&config)?;
    tinted.setTemplate(false);

    // Draw icon + rust spots composited within icon alpha
    let size = tinted.size();
    let canvas = NSImage::initWithSize(NSImage::alloc(), size);
    canvas.lockFocus();

    // Draw base tinted icon
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), size);
    tinted.drawInRect_fromRect_operation_fraction(
        rect, NSRect::ZERO, NSCompositingOperation::SourceOver, 1.0,
    );

    // Style D: rust spots clipped to icon shape via sourceAtop
    let mut seed: u64 = 42;
    for _ in 0..spot_count {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fx = (seed >> 33) as f64 / (u32::MAX as f64);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fy = (seed >> 33) as f64 / (u32::MAX as f64);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fr = (seed >> 33) as f64 / (u32::MAX as f64);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let fj = (seed >> 33) as f64 / (u32::MAX as f64);

        let x = fx * size.width;
        let y = fy * size.height;
        let r = fr * 3.5 + 1.0;
        let jr = spot_rgba.0 + fj * 0.12;
        let jg = spot_rgba.1 + fj * 0.08;
        let jb = spot_rgba.2 + fj * 0.04;

        // Draw spot into a temp image, then composite sourceAtop
        let spot_img = NSImage::initWithSize(NSImage::alloc(), size);
        spot_img.lockFocus();
        let spot_color = NSColor::colorWithSRGBRed_green_blue_alpha(jr, jg, jb, spot_rgba.3);
        spot_color.setFill();
        let oval = NSRect::new(NSPoint::new(x - r, y - r), NSSize::new(r * 2.0, r * 2.0));
        NSBezierPath::bezierPathWithOvalInRect(oval).fill();
        spot_img.unlockFocus();

        spot_img.drawInRect_fromRect_operation_fraction(
            rect, NSRect::ZERO, NSCompositingOperation::SourceAtop, 1.0,
        );
    }

    canvas.unlockFocus();
    canvas.setTemplate(false);
    Some(canvas)
}

pub fn rust_text_color(total_bytes: u64) -> Retained<NSColor> {
    let gb = total_bytes / (1024 * 1024 * 1024);
    match gb {
        0..=4 => NSColor::labelColor(),
        5..=19 => NSColor::colorWithSRGBRed_green_blue_alpha(0.95, 0.70, 0.40, 1.0),
        20..=49 => NSColor::colorWithSRGBRed_green_blue_alpha(0.90, 0.50, 0.25, 1.0),
        _ => NSColor::colorWithSRGBRed_green_blue_alpha(0.80, 0.35, 0.15, 1.0),
    }
}
