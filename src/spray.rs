// What the spray does to the plate: the mist it throws off and the record of
// what it has wiped. Spraying lifts the residue left where a target has really
// gone; live crust it cannot touch. The can itself lives in `can`.
// Exports: `Mist`, `wipe`, `clear_wiped`, `draw_film`.
// Deps: crate::{metal, treemap}, objc2 AppKit.

use crate::metal::{circle_path, grey, rnd};
use crate::treemap::{contains, rust_tone, Tile, DONE};
use objc2_app_kit::NSColor;
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
            // The nozzle now runs for the whole job, so cells nowhere near it
            // are dropped before the square root and the tile scan.
            if dx.abs() >= REACH || dy.abs() >= REACH {
                continue;
            }
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
