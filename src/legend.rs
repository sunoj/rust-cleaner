// What the plate says: how a tile's state reads, the line under the plate, and
// the name set against the tile being removed right now — the same tile the
// nozzle is working, so the two never disagree. The name is placed before the
// can is and drawn after it, so nothing can cover it.
// Exports: `phase`, `legend_text`, `idle_text`, `Name`, `worked_name`.
// Deps: crate::{drift, header, metal, state, style, treemap}, objc2 AppKit.

use crate::drift::largest;
use crate::header::scan_size;
use crate::metal::grey;
use crate::state::CleanProgress;
use crate::style::attributed;
use crate::treemap::{Tile, ACTIVE, BLOCKED, DONE, PART_GONE, SKIPPED};
use objc2::rc::Retained;
use objc2_app_kit::{NSAttributedStringNSStringDrawing, NSBezierPath};
use objc2_foundation::{NSAttributedString, NSPoint, NSRect, NSSize};

/// Clearance between the working tile and the label naming it, and between
/// that label and the edge of the plate.
const GAP: f64 = 9.0;
const EDGE: f64 = 4.0;
/// Past this a name would run off the plate whatever else was done with it.
const MAX_CHARS: usize = 30;

/// How a tile's state reads in the legend.
pub fn phase(state: u8) -> &'static str {
    match state {
        DONE => "removed",
        ACTIVE => "removing",
        SKIPPED => "left in place",
        PART_GONE => "partly removed \u{2014} the rest is still there",
        BLOCKED => "could not be removed",
        _ => "waiting",
    }
}

/// What the line under the plate says: the tile the pointer is on, or the run's
/// own figures when it is on none of them.
pub fn legend_text(tiles: &[Tile], hover: isize, idle: &str) -> String {
    match usize::try_from(hover).ok().and_then(|index| tiles.get(index)) {
        Some(tile) => {
            format!("{} \u{00b7} {} \u{00b7} {}", tile.name, scan_size(tile.size), phase(tile.state))
        }
        None => idle.to_string(),
    }
}

/// The same line with nothing under the pointer: the share of the volume this
/// job is, and how much of it has really gone.
pub fn idle_text(share: &str, progress: &CleanProgress) -> String {
    format!("{share} \u{00b7} {}/{} clear", progress.done_count, progress.total_count)
}

/// The name of what is coming off right now, placed against the patch it is
/// coming off. The tile is the one the nozzle is already working, and there is
/// none once they have all settled.
pub struct Name {
    text: Retained<NSAttributedString>,
    tile: NSRect,
    /// Where it could go, in the order it would rather have them.
    spots: [NSPoint; 2],
    taken: usize,
    size: NSSize,
    dark: bool,
}

/// Work out where the name goes without drawing it, so the can can be told
/// which piece of the plate it may not have.
pub fn worked_name(plate: NSRect, tiles: &[Tile], dark: bool) -> Option<Name> {
    let tile = largest(tiles, ACTIVE)?;
    let ink = if dark { (0.94, 0.93, 0.91) } else { (0.13, 0.12, 0.11) };
    let text = attributed(&clip(&tile.name, MAX_CHARS), ink, 11.0, true);
    let size = NSSize::new(text.size().width + 12.0, text.size().height + 6.0);
    let spots = label_spots(plate, tile.rect, size);
    Some(Name { text, tile: tile.rect, spots, taken: 0, size, dark })
}

impl Name {
    /// The patches of plate the name could claim, for the can to pick from.
    pub fn choices(&self) -> [NSRect; 2] {
        self.spots.map(|at| NSRect::new(at, self.size))
    }

    /// Take the one the can agreed to leave.
    pub fn take(&mut self, choice: usize) {
        self.taken = choice.min(self.spots.len() - 1);
    }

