// Native AppKit row background that tracks pointer entry and exit.
// Exports: `hover_row` for scan-result rows.
// Deps: objc2 AppKit tracking and drawing APIs; crate::theme.

use crate::theme::Theme;
use objc2::rc::Retained;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSBezierPath, NSColor, NSEvent, NSView, NSTrackingArea, NSTrackingAreaOptions};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use std::cell::Cell;

pub struct HoverRowIvars {
    hovered: Cell<bool>,
    color: Retained<NSColor>,
}

define_class!(
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = HoverRowIvars]
    #[name = "WD40HoverRow"]
    pub struct HoverRow;

    impl HoverRow {
        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.ivars().hovered.set(true);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().hovered.set(false);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            unsafe {
                let areas = self.trackingAreas();
                for area in areas {
                    self.removeTrackingArea(&area);
                }
                let _: () = msg_send![super(self), updateTrackingAreas];
                let options = NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect;
                let area = NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(), NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
                    options, Some(self), None,
                );
                self.addTrackingArea(&area);
            }
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, rect: NSRect) {
            unsafe { let _: () = msg_send![super(self), drawRect: rect]; }
            if self.ivars().hovered.get() {
                self.ivars().color.setFill();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect, 0.0, 0.0).fill();
            }
        }
    }
);

pub fn hover_row(parent: &NSView, y: f64, height: f64, color: (f64, f64, f64), mtm: MainThreadMarker) -> Retained<HoverRow> {
    let frame = NSRect::new(NSPoint::new(0.0, y), NSSize::new(crate::widgets::POPOVER_WIDTH, height));
    let row = mtm.alloc().set_ivars(HoverRowIvars { hovered: Cell::new(false), color: Theme::color(color) });
    let row: Retained<HoverRow> = unsafe { msg_send![super(row), initWithFrame: frame] };
    parent.addSubview(&row);
    row
}
