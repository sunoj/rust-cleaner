// The popover's animation, and the single place Reduce Motion is honoured for
// it: with that setting on every helper here lands on the final state at once,
// so nothing is ever left mid-transition.
// Exports: `arrive`, `glide`, `fade_to`.
// Deps: objc2 AppKit animator proxy, crate::metal::reduce_motion.

use crate::metal::reduce_motion;
use objc2_app_kit::{NSAnimatablePropertyContainer, NSAnimationContext, NSView};
use objc2_foundation::NSRect;

/// A figure appearing. Short enough that a burst of arriving sizes still reads
/// as a list filling in rather than a light show.
const ARRIVE: f64 = 0.22;
/// Something moving or growing: a bar, a plate tile, a row.
const GLIDE: f64 = 0.3;

/// A value that has just become known: it fades up instead of blinking in.
pub fn arrive(view: &NSView) {
    if reduce_motion() {
        view.setAlphaValue(1.0);
        return;
    }
    view.setAlphaValue(0.0);
    grouped(ARRIVE, || view.animator().setAlphaValue(1.0));
}

/// Move or resize a view to `frame`.
pub fn glide(view: &NSView, frame: NSRect) {
    if reduce_motion() {
        view.setFrame(frame);
        return;
    }
    grouped(GLIDE, || view.animator().setFrame(frame));
}

fn grouped(duration: f64, body: impl FnOnce()) {
    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(duration);
    body();
    NSAnimationContext::endGrouping();
}