    /// Drawn after everything that could cover it, so the name always reads.
    pub fn draw(&self) {
        let at = self.spots[self.taken];
        leader(self.tile, at, self.size, self.dark);
        grey(if self.dark { 0.14 } else { 0.99 }, 0.9).setFill();
        let pill = NSRect::new(at, self.size);
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(pill, 5.0, 5.0).fill();
        self.text.drawAtPoint(NSPoint::new(at.x + 6.0, at.y + 3.0));
    }
}

/// Beside the tile on the bare steel, the side with room for it first, and
/// neither of them off the plate. The can is given both and takes the one it
/// can keep off.
pub fn label_spots(plate: NSRect, tile: NSRect, size: NSSize) -> [NSPoint; 2] {
    let floor = plate.origin.y + EDGE;
    let y = (tile.origin.y + tile.size.height / 2.0 - size.height / 2.0)
        .clamp(floor, (plate.origin.y + plate.size.height - size.height - EDGE).max(floor));
    let (left, right) = (tile.origin.x - GAP - size.width, tile.origin.x + tile.size.width + GAP);
    let far = plate.origin.x + plate.size.width - size.width - EDGE;
    let held = |x: f64| NSPoint::new(x.clamp(plate.origin.x + EDGE, far.max(plate.origin.x + EDGE)), y);
    match right <= far {
        true => [NSPoint::new(right, y), held(left)],
        false => [held(left), held(right)],
    }
}

/// A hairline from the tile's near edge to the label, so a label that had to
/// move still says which patch it belongs to.
fn leader(tile: NSRect, at: NSPoint, size: NSSize, dark: bool) {
    // A crust with no bare steel beside it leaves the label on the patch, and
    // a label already on its patch needs no line drawn to it.
    if at.x < tile.origin.x + tile.size.width && at.x + size.width > tile.origin.x {
        return;
    }
    let from = match at.x > tile.origin.x {
        true => tile.origin.x + tile.size.width,
        false => tile.origin.x,
    };
    let mid = at.y + size.height / 2.0;
    let path = NSBezierPath::new();
    path.moveToPoint(NSPoint::new(from, mid));
    path.lineToPoint(NSPoint::new(if at.x > from { at.x } else { at.x + size.width }, mid));
    grey(if dark { 0.8 } else { 0.3 }, 0.55).setStroke();
    path.stroke();
}

/// Names are short by the time they reach here, but a deep one still has to
/// stop before it runs off the plate.
fn clip(name: &str, max_chars: usize) -> String {
    match name.chars().count() > max_chars {
        true => format!("{}\u{2026}", name.chars().take(max_chars - 1).collect::<String>()),
        false => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{clip, label_spots};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
    }

    #[test]
    fn the_working_tile_is_named_beside_it_and_never_off_the_plate() {
        let plate = rect(0.0, 0.0, 348.0, 190.0);
        let label = NSSize::new(90.0, 18.0);
        // Room to the right of the patch: the label goes there.
        let at = label_spots(plate, rect(0.0, 150.0, 40.0, 40.0), label)[0];
        assert!(at.x > 40.0 && at.x + label.width < 348.0, "{at:?}");
        assert!(at.y >= 0.0 && at.y + label.height <= 190.0, "{at:?}");
        // A patch against the right edge pushes the label back to its left.
        let at = label_spots(plate, rect(300.0, 20.0, 48.0, 40.0), label)[0];
        assert!(at.x + label.width <= 300.0, "{at:?}");
        // A crust that fills the plate leaves nowhere outside it to go.
        let at = label_spots(plate, plate, label)[0];
        assert!(at.x >= 0.0 && at.x + label.width <= 348.0, "{at:?}");
        assert!(at.y >= 0.0 && at.y + label.height <= 190.0, "{at:?}");
    }

    #[test]
    fn a_long_name_stops_before_it_runs_off_the_plate() {
        assert_eq!(clip("wd-40 \u{00b7} target", 30), "wd-40 \u{00b7} target");
        assert_eq!(clip("abcdefghij", 5), "abcd\u{2026}");
    }
}
