// The WD-40 spray: the can that follows the pointer, the cone under it, the
// mist it throws off, and the record of what it has wiped. Spraying lifts the
// residue left where a target has really gone; live crust it cannot touch.
// Exports: `Mist`, `draw_nozzle`, `wipe`, `clear_wiped`, `draw_film`.
// Deps: crate::{metal, treemap}, objc2 AppKit.

use crate::metal::{circle_path, grey, rnd};
use crate::treemap::{contains, rust_tone, Tile, DONE};
use objc2_app_kit::{NSBezierPath, NSColor};
use objc2_foundation::{NSPoint, NSRect};
use std::cell::RefCell;

/// Mask resolution across the plate — fine enough for a soft wipe edge.
const GX: usize = 44;
const GY: usize = 24;
/// The mock sprays a 22pt circle and clears cells inside 85% of it.
const REACH: f64 = 18.7;

thread_local! {
    /// What the spray has wiped. It outlives the popover rebuild that every
    /// progress tick triggers, so a wipe is not undone by the next removal.
    static WIPED: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

pub fn clear_wiped() {
    WIPED.with(|mask| mask.borrow_mut().clear());
}

/// Mark what the nozzle just covered. Cells under live crust are left alone:
/// that crust is still on disk and only the removal job may lift it.
pub fn wipe(point: NSPoint, rect: NSRect, tiles: &[Tile]) {
    let (cw, ch) = (rect.size.width / GX as f64, rect.size.height / GY as f64);
    WIPED.with(|cell| {
        let mut mask = cell.borrow_mut();
        if mask.len() != GX * GY {
            mask.clear();
            mask.resize(GX * GY, 0);
        }
        for (index, value) in mask.iter_mut().enumerate() {
            let centre = cell_centre(rect, index, cw, ch);
            let (dx, dy) = (centre.x - point.x, centre.y - point.y);
            let distance = (dx * dx + dy * dy).sqrt();
            if distance >= REACH || crusted(tiles, centre) {
                continue;
            }
            *value = (*value as u32 + (80.0 * (1.0 - distance / REACH)) as u32).min(255) as u8;
        }
    });
}

/// Residue over ground the job has already cleared, thinned by whatever the
/// spray has wiped, plus the wet shine the spray leaves behind.
pub fn draw_film(rect: NSRect, tiles: &[Tile]) {
    let (cw, ch) = (rect.size.width / GX as f64, rect.size.height / GY as f64);
    let radius = cw.max(ch) * 0.9;
    WIPED.with(|cell| {
        let mask = cell.borrow();
        for index in 0..GX * GY {
            let centre = cell_centre(rect, index, cw, ch);
            if crusted(tiles, centre) {
                continue;
            }
            let wiped = mask.get(index).map_or(0.0, |v| *v as f64 / 255.0);
            if let Some(position) = tiles.iter().position(|tile| contains(tile.rect, centre)) {
                let (r, g, b) = rust_tone(position);
                NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 0.32 * (1.0 - wiped)).setFill();
                circle_path(centre, radius).fill();
            }
            if wiped > 0.0 {
                grey(1.0, 0.3 * wiped).setFill();
                circle_path(centre, radius).fill();
            }
        }
    });
}

fn cell_centre(rect: NSRect, index: usize, cw: f64, ch: f64) -> NSPoint {
    NSPoint::new(
        rect.origin.x + ((index % GX) as f64 + 0.5) * cw,
        rect.origin.y + ((index / GX) as f64 + 0.5) * ch,
    )
}

fn crusted(tiles: &[Tile], point: NSPoint) -> bool {
    tiles
        .iter()
        .find(|tile| contains(tile.rect, point))
        .is_some_and(|tile| tile.state != DONE)
}

// ── mist ─────────────────────────────────────────────────────────────────────

struct Mote {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    r: f64,
    life: f64,
}

#[derive(Default)]
pub struct Mist {
    motes: Vec<Mote>,
    seed: u32,
}

