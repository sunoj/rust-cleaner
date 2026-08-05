// Row container that tints itself while the pointer is inside it.
// Exports: `hover_row` for scan-result rows.
// Deps: objc2 AppKit tracking APIs; crate::widgets.

use crate::widgets::add_fill;
use objc2::rc::Retained;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSBox, NSEvent, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

pub struct HoverRowIvars {
    /// Sits behind the row's content, hidden until the pointer arrives.
    tint: Retained<NSBox>,
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
            self.ivars().tint.setHidden(false);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().tint.setHidden(true);
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            unsafe {
                for area in self.trackingAreas() {
                    self.removeTrackingArea(&area);
                }
                let _: () = msg_send![super(self), updateTrackingAreas];
                let options = NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect;
                let area = NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0)),
                    options,
                    Some(self),
                    None,
                );
                self.addTrackingArea(&area);
            }
        }
    }
);

/// A row that highlights on hover. The tint is a plain filled box rather than a
/// custom `drawRect:` — a non-opaque view that paints its own background does
/// not reliably erase what it painted last, and the leftovers wiped out the
/// group header sitting above the row.
pub fn hover_row(
    parent: &NSView,
    y: f64,
    height: f64,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Retained<HoverRow> {
    let width = crate::widgets::POPOVER_WIDTH;
    let frame = NSRect::new(NSPoint::new(0.0, y), NSSize::new(width, height));
    let row = mtm.alloc().set_ivars(HoverRowIvars {
        tint: make_tint(width, height, color, mtm),
    });
    let row: Retained<HoverRow> = unsafe { msg_send![super(row), initWithFrame: frame] };
    // Added first so every label and control the caller adds lands on top of it.
    row.addSubview(&row.ivars().tint);
    parent.addSubview(&row);
    row
}

fn make_tint(
    width: f64,
    height: f64,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Retained<NSBox> {
    let holder = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height)),
    );
    let tint = add_fill(&holder, 0.0, 0.0, width, height, color, 1.0, 0.0, mtm);
    tint.removeFromSuperview();
    tint.setHidden(true);
    tint
}
