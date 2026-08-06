// Our can, drawn from the app icon at the size the plate can carry: blue glass
// body under a yellow label band, silver crimps and a tapered shoulder, red
// head and straw. The icon's glass is flattened against its own panel, so the
// can reads the same on a light plate as on a dark one. Exports: `draw_nozzle`.
// Deps: crate::metal, objc2 AppKit. Drawing calls belong in `drawRect:`.

use crate::metal::{circle_path, grey};
use objc2::rc::Retained;
use objc2_app_kit::{NSBezierPath, NSColor};
use objc2_foundation::NSPoint;

// The can's own frame, in points: x runs right from the pivot, y runs up from
// the head — the end that is pointed at the plate. Every figure is the icon's
// geometry at 24/96 scale, turned over so the head leads and the base is on
// top, which is how you hold a can at something below you.
const BODY_X: (f64, f64) = (0.0, 24.0);
const BODY_Y: (f64, f64) = (14.5, 52.5);
/// Crimped rim where the body meets the shoulder, and the base at the far end.
const RIM_TOP_Y: (f64, f64) = (14.5, 16.25);
const RIM_BASE_Y: (f64, f64) = (49.75, 52.5);
/// The icon's 26px corner radius on the base, as much of it as 24pt can carry.
const BASE_CHAMFER: (f64, f64) = (3.0, 21.0);
const BAND_Y: (f64, f64) = (29.0, 40.5);
const SHOULDER_Y: (f64, f64) = (10.0, 18.0);
/// The shoulder's narrow end — the icon clips it to 29%–71% at the neck.
const SHOULDER_NECK: (f64, f64) = (7.2, 16.8);
const NECK_X: (f64, f64) = (6.75, 17.25);
const NECK_Y: (f64, f64) = (7.5, 11.0);
const HEAD_X: (f64, f64) = (-2.0, 19.0);
const HEAD_Y: (f64, f64) = (0.0, 8.5);
const STRAW_X: (f64, f64) = (0.0, 2.25);
const STRAW_Y: (f64, f64) = (6.5, 30.5);
/// The free end of the straw, out of the head and pointed at the plate.
const TIP_X: (f64, f64) = (5.5, 8.0);
const TIP_Y: (f64, f64) = (-9.0, 0.0);

/// Every palette below is the icon's own gradient, composited over the panel it
/// sits on (#E0DCD5) so it can be drawn as opaque paint. Channels are kept as
/// 0-255 so they can be checked against the design without decoding a float.
type Stops<'a> = &'a [(f64, [f64; 3])];

/// Blue body: the icon's 105° glass gradient under the 90° shading that wraps
/// the light down the left and darkens both edges.
const BODY: [(f64, [f64; 3]); 5] = [
    (0.0, [59.0, 78.0, 125.0]),
    (0.16, [153.0, 171.0, 216.0]),
    (0.42, [86.0, 108.0, 175.0]),
    (0.72, [78.0, 101.0, 161.0]),
    (1.0, [57.0, 75.0, 118.0]),
];
/// Label band, carrying the same wrap so it lights with the body.
const BAND: [(f64, [f64; 3]); 5] = [
    (0.0, [158.0, 120.0, 29.0]),
    (0.17, [243.0, 224.0, 167.0]),
    (0.44, [227.0, 190.0, 79.0]),
    (0.74, [198.0, 158.0, 45.0]),
    (1.0, [144.0, 109.0, 26.0]),
];
const RIM_TOP: [(f64, [f64; 3]); 4] = [
    (0.0, [130.0, 141.0, 158.0]),
    (0.22, [245.0, 248.0, 250.0]),
    (0.55, [193.0, 202.0, 214.0]),
    (1.0, [106.0, 117.0, 135.0]),
];
const RIM_BASE: [(f64, [f64; 3]); 4] = [
    (0.0, [95.0, 106.0, 124.0]),
    (0.2, [228.0, 233.0, 239.0]),
    (0.56, [168.0, 178.0, 193.0]),
    (1.0, [79.0, 90.0, 109.0]),
];
const HEAD: [(f64, [f64; 3]); 4] = [
    (0.0, [133.0, 26.0, 24.0]),
    (0.2, [245.0, 114.0, 93.0]),
    (0.54, [215.0, 60.0, 47.0]),
    (1.0, [125.0, 24.0, 22.0]),
];
const STRAW: [(f64, [f64; 3]); 3] =
    [(0.0, [110.0, 21.0, 20.0]), (0.38, [236.0, 86.0, 66.0]), (1.0, [161.0, 32.0, 28.0])];
/// The shoulder's gradient runs along the can, not across it, so it is sampled
/// per band rather than per strip.
const SHOULDER: [(f64, [f64; 3]); 4] = [
    (0.0, [254.0, 254.0, 254.0]),
    (0.2, [234.0, 239.0, 246.0]),
    (0.56, [194.0, 204.0, 218.0]),
    (1.0, [136.0, 149.0, 169.0]),
];

