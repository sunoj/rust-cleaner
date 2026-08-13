// How the can is held: where its own frame sits, which way it leans, and the
// turn that keeps the whole of it inside the plate and off the name it must not
// cover. Nothing here draws — it only works out where a rigid thing of a given
// size may sit around the point it is aimed at.
// Exports: `Grip`, `Held`, `Span`, `hold`, `turns`, `fits`.
// Deps: crate::{metal, treemap}, objc2 Foundation geometry.

use crate::metal::inset;
use crate::treemap::contains;
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// How far the can leans back from the line it sprays along.
const LEAN: f64 = -24.0;
/// Where the held thing's own origin sits from the point it is aimed at.
const PIVOT: (f64, f64) = (6.0, 14.0);
/// Room kept between the can and the plate's boundary, which is also where the
/// plate's rounded corners cut in.
const CLEARANCE: f64 = 3.0;

/// The box a held thing covers in its own frame: the x span, then the y span.
pub type Span = ((f64, f64), (f64, f64));

/// Where the held thing's frame sits, and which way it leans.
pub struct Grip {
    origin: NSPoint,
    sin: f64,
    cos: f64,
}

/// Where the can settled: how far it had to turn, and which of the boxes it was
/// asked to keep off it managed to clear.
pub struct Held {
    pub turn: f64,
    pub cleared: Option<usize>,
}

/// Everything drawn on the plate is clipped to it, and the can reaches some
/// 60pt up and to the right of what it is aimed at, so against an edge the body
/// would be sliced off. What gives way is the grip: the whole can turns about
/// the point it is aimed at until it is inside the plate again. The tip stays
/// exactly where it was aimed, so the nozzle keeps every inch of the plate and
/// the can arrives whole.
///
/// `keep_off` is where the name of the tile being worked could go, in the order
/// the name would rather have them. The can is aimed at that same tile, so the
/// two would otherwise be drawn over each other every time; the can is what
/// gives way, being the decoration of the pair, and it takes the first place it
/// can leave the name. Against the very side of the plate there is room beside
/// a tile for one of them but not both — then the can takes what the plate
/// allows, and the name, drawn after it, stays readable over the top.
pub fn hold(at: NSPoint, plate: NSRect, keep_off: &[NSRect], span: Span) -> Held {
    for (choice, taken) in keep_off.iter().enumerate() {
        if let Some(turn) = turns().find(|turn| fits(at, *turn, plate, Some(*taken), span)) {
            return Held { turn, cleared: Some(choice) };
        }
    }
    Held {
        turn: turns().find(|turn| fits(at, *turn, plate, None, span)).unwrap_or(0.0),
        cleared: None,
    }
}

impl Grip {
    pub fn turned(at: NSPoint, turn: f64) -> Self {
        let (spin, lean) = (turn.to_radians().sin_cos(), (turn + LEAN).to_radians().sin_cos());
        Self {
            origin: NSPoint::new(
                at.x + PIVOT.0 * spin.1 - PIVOT.1 * spin.0,
                at.y + PIVOT.0 * spin.0 + PIVOT.1 * spin.1,
            ),
            sin: lean.0,
            cos: lean.1,
        }
    }

    /// A point of the held thing's own drawing, in the plate's coordinates.
    pub fn map(&self, x: f64, y: f64) -> NSPoint {
        NSPoint::new(
            self.origin.x + x * self.cos - y * self.sin,
            self.origin.y + x * self.sin + y * self.cos,
        )
    }
}

/// Turns to try: none at all first, then further round either way in small
/// steps, so the least that will do is always what is picked. A can with the
/// room it wants never moves, and one that has to give way does it by degrees
/// as the nozzle travels rather than in one jump.
pub fn turns() -> impl Iterator<Item = f64> {
    let step = |n: i32| f64::from(n) * 3.0;
    std::iter::once(0.0).chain((1..=60).flat_map(move |n| [-step(n), step(n)]))
}

/// Whether `span` lands inside the plate at this turn and clear of `keep_off`.
/// It is measured through the same mapping the parts are drawn with, so the
/// two cannot drift out of step.
pub fn fits(at: NSPoint, turn: f64, plate: NSRect, keep_off: Option<NSRect>, span: Span) -> bool {
    let held = Grip::turned(at, turn);
    let ((x0, x1), (y0, y1)) = span;
    let corners = [held.map(x0, y0), held.map(x1, y0), held.map(x0, y1), held.map(x1, y1)];
    if !corners.iter().all(|corner| contains(inset(plate, CLEARANCE), *corner)) {
        return false;
    }
    // Against the box it must keep off, the can is taken as its upright
    // bounding box: more than it really covers, which only ever makes it give
    // the name a wider berth.
    keep_off.is_none_or(|taken| apart(box_of(&corners), taken))
}

/// The upright box a set of corners sits in.
fn box_of(corners: &[NSPoint; 4]) -> NSRect {
    let (mut lo, mut hi) = (corners[0], corners[0]);
    for corner in corners {
        lo = NSPoint::new(lo.x.min(corner.x), lo.y.min(corner.y));
        hi = NSPoint::new(hi.x.max(corner.x), hi.y.max(corner.y));
    }
    NSRect::new(lo, NSSize::new(hi.x - lo.x, hi.y - lo.y))
}

fn apart(one: NSRect, two: NSRect) -> bool {
    one.origin.x >= two.origin.x + two.size.width
        || two.origin.x >= one.origin.x + one.size.width
        || one.origin.y >= two.origin.y + two.size.height
        || two.origin.y >= one.origin.y + one.size.height
}