impl Mist {
    /// Three motes per burst, as the mock emits them: random heading, random
    /// speed, a small downward bias.
    pub fn emit(&mut self, point: NSPoint) {
        for _ in 0..3 {
            let angle = self.next() * std::f64::consts::TAU;
            let speed = 0.5 + self.next() * 1.9;
            let r = 1.0 + self.next() * 2.6;
            self.motes.push(Mote {
                x: point.x,
                y: point.y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed - 0.35,
                r,
                life: 1.0,
            });
        }
    }

    /// One frame of drift and fade. False once the last mote has gone.
    pub fn step(&mut self) -> bool {
        for mote in self.motes.iter_mut() {
            mote.x += mote.vx;
            mote.y += mote.vy;
            mote.vy -= 0.05;
            mote.life -= 0.045;
        }
        self.motes.retain(|mote| mote.life > 0.0);
        !self.motes.is_empty()
    }

    pub fn draw(&self) {
        for mote in &self.motes {
            grey(1.0, (mote.life * 0.75).clamp(0.0, 1.0)).setFill();
            circle_path(NSPoint::new(mote.x, mote.y), mote.r).fill();
        }
    }

    /// Wobble for a nozzle held still, so the spray widens as the mock's does.
    pub fn wobble(&mut self, point: NSPoint, amount: f64) -> NSPoint {
        let (dx, dy) = (self.next() - 0.5, self.next() - 0.5);
        NSPoint::new(point.x + dx * amount, point.y + dy * amount)
    }

    fn next(&mut self) -> f64 {
        if self.seed == 0 {
            self.seed = 0x2545_f491;
        }
        rnd(&mut self.seed)
    }
}

// ── the can ──────────────────────────────────────────────────────────────────

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
    ink(58.0, 55.0, 51.0).setFill();
    quad(pivot, angle, 6.0, -9.0, 11.0, 0.0);
    for strip in 0..8 {
        let t = strip as f64 / 7.0;
        let (r, g, b) = body_tone(t);
        ink(r, g, b).setFill();
        quad(pivot, angle, t * 24.0, 0.0, (t + 0.15) * 24.0, 44.0);
    }
    grey(0.94, 0.72).setFill();
    quad(pivot, angle, 0.0, 8.0, 24.0, 13.0);
    ink(91.0, 87.0, 81.0).setFill();
    quad(pivot, angle, 5.0, 43.0, 19.0, 51.0);
}

/// The mock's 100° body gradient: #2A2825 → #4A4742 at 46% → #1F1D1B.
fn body_tone(t: f64) -> (f64, f64, f64) {
    const STOPS: [(f64, [f64; 3]); 3] = [
        (0.0, [42.0, 40.0, 37.0]),
        (0.46, [74.0, 71.0, 66.0]),
        (1.0, [31.0, 29.0, 27.0]),
    ];
    let (lo, hi) = if t <= STOPS[1].0 { (STOPS[0], STOPS[1]) } else { (STOPS[1], STOPS[2]) };
    let f = ((t - lo.0) / (hi.0 - lo.0)).clamp(0.0, 1.0);
    let mix = |i: usize| lo.1[i] + (hi.1[i] - lo.1[i]) * f;
    (mix(0), mix(1), mix(2))
}

/// The can's parts carry the mock's hex values, kept as 0-255 channels so they
/// can be checked against the design without decoding a float.
fn ink(r: f64, g: f64, b: f64) -> objc2::rc::Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(r / 255.0, g / 255.0, b / 255.0, 1.0)
}

/// A rectangle in the can's own frame, rotated about the pivot and filled.
fn quad(pivot: NSPoint, angle: f64, x0: f64, y0: f64, x1: f64, y1: f64) {
    let (sin, cos) = angle.sin_cos();
    let path = NSBezierPath::new();
    for (index, (x, y)) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].iter().enumerate() {
        let point = NSPoint::new(pivot.x + x * cos - y * sin, pivot.y + x * sin + y * cos);
        match index {
            0 => path.moveToPoint(point),
            _ => path.lineToPoint(point),
        }
    }
    path.closePath();
    path.fill();
}
