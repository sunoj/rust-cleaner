// The cleaning screen's plate: brushed steel under a rust crust, one tile per
// target. Tiles clear as the job removes them; rubbing only polishes the metal.
// Exports: `plate_view`, `clear_polish`.
// Deps: crate::{header, metal, state, treemap}, objc2 AppKit.

use crate::header::scan_size;
use crate::metal::{brushed, circle_path, grey, inset, outline, rings, rnd};
use crate::state::CleanProgress;
use crate::treemap::{contains, tiles, Tile, ACTIVE, DONE, SKIPPED};
use objc2::rc::Retained;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBezierPath, NSColor, NSEvent, NSTextField, NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, RefCell};

const GX: usize = 44;
const GY: usize = 24;

thread_local! {
    /// A rub has to outlive the popover rebuild that every progress tick
    /// triggers, so the polish mask lives beside the view, not inside it.
    static POLISH: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

pub fn clear_polish() {
    POLISH.with(|mask| mask.borrow_mut().clear());
}

pub struct PlateIvars {
    tiles: Vec<Tile>,
    hover: Cell<isize>,
    /// Tile names live under the plate, not on it: the small tiles have no room.
    legend: Retained<NSTextField>,
    idle: String,
    dark: bool,
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

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            self.track(event);
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            self.track(event);
            self.rub(event);
        }

        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, event: &NSEvent) {
            self.track(event);
            self.rub(event);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().hover.set(-1);
            self.refresh_legend();
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
        let centre = NSPoint::new(bounds.size.width / 2.0, bounds.size.height / 2.0);
        rings(centre, bounds.size.height * 0.44, ivars.dark);
        for (index, tile) in ivars.tiles.iter().enumerate() {
            draw_tile(index, tile);
        }
        draw_polish(bounds, &ivars.tiles);
        if let Some(tile) = self.hovered() {
            grey(1.0, 0.8).setStroke();
            outline(inset(tile.rect, 1.0), 1.5);
        }
        NSColor::colorWithSRGBRed_green_blue_alpha(0.11, 0.1, 0.09, 0.16).setStroke();
        let edge = NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(inset(bounds, 0.5), 9.5, 9.5);
        edge.setLineWidth(1.0);
        edge.stroke();
    }

    fn hovered(&self) -> Option<&Tile> {
        usize::try_from(self.ivars().hover.get())
            .ok()
            .and_then(|index| self.ivars().tiles.get(index))
    }

    fn track(&self, event: &NSEvent) {
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        let hit = self
            .ivars()
            .tiles
            .iter()
            .position(|tile| contains(tile.rect, point))
            .map_or(-1, |index| index as isize);
        if hit != self.ivars().hover.get() {
            self.ivars().hover.set(hit);
            self.refresh_legend();
            self.setNeedsDisplay(true);
        }
    }

    fn rub(&self, event: &NSEvent) {
        mark_polish(self.convertPoint_fromView(event.locationInWindow(), None), self.bounds());
        self.setNeedsDisplay(true);
    }

    fn refresh_legend(&self) {
        let text = match self.hovered() {
            Some(tile) => {
                let phase = match tile.state {
                    DONE => "removed",
                    ACTIVE => "removing",
                    SKIPPED => "left in place",
                    _ => "waiting",
                };
                format!("{} \u{00b7} {} \u{00b7} {phase}", tile.name, scan_size(tile.size))
            }
            None => self.ivars().idle.clone(),
        };
        self.ivars().legend.setStringValue(&NSString::from_str(&text));
    }
}

pub fn plate_view(
    frame: NSRect,
    progress: &CleanProgress,
    legend: Retained<NSTextField>,
    idle: String,
    dark: bool,
    mtm: MainThreadMarker,
) -> Retained<PlateView> {
    let view = mtm.alloc::<PlateView>().set_ivars(PlateIvars {
        tiles: tiles(progress, frame.size.width, frame.size.height),
        hover: Cell::new(-1),
        legend,
        idle,
        dark,
    });
    let view: Retained<PlateView> = unsafe { msg_send![super(view), initWithFrame: frame] };
    view.refresh_legend();
    view
}

