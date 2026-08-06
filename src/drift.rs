// The nozzle nobody is holding: the plate's clock, and where the spray aims
// when the machine is working it alone. The aim is read off the job — the
// ground a target has just left, else the target coming off now — and this
// module never decides any of it. Exports: `Drift`, `TICK`, the clock.
// Deps: crate::treemap, objc2 Foundation timer + geometry.

use crate::treemap::{Tile, ACTIVE, DONE, PENDING};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2_foundation::{NSPoint, NSTimer};
use std::cell::RefCell;

/// 50 Hz while there is something to move, and not a tick longer.
pub const TICK: f64 = 1.0 / 50.0;
/// Seconds spent working ground a target has just left before the nozzle goes
/// back to where the job is.
const POLISH: f64 = 2.0;
/// Share of the gap to the aim point closed each frame. Slow enough to read as
/// one steady hand, quick enough to reach the next tile while it is still hot.
const EASE: f64 = 0.08;
/// How far the orbit swings from the tile centre, as a share of the tile.
const SWING: f64 = 0.3;

thread_local! {
    /// One plate on screen at a time, so one clock.
    static CLOCK: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

pub fn start_clock(target: &AnyObject, selector: Sel) {
    stop_clock();
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            TICK, target, selector, None, true,
        )
    };
    CLOCK.with(|cell| *cell.borrow_mut() = Some(timer));
}

pub fn ticking() -> bool {
    CLOCK.with(|cell| cell.borrow().is_some())
}

pub fn stop_clock() {
    CLOCK.with(|cell| {
        if let Some(timer) = cell.borrow_mut().take() {
            timer.invalidate();
        }
    });
}

/// Where the unattended nozzle is, and what it is heading for.
pub struct Drift {
    point: NSPoint,
    clock: f64,
    /// Tile whose target has just come off, and the seconds left working it.
    focus: Option<usize>,
    polish: f64,
}

impl Drift {
    pub fn new(home: NSPoint) -> Self {
        Self { point: home, clock: 0.0, focus: None, polish: 0.0 }
    }

    pub fn point(&self) -> NSPoint {
        self.point
    }

    /// A target has really gone: the ground it left is what to work next.
    pub fn cleared(&mut self, index: usize) {
        self.focus = Some(index);
        self.polish = POLISH;
    }

    /// Carry on from where the hand left off, so the nozzle glides back to the
    /// job instead of jumping there the moment the pointer leaves the plate.
    pub fn resume_from(&mut self, point: NSPoint) {
        self.point = point;
    }

    /// One frame: ease toward whatever the job is working on. `home` is only
    /// used while there is no tile left to aim at.
    pub fn step(&mut self, tiles: &[Tile], home: NSPoint) {
        self.clock += TICK;
        self.polish = (self.polish - TICK).max(0.0);
        if self.polish == 0.0 {
            self.focus = None;
        }
        let aim = self.aim(tiles).map_or(home, |tile| self.orbit(tile));
        self.point = NSPoint::new(
            self.point.x + (aim.x - self.point.x) * EASE,
            self.point.y + (aim.y - self.point.y) * EASE,
        );
    }

    /// Ground just cleared first, then the target being removed right now, then
    /// the largest one still queued.
    fn aim<'t>(&self, tiles: &'t [Tile]) -> Option<&'t Tile> {
        self.focus
            .and_then(|index| tiles.get(index))
            .filter(|tile| tile.state == DONE)
            .or_else(|| largest(tiles, ACTIVE))
            .or_else(|| largest(tiles, PENDING))
    }

    /// A slow figure over the tile, so the spray covers it instead of drilling
    /// one spot. The two rates differ, so the path does not close into a circle.
    fn orbit(&self, tile: &Tile) -> NSPoint {
        let (size, origin) = (tile.rect.size, tile.rect.origin);
        NSPoint::new(
            origin.x + size.width * (0.5 + SWING * (self.clock * 1.7).cos()),
            origin.y + size.height * (0.5 + SWING * (self.clock * 1.1).sin()),
        )
    }
}

/// The biggest tile in this state — the one whose work is most worth showing.
fn largest(tiles: &[Tile], state: u8) -> Option<&Tile> {
    tiles
        .iter()
        .filter(|tile| tile.state == state)
        .max_by(|a, b| area(a).total_cmp(&area(b)))
}

fn area(tile: &Tile) -> f64 {
    tile.rect.size.width * tile.rect.size.height
}

#[cfg(test)]
mod tests {
    use super::Drift;
    use crate::treemap::{Tile, ACTIVE, DONE};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    fn tile(x: f64, y: f64, side: f64, state: u8) -> Tile {
        Tile {
            rect: NSRect::new(NSPoint::new(x, y), NSSize::new(side, side)),
            name: String::new(),
            size: 0,
            state,
        }
    }

    fn run(drift: &mut Drift, tiles: &[Tile], frames: usize) -> NSPoint {
        let home = NSPoint::new(100.0, 50.0);
        for _ in 0..frames {
            drift.step(tiles, home);
        }
        drift.point()
    }

    #[test]
    fn the_nozzle_walks_to_the_target_coming_off_now() {
        let tiles = [tile(0.0, 0.0, 40.0, DONE), tile(200.0, 100.0, 60.0, ACTIVE)];
        let mut drift = Drift::new(NSPoint::new(0.0, 0.0));
        let point = run(&mut drift, &tiles, 200);
        assert!(point.x > 200.0 && point.x < 260.0, "{point:?}");
        assert!(point.y > 100.0 && point.y < 160.0, "{point:?}");
    }

    #[test]
    fn ground_just_cleared_is_worked_before_the_job_is_rejoined() {
        let tiles = [tile(0.0, 0.0, 40.0, DONE), tile(200.0, 100.0, 60.0, ACTIVE)];
        let mut drift = Drift::new(NSPoint::new(200.0, 100.0));
        drift.cleared(0);
        assert!(run(&mut drift, &tiles, 80).x < 60.0);
        assert!(run(&mut drift, &tiles, 300).x > 190.0);
    }

    #[test]
    fn with_nothing_left_to_work_the_nozzle_goes_home() {
        let tiles = [tile(0.0, 0.0, 40.0, DONE)];
        let mut drift = Drift::new(NSPoint::new(0.0, 0.0));
        let point = run(&mut drift, &tiles, 300);
        assert!((point.x - 100.0).abs() < 0.5 && (point.y - 50.0).abs() < 0.5, "{point:?}");
    }
}
