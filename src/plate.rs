// The cleaning screen's plate: brushed steel under a rust crust, and a WD-40
// nozzle that works it — by itself while the job runs, from the pointer for as
// long as the pointer is on the plate. A tile clears only when its target is
// really gone. Exports: `plate_view`. Deps: crate::{can, crust, drift, spray}.

use crate::can::draw_nozzle;
use crate::crust::{idle_text, legend_text, paint_plate, plate_edge};
use crate::drift::{start_clock, stop_clock, ticking, Drift};
use crate::metal::{grey, inset, outline, reduce_motion};
use crate::spray::{wipe, Mist};
use crate::state::CleanProgress;
use crate::treemap::{contains, tiles, Tile, DONE};
use crate::widgets::retrack;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSTextField, NSTrackingAreaOptions, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSString};
use std::cell::{Cell, RefCell};

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
    /// Reduce Motion: the wipe still lands, nothing ever moves on its own.
    still: bool,
    pointer: Cell<NSPoint>,
    inside: Cell<bool>,
    held: Cell<bool>,
    mist: RefCell<Mist>,
    /// True while the job still has targets to work through. The spray runs
    /// itself for exactly that long, and this is read from the job's state.
    working: Cell<bool>,
    drift: RefCell<Drift>,
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
            // The plate is on screen and drawing, which is the one moment it is
            // certain the clock may run; from here on the clock draws itself.
            self.wake();
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
            self.ivars().held.set(true);
            self.burst(self.ivars().pointer.get());
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.track(event);
            self.burst(self.ivars().pointer.get());
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            self.ivars().held.set(false);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            let ivars = self.ivars();
            ivars.inside.set(false);
            ivars.held.set(false);
            ivars.hover.set(-1);
            // Hand back where the pointer left it, not where the job is.
            ivars.drift.borrow_mut().resume_from(ivars.pointer.get());
            self.refresh_legend();
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(sprayTick:))]
        fn spray_tick(&self, _timer: *mut AnyObject) {
            let ivars = self.ivars();
            if !self.on_screen() {
                ivars.held.set(false);
                return stop_clock();
            }
            if ivars.working.get() {
                let home = self.home();
                let tiles = ivars.tiles.borrow();
                ivars.drift.borrow_mut().step(&tiles, home);
            }
            if self.spray_on() {
                let point = ivars.mist.borrow_mut().wobble(self.nozzle(), 5.0);
                self.burst(point);
            }
            let alive = ivars.mist.borrow_mut().step();
            if !alive && !self.spray_on() {
                stop_clock();
            }
            self.setNeedsDisplay(true);
        }

        /// A closed popover takes its window with it, and a view with no window
        /// on screen must never keep a 50 Hz timer alive.
        #[unsafe(method(viewDidMoveToWindow))]
        fn view_did_move_to_window(&self) {
            if !self.on_screen() {
                self.ivars().inside.set(false);
                self.ivars().held.set(false);
                stop_clock();
            }
        }

        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: *mut NSEvent) -> bool {
            true
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            unsafe {
                let _: () = msg_send![super(self), updateTrackingAreas];
            }
            retrack(self, NSTrackingAreaOptions::MouseMoved);
        }
    }
);

impl PlateView {
    fn paint(&self) {
        let bounds = self.bounds();
        let ivars = self.ivars();
        let tiles = ivars.tiles.borrow();
        paint_plate(bounds, ivars.dark, &tiles);
        if let Some(tile) = usize::try_from(ivars.hover.get()).ok().and_then(|at| tiles.get(at)) {
            grey(1.0, 0.8).setStroke();
            outline(inset(tile.rect, 1.0), 1.5);
        }
        drop(tiles);
        ivars.mist.borrow().draw();
        if ivars.inside.get() || self.moving() {
            draw_nozzle(self.nozzle(), self.spray_on());
        }
        plate_edge(bounds);
    }

    /// Take the run's new state without rebuilding the view.
    pub fn update(&self, progress: &CleanProgress, crust: NSRect) {
        let ivars = self.ivars();
        let before: Vec<u8> = ivars.tiles.borrow().iter().map(|tile| tile.state).collect();
        let fresh = tiles(progress, crust);
        // Ground a target has just really left is what the nozzle works next.
        // The aim is read off the job; nothing here moves the job along.
        for (index, tile) in fresh.iter().enumerate() {
            if tile.state == DONE && before.get(index).is_some_and(|state| *state != DONE) {
                ivars.drift.borrow_mut().cleared(index);
            }
        }
        *ivars.tiles.borrow_mut() = fresh;
        ivars.working.set(progress.working());
        *ivars.idle.borrow_mut() = idle_text(&ivars.share, progress);
        let hover = ivars.hover.get();
        if usize::try_from(hover).is_ok_and(|index| index >= ivars.tiles.borrow().len()) {
            ivars.hover.set(-1);
        }
        self.refresh_legend();
        self.setNeedsDisplay(true);
    }

    /// True while the plate is performing: with Reduce Motion on it never is.
    fn moving(&self) -> bool {
        self.ivars().working.get() && !self.ivars().still
    }

    /// True while spray is coming out: the whole time the job is working, and
    /// under the hand while the button is down.
    fn spray_on(&self) -> bool {
        self.moving() || self.ivars().held.get()
    }

    /// Where the can is: the pointer takes it over for as long as it is on the
    /// plate, and the automatic path has it back the moment the pointer leaves.
    fn nozzle(&self) -> NSPoint {
        let ivars = self.ivars();
        match ivars.inside.get() {
            true => ivars.pointer.get(),
            false => ivars.drift.borrow().point(),
        }
    }

    /// Where the nozzle waits when the job has nothing left to point it at.
    fn home(&self) -> NSPoint {
        let size = self.bounds().size;
        NSPoint::new(size.width / 2.0, size.height / 2.0)
    }

    fn on_screen(&self) -> bool {
        self.window().is_some_and(|window| window.isVisible())
    }

    /// Run the clock while something still has to move. Called only from a
    /// draw, so the plate is on screen by then; a view with no window at all is
    /// a closed or rebuilt screen. Reduce Motion never starts it — a rub still
    /// wipes, straight off the mouse events.
    fn wake(&self) {
        if self.ivars().still || ticking() || !self.spray_on() || self.window().is_none() {
            return;
        }
        let target: &AnyObject = unsafe { &*(self as *const Self as *const AnyObject) };
        start_clock(target, sel!(sprayTick:));
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
        let ivars = self.ivars();
        let text = legend_text(&ivars.tiles.borrow(), ivars.hover.get(), &ivars.idle.borrow());
        ivars.legend.setStringValue(&NSString::from_str(&text));
    }
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
    stop_clock();
    let home = NSPoint::new(frame.size.width / 2.0, frame.size.height / 2.0);
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
        held: Cell::new(false),
        mist: RefCell::new(Mist::default()),
        working: Cell::new(progress.working()),
        drift: RefCell::new(Drift::new(home)),
    });
    let view: Retained<PlateView> = unsafe { msg_send![super(view), initWithFrame: frame] };
    view.refresh_legend();
    view
}
