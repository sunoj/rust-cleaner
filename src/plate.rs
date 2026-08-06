// The cleaning screen's plate: brushed steel, a rust crust covering the share
// of the volume these targets occupy, and a WD-40 nozzle on the pointer. A tile
// only clears when its target is really gone; the spray lifts the residue left
// behind. Exports: `plate_view`. Deps: crate::{header, metal, spray, treemap}.

use crate::header::scan_size;
use crate::metal::{brushed, grey, inset, outline, reduce_motion, rings};
use crate::spray::{draw_film, draw_nozzle, wipe, Mist};
use crate::state::CleanProgress;
use crate::crust::{draw_tile, phase};
use crate::treemap::{contains, tiles, Tile};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSEvent, NSTextField, NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString, NSTimer};
use std::cell::{Cell, RefCell};

/// 50 Hz while the mist is alive, and not a tick longer.
const MIST_TICK: f64 = 1.0 / 50.0;

thread_local! {
    static MIST: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };
}

pub struct PlateIvars {
    /// Replaced in place as the run advances. Rebuilding the whole view for
    /// each tile that comes off took the hover and the spray in flight with it.
    tiles: RefCell<Vec<Tile>>,
    hover: Cell<isize>,
    /// Tile names live under the plate, not on it: small tiles have no room.
    legend: Retained<NSTextField>,
    /// How much of the volume this job is, fixed for the run.
    share: String,
    idle: RefCell<String>,
    dark: bool,
    /// Reduce Motion: the wipe still lands, the mist never travels.
    still: bool,
    pointer: Cell<NSPoint>,
    inside: Cell<bool>,
    spraying: Cell<bool>,
    mist: RefCell<Mist>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = PlateIvars]
    #[name = "WD40CleanPlate"]
    pub struct PlateView;

    impl PlateView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            self.paint();
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, event: &NSEvent) {
            self.track(event);
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.track(event);
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.track(event);
            self.ivars().spraying.set(true);
            self.burst(self.ivars().pointer.get());
            start_mist(self);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.track(event);
            self.burst(self.ivars().pointer.get());
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            self.ivars().spraying.set(false);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            let ivars = self.ivars();
            ivars.inside.set(false);
            ivars.spraying.set(false);
            ivars.hover.set(-1);
            self.refresh_legend();
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mistTick:))]
        fn mist_tick(&self, _timer: *mut AnyObject) {
            let ivars = self.ivars();
            // A popover closed mid-spray takes its window with it, and a view
            // with no window must never keep a 50 Hz timer alive.
            if self.window().is_none() {
                ivars.spraying.set(false);
                return stop_mist();
            }
            if ivars.spraying.get() {
                let point = ivars.mist.borrow_mut().wobble(ivars.pointer.get(), 5.0);
                self.burst(point);
            }
            let alive = ivars.mist.borrow_mut().step();
            if !alive && !ivars.spraying.get() {
                stop_mist();
            }
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: *mut NSEvent) -> bool {
            true
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            unsafe {
                for area in self.trackingAreas() {
                    self.removeTrackingArea(&area);
                }
                let _: () = msg_send![super(self), updateTrackingAreas];
                let area = NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
                    NSTrackingAreaOptions::MouseEnteredAndExited
                        | NSTrackingAreaOptions::MouseMoved
                        | NSTrackingAreaOptions::ActiveAlways
                        | NSTrackingAreaOptions::InVisibleRect,
                    Some(self),
                    None,
                );
                self.addTrackingArea(&area);
            }
        }
    }
);

