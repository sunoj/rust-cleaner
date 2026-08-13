// The row's reveal control: a small button on the right that shows that one
// directory in Finder. It carries a target and an action of its own, which is
// what keeps `HoverRow::hitTest` from handing its click to the row's tick.
// Exports: `reveal_button`, `reveal_item`.
// Deps: objc2 AppKit (NSButton, NSImage, NSWorkspace), crate::{state, theme}.

use crate::theme::Theme;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2_app_kit::{NSButton, NSButtonType, NSImage, NSImageScaling, NSView, NSWorkspace};
use objc2_foundation::{ns_string, MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use std::path::Path;

/// Side of the control's hit box. The glyph is drawn smaller inside it — the
/// box is sized to be hit, not to be seen.
pub const SIDE: f64 = 18.0;
const GLYPH: f64 = 13.0;
const HELP: &str = "Reveal in Finder";

#[allow(deprecated)]
#[allow(clippy::too_many_arguments)]
pub fn reveal_button(
    parent: &NSView,
    x: f64,
    y: f64,
    action: Sel,
    target: &AnyObject,
    tag: isize,
    color: (f64, f64, f64),
    mtm: MainThreadMarker,
) -> Option<Retained<NSButton>> {
    let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        ns_string!("folder"),
        Some(&NSString::from_str(HELP)),
    )?;
    image.setTemplate(true);
    image.setSize(NSSize::new(GLYPH, GLYPH));
    let button =
        unsafe { NSButton::buttonWithImage_target_action(&image, Some(target), Some(action), mtm) };
    button.setButtonType(NSButtonType::MomentaryChange);
    button.setBordered(false);
    button.setImageScaling(NSImageScaling::ScaleProportionallyDown);
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(SIDE, SIDE)));
    button.setTag(tag);
    button.setContentTintColor(Some(&Theme::color(color)));
    button.setToolTip(Some(&NSString::from_str(HELP)));
    parent.addSubview(&button);
    Some(button)
}

/// Show one row's directory in Finder. The path is read from the state at the
/// moment of the click, so it is the same target the row was drawn from — and
/// nothing here removes, moves or opens anything.
pub fn reveal_item(tag: isize) {
    let index = tag - crate::scan_view::TAG_REVEAL_BASE;
    let Ok(index) = usize::try_from(index) else { return };
    let path = crate::state::with_state_ret(|state| {
        state.targets.get(index).map(|target| target.path.clone())
    });
    if let Some(path) = path.flatten() {
        reveal(&path);
    }
}

/// An empty root asks Finder for a window of its own showing the item selected,
/// which is what "reveal" means everywhere else on the system.
fn reveal(path: &Path) {
    let full = NSString::from_str(&path.to_string_lossy());
    NSWorkspace::sharedWorkspace().selectFile_inFileViewerRootedAtPath(Some(&full), ns_string!(""));
}
