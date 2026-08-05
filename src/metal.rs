// Brushed-metal drawing primitives shared by the cleaning plate and the done
// medal: the 135° band sweep, the concentric turning marks, and small helpers.
// Exports: `brushed`, `rings`, `circle_path`, `grey`, `rnd`, `outline`, `inset`.
// Deps: objc2_app_kit, objc2_foundation. Call these inside `drawRect:` only.

use objc2::rc::Retained;
use objc2_app_kit::{NSBezierPath, NSColor};
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// The mock's 135° brushed steel. The bands run corner to corner and each one
/// sits a hair off its neighbour, so the surface reads as brushed, not printed.
pub fn brushed(rect: NSRect, dark: bool) {
    let height = rect.size.height;
    let span = rect.size.width + height;
    let bands = (span / 4.0).ceil().max(1.0) as i32;
    let mut seed = 0x51ed_2701_u32;
    for i in 0..bands {
        let t = i as f64 / bands as f64;
        let x = rect.origin.x - height + t * span;
        let base = rect.origin.y;
        let path = NSBezierPath::new();
        path.moveToPoint(NSPoint::new(x, base));
        path.lineToPoint(NSPoint::new(x + 5.0, base));
        path.lineToPoint(NSPoint::new(x + 5.0 + height, base + height));
        path.lineToPoint(NSPoint::new(x + height, base + height));
        path.closePath();
        let shade = 0.865 + 0.09 * (t * 7.6).sin() + (rnd(&mut seed) - 0.5) * 0.035;
        grey(if dark { shade * 0.46 } else { shade }, 1.0).setFill();
        path.fill();
    }
}

/// Concentric turning marks — the mock's repeating radial gradient and its cap.
pub fn rings(centre: NSPoint, radius: f64, dark: bool) {
    let light = grey(if dark { 0.78 } else { 1.0 }, 0.3);
    let shadow = grey(0.11, 0.055);
    let mut r = radius;
    let mut step = 0;
    while r > 1.5 {
        if step % 2 == 0 { light.setStroke() } else { shadow.setStroke() }
        let circle = circle_path(centre, r);
        circle.setLineWidth(1.0);
        circle.stroke();
        r -= 2.0;
        step += 1;
    }
    let cap = radius * 0.2;
    grey(if dark { 0.62 } else { 0.9 }, 1.0).setFill();
    circle_path(centre, cap).fill();
    grey(if dark { 0.34 } else { 0.56 }, 1.0).setFill();
    circle_path(centre, cap * 0.3).fill();
}

pub fn circle_path(centre: NSPoint, radius: f64) -> Retained<NSBezierPath> {
    NSBezierPath::bezierPathWithOvalInRect(NSRect::new(
        NSPoint::new(centre.x - radius, centre.y - radius),
        NSSize::new(radius * 2.0, radius * 2.0),
    ))
}

pub fn grey(value: f64, alpha: f64) -> Retained<NSColor> {
    let v = value.clamp(0.0, 1.0);
    NSColor::colorWithSRGBRed_green_blue_alpha(v * 0.985, v * 0.995, v, alpha)
}

/// Deterministic noise: `drawRect:` runs many times over, and fresh random
/// numbers on each pass would make the surface crawl.
pub fn rnd(seed: &mut u32) -> f64 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 8) as f64 / 16_777_216.0
}

pub fn outline(rect: NSRect, width: f64) {
    let path = NSBezierPath::bezierPathWithRect(rect);
    path.setLineWidth(width);
    path.stroke();
}

pub fn inset(rect: NSRect, by: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(rect.origin.x + by, rect.origin.y + by),
        NSSize::new((rect.size.width - by * 2.0).max(0.0), (rect.size.height - by * 2.0).max(0.0)),
    )
}
