// The done screen's celebration medal: a polished steel disc with the mock's
// sheen sweep and twinkling sparkles, driven by a short self-stopping timer.
// Exports: `medal`, `start_sheen`, `stop_sheen`. Honours Reduce Motion.
// Deps: crate::metal, objc2 AppKit/Foundation.

use crate::metal::{brushed, circle_path, grey, reduce_motion, rings};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSBezierPath, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSTimer};
use std::cell::{Cell, RefCell};

/// One sweep every two seconds, stopping after four: a menu bar app has no
/// business running a timer behind a popover the user has already closed.
const TICK: f64 = 1.0 / 36.0;
const SWEEP_TICKS: f64 = 72.0;
const TOTAL_TICKS: u32 = 290;

thread_local! {
    static SHEEN: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

pub struct MedalIvars {
    phase: Cell<f64>,
    ticks: Cell<u32>,
    dark: bool,
    still: bool,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = MedalIvars]
    #[name = "WD40DoneMedal"]
    pub struct Medal;

    impl Medal {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            self.paint();
        }

        #[unsafe(method(sheenTick:))]
        fn sheen_tick(&self, _timer: *mut AnyObject) {
            let ivars = self.ivars();
            ivars.phase.set((ivars.phase.get() + 1.0 / SWEEP_TICKS) % 1.0);
            ivars.ticks.set(ivars.ticks.get() + 1);
            if ivars.ticks.get() >= TOTAL_TICKS {
                stop_sheen();
            }
            self.setNeedsDisplay(true);
        }
    }
);

impl Medal {
    fn paint(&self) {
        let bounds = self.bounds();
        let ivars = self.ivars();
        let radius = bounds.size.width / 2.0;
        let centre = NSPoint::new(radius, radius);
        circle_path(centre, radius).addClip();
        brushed(bounds, ivars.dark);
        rings(centre, radius * 0.96, ivars.dark);
        sheen(bounds, ivars.phase.get());
        star(NSPoint::new(radius * 0.55, radius * 1.42), radius * 0.16, self.twinkle(0.0));
        star(NSPoint::new(radius * 1.48, radius * 0.7), radius * 0.13, self.twinkle(0.35));
        grey(0.25, 0.22).setStroke();
        let edge = circle_path(centre, radius - 0.5);
        edge.setLineWidth(1.0);
        edge.stroke();
    }

    fn twinkle(&self, offset: f64) -> f64 {
        let ivars = self.ivars();
        if ivars.still {
            return 1.0;
        }
        (0.35 + 0.65 * ((ivars.phase.get() + offset) * std::f64::consts::TAU).sin().abs()).min(1.0)
    }
}

pub fn medal(frame: NSRect, dark: bool, mtm: MainThreadMarker) -> Retained<Medal> {
    let still = reduce_motion();
    let view = mtm.alloc::<Medal>().set_ivars(MedalIvars {
        // Reduce Motion parks the sweep where it looks resolved rather than
        // starting off-disc: the same final state, arrived at without travel.
        phase: Cell::new(if still { 0.5 } else { 0.0 }),
        ticks: Cell::new(0),
        dark,
        still,
    });
    unsafe { msg_send![super(view), initWithFrame: frame] }
}

pub fn start_sheen(disc: &Medal) {
    if disc.ivars().still {
        return;
    }
    let target: &AnyObject = unsafe { &*(disc as *const Medal as *const AnyObject) };
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            TICK, target, sel!(sheenTick:), None, true,
        )
    };
    SHEEN.with(|cell| *cell.borrow_mut() = Some(timer));
}

pub fn stop_sheen() {
    SHEEN.with(|cell| {
        if let Some(timer) = cell.borrow_mut().take() {
            timer.invalidate();
        }
    });
}

/// A soft highlight band travelling left to right across the disc.
fn sheen(rect: NSRect, phase: f64) {
    let x = -30.0 + phase * (rect.size.width + 60.0);
    for step in 0..12 {
        let t = step as f64 / 11.0;
        grey(1.0, 0.85 * (1.0 - (t * 2.0 - 1.0).abs()).powf(1.4)).setFill();
        NSBezierPath::fillRect(NSRect::new(
            NSPoint::new(x + t * 30.0, rect.origin.y),
            NSSize::new(3.0, rect.size.height),
        ));
    }
}

/// The mock's four-point sparkle, drawn as its clip-path polygon.
fn star(centre: NSPoint, radius: f64, alpha: f64) {
    const POINTS: [(f64, f64); 8] = [
        (0.5, 0.0), (0.57, 0.43), (1.0, 0.5), (0.57, 0.57),
        (0.5, 1.0), (0.43, 0.57), (0.0, 0.5), (0.43, 0.43),
    ];
    let path = NSBezierPath::new();
    for (index, (px, py)) in POINTS.iter().enumerate() {
        let point = NSPoint::new(centre.x + (px - 0.5) * radius * 2.0, centre.y + (py - 0.5) * radius * 2.0);
        match index {
            0 => path.moveToPoint(point),
            _ => path.lineToPoint(point),
        }
    }
    path.closePath();
    grey(1.0, alpha).setFill();
    path.fill();
}
