// Row container: tints itself while the pointer is inside it, says which
// directory it stands for while the pointer stays, and treats the whole row as
// the hit target rather than just the 15pt tick box.
// Exports: `HoverRow`, `hover_row`.
// Deps: objc2 AppKit tracking APIs; crate::{actions, widgets}.

use crate::widgets::{add_fill, retrack};
use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadOnly, Message};
use objc2_app_kit::{NSBox, NSControl, NSEvent, NSTextField, NSTrackingAreaOptions, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, RefCell};

/// The row's second line, and the two things it says.
struct Detail {
    field: Retained<NSTextField>,
    rest: Retained<NSString>,
    hover: Retained<NSString>,
}

pub struct HoverRowIvars {
    /// Sits behind the row's content, hidden until the pointer arrives.
    tint: Retained<NSBox>,
    /// Which row this is, in the same tag space the tick box uses.
    tag: Cell<isize>,
    /// Set once the row's labels exist, which is after the row itself does.
    detail: RefCell<Option<Detail>>,
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
            self.show_path(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().tint.setHidden(true);
            self.show_path(false);
        }

        /// A 15pt box is a small thing to ask someone to hit for an
        /// irreversible choice; the whole row carries it.
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            crate::actions::toggle_item(self.ivars().tag.get(), self.mtm());
        }

        /// Anything in the row with an action of its own keeps the click, so the
        /// tick box is not toggled twice; labels, washes and badges hand it up.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> *mut NSView {
            let hit: *mut NSView = unsafe { msg_send![super(self), hitTest: point] };
            if hit.is_null() || keeps_click(hit) {
                return hit;
            }
            self as *const Self as *mut NSView
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            unsafe {
                let _: () = msg_send![super(self), updateTrackingAreas];
            }
            retrack(self, NSTrackingAreaOptions::empty());
        }
    }
);

impl HoverRow {
    /// Hand the row the line under its name, what that line says at rest, and
    /// what it says under the pointer.
    pub fn set_detail(&self, field: &NSTextField, rest: &str, hover: &str) {
        *self.ivars().detail.borrow_mut() = Some(Detail {
            field: field.retain(),
            rest: NSString::from_str(rest),
            hover: NSString::from_str(hover),
        });
    }

    /// A row is a hit target for an irreversible delete and says only a project
    /// name; while the pointer is on it, the second line is the directory that
    /// would actually go. The whole path is on the row's tooltip as well, since
    /// this line has one line's worth of room.
    fn show_path(&self, hovering: bool) {
        let detail = self.ivars().detail.borrow();
        let Some(detail) = detail.as_ref() else { return };
        let text = if hovering { &detail.hover } else { &detail.rest };
        detail.field.setStringValue(text);
    }
}

/// True when this view answers clicks itself.
fn keeps_click(view: *mut NSView) -> bool {
    let is_control: bool = unsafe { msg_send![view, isKindOfClass: NSControl::class()] };
    if !is_control {
        return false;
    }
    let action: Option<Sel> = unsafe { msg_send![view, action] };
    action.is_some()
}

/// A row that highlights on hover and toggles from anywhere inside it. The tint
/// is a plain filled box rather than a custom `drawRect:` — a non-opaque view
/// that paints its own background does not reliably erase what it painted last,
/// and the leftovers wiped out the group header sitting above the row.
pub fn hover_row(
    parent: &NSView,
    y: f64,
    height: f64,
    color: (f64, f64, f64),
    tag: isize,
    mtm: MainThreadMarker,
) -> Retained<HoverRow> {
    let width = crate::widgets::POPOVER_WIDTH;
    let frame = NSRect::new(NSPoint::new(0.0, y), NSSize::new(width, height));
    let row = mtm.alloc().set_ivars(HoverRowIvars {
        tint: make_tint(width, height, color, mtm),
        tag: Cell::new(tag),
        detail: RefCell::new(None),
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