fn draw_tile(index: usize, tile: &Tile) {
    const RUST: [(f64, f64, f64); 4] = [
        (0.545, 0.333, 0.2), (0.486, 0.275, 0.149), (0.584, 0.337, 0.18), (0.635, 0.376, 0.227),
    ];
    if tile.state == DONE || tile.rect.size.width < 0.5 {
        return;
    }
    let alpha = if tile.state == SKIPPED { 0.5 } else { 1.0 };
    let (r, g, b) = RUST[index % RUST.len()];
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, alpha).setFill();
    NSBezierPath::fillRect(tile.rect);
    let mut seed = 0x9e37_79b9_u32 ^ (index as u32).wrapping_mul(2_654_435_761);
    let pits = ((tile.rect.size.width * tile.rect.size.height / 220.0) as usize).clamp(6, 70);
    for _ in 0..pits {
        let cx = tile.rect.origin.x + rnd(&mut seed) * tile.rect.size.width;
        let cy = tile.rect.origin.y + rnd(&mut seed) * tile.rect.size.height;
        let (pr, pg, pb) = if rnd(&mut seed) > 0.5 { (0.769, 0.529, 0.31) } else { (0.29, 0.173, 0.106) };
        NSColor::colorWithSRGBRed_green_blue_alpha(pr, pg, pb, (0.1 + rnd(&mut seed) * 0.26) * alpha).setFill();
        circle_path(NSPoint::new(cx, cy), 1.5 + rnd(&mut seed) * 5.5).fill();
    }
    NSColor::colorWithSRGBRed_green_blue_alpha(0.118, 0.067, 0.035, 0.55 * alpha).setStroke();
    outline(inset(tile.rect, 0.5), 1.0);
    if tile.state == ACTIVE {
        NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 0.94, 0.86, 0.75).setStroke();
        outline(inset(tile.rect, 1.5), 2.0);
    }
}

/// Rubbing brings up a shine. It never lifts crust: what is still rusty is
/// still on disk, and only the removal job may say otherwise.
fn draw_polish(rect: NSRect, tiles: &[Tile]) {
    let (cw, ch) = (rect.size.width / GX as f64, rect.size.height / GY as f64);
    POLISH.with(|cell| {
        for (index, value) in cell.borrow().iter().enumerate().filter(|(_, v)| **v > 0) {
            let centre = NSPoint::new(
                rect.origin.x + ((index % GX) as f64 + 0.5) * cw,
                rect.origin.y + ((index / GX) as f64 + 0.5) * ch,
            );
            let bare = tiles
                .iter()
                .find(|tile| contains(tile.rect, centre))
                .is_none_or(|tile| tile.state == DONE);
            let alpha = *value as f64 / 255.0 * if bare { 0.36 } else { 0.08 };
            NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, alpha).setFill();
            circle_path(centre, cw.max(ch) * 0.9).fill();
        }
    });
}

fn mark_polish(point: NSPoint, rect: NSRect) {
    let (cw, ch) = (rect.size.width / GX as f64, rect.size.height / GY as f64);
    let radius = 18.0_f64;
    POLISH.with(|cell| {
        let mut mask = cell.borrow_mut();
        if mask.len() != GX * GY {
            mask.clear();
            mask.resize(GX * GY, 0);
        }
        for (index, value) in mask.iter_mut().enumerate() {
            let dx = ((index % GX) as f64 + 0.5) * cw - point.x;
            let dy = ((index / GX) as f64 + 0.5) * ch - point.y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance < radius {
                *value = (*value as u32 + (70.0 * (1.0 - distance / radius)) as u32).min(255) as u8;
            }
        }
    });
}