/// Cone then can, both hung off the pointer exactly as the mock hangs them: the
/// can sits up and to the right, which also keeps it clear of the arrow cursor.
pub fn draw_nozzle(point: NSPoint, cone: bool) {
    if cone {
        for step in 0..12 {
            grey(1.0, 0.1).setFill();
            circle_path(point, 26.0 * (1.0 - step as f64 / 12.0)).fill();
        }
    }
    let angle = -24.0_f64.to_radians();
    let pivot = NSPoint::new(point.x + 6.0, point.y + 14.0);
    shoulder(pivot, angle);
    sweep(pivot, angle, NECK_Y, NECK_X, NECK_X, &RIM_TOP);
    sweep(pivot, angle, BODY_Y, BODY_X, BODY_X, &BODY);
    sweep(pivot, angle, BAND_Y, BODY_X, BODY_X, &BAND);
    sweep(pivot, angle, RIM_TOP_Y, BODY_X, BODY_X, &RIM_TOP);
    sweep(pivot, angle, RIM_BASE_Y, BODY_X, BASE_CHAMFER, &RIM_BASE);
    sweep(pivot, angle, STRAW_Y, STRAW_X, STRAW_X, &STRAW);
    sweep(pivot, angle, TIP_Y, TIP_X, TIP_X, &STRAW);
    sweep(pivot, angle, HEAD_Y, HEAD_X, HEAD_X, &HEAD);
}

/// The tapered shoulder between neck and body, in bands along the can because
/// that is the way its gradient runs.
fn shoulder(pivot: NSPoint, angle: f64) {
    const BANDS: usize = 3;
    let span = |t: f64| {
        (
            lerp(SHOULDER_NECK.0, BODY_X.0 + 0.5, t),
            lerp(SHOULDER_NECK.1, BODY_X.1 - 0.5, t),
        )
    };
    for band in 0..BANDS {
        let (a, b) = (band as f64 / BANDS as f64, (band + 1) as f64 / BANDS as f64);
        let tone = mix(&SHOULDER, (a + b) / 2.0);
        let y = (lerp(SHOULDER_Y.0, SHOULDER_Y.1, a), lerp(SHOULDER_Y.0, SHOULDER_Y.1, b));
        sweep(pivot, angle, y, span(a), span(b), &[(0.0, tone)]);
    }
}

/// One part of the can: a gradient across it, drawn as vertical strips, with an
/// x span at each end so the same call covers a straight side, the tapered
/// shoulder or the chamfered base. Narrow parts take fewer strips.
fn sweep(pivot: NSPoint, angle: f64, y: (f64, f64), lower: (f64, f64), upper: (f64, f64), stops: Stops) {
    let strips = ((lower.1 - lower.0) / 2.0).ceil().clamp(2.0, 12.0) as usize;
    for strip in 0..strips {
        let (a, b) = (strip as f64 / strips as f64, (strip + 1) as f64 / strips as f64);
        ink(mix(stops, (a + b) / 2.0)).setFill();
        // Each strip runs half a strip into the one after it, so the seam is
        // painted over instead of showing as an antialiased hairline.
        let b = (b + 0.5 / strips as f64).min(1.0);
        poly(
            pivot,
            angle,
            &[
                (lerp(lower.0, lower.1, a), y.0),
                (lerp(lower.0, lower.1, b), y.0),
                (lerp(upper.0, upper.1, b), y.1),
                (lerp(upper.0, upper.1, a), y.1),
            ],
        );
    }
}

/// The tone at `t` along a flattened gradient.
fn mix(stops: Stops, t: f64) -> [f64; 3] {
    let mut out = stops[0].1;
    for pair in stops.windows(2) {
        let ((p0, c0), (p1, c1)) = (pair[0], pair[1]);
        if t >= p0 && p1 > p0 {
            let f = ((t - p0) / (p1 - p0)).clamp(0.0, 1.0);
            out = [0, 1, 2].map(|channel| c0[channel] + (c1[channel] - c0[channel]) * f);
        }
    }
    out
}

fn lerp(from: f64, to: f64, t: f64) -> f64 {
    from + (to - from) * t
}

fn ink(tone: [f64; 3]) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(tone[0] / 255.0, tone[1] / 255.0, tone[2] / 255.0, 1.0)
}

/// Points in the can's own frame, rotated about the pivot and filled.
fn poly(pivot: NSPoint, angle: f64, points: &[(f64, f64)]) {
    let (sin, cos) = angle.sin_cos();
    let path = NSBezierPath::new();
    for (index, (x, y)) in points.iter().enumerate() {
        let point = NSPoint::new(pivot.x + x * cos - y * sin, pivot.y + x * sin + y * cos);
        match index {
            0 => path.moveToPoint(point),
            _ => path.lineToPoint(point),
        }
    }
    path.closePath();
    path.fill();
}
