// The popover's animation, and the single place Reduce Motion is honoured for
// it: with that setting on every helper here lands on the final state at once,
// so nothing is ever left mid-transition.
// Exports: `arrive`, `glide`, `scan_indicator`, `scan_tick`, `stop_scan`.
// Deps: objc2 AppKit animator proxy, crate::metal::reduce_motion.

use crate::metal::reduce_motion;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAnimatablePropertyContainer, NSAnimationContext, NSControlSize, NSProgressIndicator,
    NSProgressIndicatorStyle, NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use std::cell::RefCell;

thread_local! {
    static SCAN_INDICATOR: RefCell<Option<Retained<NSProgressIndicator>>> = const { RefCell::new(None) };
}

/// A figure appearing. Short enough that a burst of arriving sizes still reads
/// as a list filling in rather than a light show.
const ARRIVE: f64 = 0.22;
/// Something moving or growing: a bar, a plate tile, a row.
const GLIDE: f64 = 0.3;
const SCAN_INDICATOR_SIZE: f64 = 16.0;

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

/// Add the one native spinner used while the scan is discovering or sizing.
pub fn scan_indicator(parent: &NSView, x: f64, y: f64, mtm: MainThreadMarker) {
    stop_scan();
    if reduce_motion() {
        return;
    }
    let indicator = NSProgressIndicator::new(mtm);
    indicator.setFrame(NSRect::new(
        NSPoint::new(x, y),
        NSSize::new(SCAN_INDICATOR_SIZE, SCAN_INDICATOR_SIZE),
    ));
    indicator.setStyle(NSProgressIndicatorStyle::Spinning);
    indicator.setControlSize(NSControlSize::Small);
    indicator.setIndeterminate(true);
    indicator.setDisplayedWhenStopped(false);
    parent.addSubview(&indicator);
    unsafe { indicator.startAnimation(None) };
    SCAN_INDICATOR.with(|cell| *cell.borrow_mut() = Some(indicator));
}

/// Keep the native indicator moving while sizing has no size tick to deliver.
pub fn scan_tick() {
    let indicator = SCAN_INDICATOR.with(|cell| cell.borrow().clone());
    let Some(indicator) = indicator else { return };
    if reduce_motion() {
        unsafe { indicator.stopAnimation(None) };
    } else {
        unsafe { indicator.startAnimation(None) };
    }
}

/// Stop and forget the scan indicator, including any in-flight native motion.
pub fn stop_scan() {
    SCAN_INDICATOR.with(|cell| {
        if let Some(indicator) = cell.borrow_mut().take() {
            unsafe { indicator.stopAnimation(None) };
        }
    });
}

pub fn scan_motion_allowed() -> bool {
    !reduce_motion()
}

fn grouped(duration: f64, body: impl FnOnce()) {
    NSAnimationContext::beginGrouping();
    NSAnimationContext::currentContext().setDuration(duration);
    body();
    NSAnimationContext::endGrouping();
}