impl PlateView {
    fn paint(&self) {
        let bounds = self.bounds();
        let ivars = self.ivars();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, 10.0, 10.0).addClip();
        brushed(bounds, ivars.dark);
        rings(NSPoint::new(bounds.size.width / 2.0, bounds.size.height / 2.0), bounds.size.height * 0.44, ivars.dark);
        let tiles = ivars.tiles.borrow();
        draw_film(bounds, &tiles);
        for (index, tile) in tiles.iter().enumerate() {
            draw_tile(index, tile);
        }
        drop(tiles);
        self.hovered(|tile| {
            if let Some(tile) = tile {
                grey(1.0, 0.8).setStroke();
                outline(inset(tile.rect, 1.0), 1.5);
            }
        });
        ivars.mist.borrow().draw();
        if ivars.inside.get() {
            draw_nozzle(ivars.pointer.get(), ivars.spraying.get());
        }
        NSColor::colorWithSRGBRed_green_blue_alpha(0.11, 0.1, 0.09, 0.16).setStroke();
        let edge = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(inset(bounds, 0.5), 9.5, 9.5);
        edge.setLineWidth(1.0);
        edge.stroke();
    }

    /// Hand the hovered tile to `body`; the tiles live behind a borrow now.
    fn hovered<R>(&self, body: impl FnOnce(Option<&Tile>) -> R) -> R {
        let tiles = self.ivars().tiles.borrow();
        let index = usize::try_from(self.ivars().hover.get()).ok();
        body(index.and_then(|index| tiles.get(index)))
    }

    /// Take the run's new state without rebuilding the view.
    pub fn update(&self, progress: &CleanProgress, crust: NSRect) {
        let ivars = self.ivars();
        *ivars.tiles.borrow_mut() = tiles(progress, crust);
        *ivars.idle.borrow_mut() = idle_text(&ivars.share, progress);
        let hover = ivars.hover.get();
        if usize::try_from(hover).is_ok_and(|index| index >= ivars.tiles.borrow().len()) {
            ivars.hover.set(-1);
        }
        self.refresh_legend();
        self.setNeedsDisplay(true);
    }

    /// One puff: wipe what the nozzle covers, then throw off mist.
    fn burst(&self, point: NSPoint) {
        let ivars = self.ivars();
        wipe(point, self.bounds(), &ivars.tiles.borrow());
        if !ivars.still {
            ivars.mist.borrow_mut().emit(point);
        }
        self.setNeedsDisplay(true);
    }

    fn track(&self, event: &NSEvent) {
        let ivars = self.ivars();
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        ivars.pointer.set(point);
        ivars.inside.set(true);
        let hit = ivars
            .tiles
            .borrow()
            .iter()
            .position(|tile| contains(tile.rect, point))
            .map_or(-1, |index| index as isize);
        if hit != ivars.hover.get() {
            ivars.hover.set(hit);
            self.refresh_legend();
        }
        self.setNeedsDisplay(true);
    }

    fn refresh_legend(&self) {
        let text = self.hovered(|tile| match tile {
            Some(tile) => format!("{} \u{00b7} {} \u{00b7} {}", tile.name, scan_size(tile.size), phase(tile.state)),
            None => self.ivars().idle.borrow().clone(),
        });
        self.ivars().legend.setStringValue(&NSString::from_str(&text));
    }
}

fn idle_text(share: &str, progress: &CleanProgress) -> String {
    format!("{share} \u{00b7} {}/{} clear", progress.done_count, progress.total_count)
}

#[allow(clippy::too_many_arguments)]
pub fn plate_view(
    frame: NSRect,
    progress: &CleanProgress,
    crust: NSRect,
    legend: Retained<NSTextField>,
    share: String,
    dark: bool,
    mtm: MainThreadMarker,
) -> Retained<PlateView> {
    stop_mist();
    let view = mtm.alloc::<PlateView>().set_ivars(PlateIvars {
        tiles: RefCell::new(tiles(progress, crust)),
        hover: Cell::new(-1),
        legend,
        idle: RefCell::new(idle_text(&share, progress)),
        share,
        dark,
        still: reduce_motion(),
        pointer: Cell::new(NSPoint::new(-100.0, -100.0)),
        inside: Cell::new(false),
        spraying: Cell::new(false),
        mist: RefCell::new(Mist::default()),
    });
    let view: Retained<PlateView> = unsafe { msg_send![super(view), initWithFrame: frame] };
    view.refresh_legend();
    view
}

fn start_mist(view: &PlateView) {
    if view.ivars().still {
        return;
    }
    stop_mist();
    let target: &AnyObject = unsafe { &*(view as *const PlateView as *const AnyObject) };
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            MIST_TICK, target, sel!(mistTick:), None, true,
        )
    };
    MIST.with(|cell| *cell.borrow_mut() = Some(timer));
}

fn stop_mist() {
    MIST.with(|cell| {
        if let Some(timer) = cell.borrow_mut().take() {
            timer.invalidate();
        }
    });
}
