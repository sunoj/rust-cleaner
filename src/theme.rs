// Design tokens from docs/design/.../colors_and_type.css (faded-rust accent).
// Exports: `Theme`, `Theme::current_app`, hex→NSColor helpers.
// Deps: objc2_app_kit, objc2_foundation.

use objc2::rc::Retained;
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSColor,
};
use objc2_foundation::NSArray;

/// Mapped from the design system; accent is the design's faded-rust opt-in.
#[derive(Clone, Copy)]
pub struct Theme {
    #[allow(dead_code)]
    pub dark: bool,
    pub ink: (f64, f64, f64),
    pub ink_2: (f64, f64, f64),
    pub ink_3: (f64, f64, f64),
    pub ink_4: (f64, f64, f64),
    #[allow(dead_code)]
    pub paper: (f64, f64, f64),
    pub surface: (f64, f64, f64),
    pub surface_2: (f64, f64, f64),
    pub surface_3: (f64, f64, f64),
    pub line: (f64, f64, f64),
    pub line_2: (f64, f64, f64),
    pub accent: (f64, f64, f64),
    pub accent_weak: (f64, f64, f64),
    pub pos: (f64, f64, f64),
    pub warn: (f64, f64, f64),
}

impl Theme {
    pub fn light() -> Self {
        Self {
            dark: false,
            ink: hex(0x1B1B1A),
            ink_2: hex(0x5C5B57),
            ink_3: hex(0x8A8984),
            ink_4: hex(0xB3B2AC),
            paper: hex(0xF6F5F3),
            surface: hex(0xFFFFFF),
            surface_2: hex(0xEFEEEB),
            surface_3: hex(0xE7E5E1),
            line: hex(0xE4E2DD),
            line_2: hex(0xD5D3CD),
            accent: hex(0x95604A),
            accent_weak: hex(0xEFE6E1),
            pos: hex(0x5E7257),
            warn: hex(0x9A7B43),
        }
    }

    pub fn dark_theme() -> Self {
        Self {
            dark: true,
            ink: hex(0xECEBE7),
            ink_2: hex(0xA7A6A0),
            ink_3: hex(0x7A7974),
            ink_4: hex(0x54534F),
            paper: hex(0x161614),
            surface: hex(0x1E1E1C),
            surface_2: hex(0x262624),
            surface_3: hex(0x2F2F2C),
            line: hex(0x2C2C29),
            line_2: hex(0x3A3A36),
            accent: hex(0x95604A),
            accent_weak: hex(0x3A2E28),
            pos: hex(0x5E7257),
            warn: hex(0x9A7B43),
        }
    }

    /// Resolve against the application's appearance, which is what the
    /// popover's material renders against. The status-bar button is not a
    /// substitute: the menu bar is dark under a dark wallpaper even while the
    /// system is in Light Mode, which paired light material with dark-mode ink.
    pub fn current_app(mtm: objc2_foundation::MainThreadMarker) -> Self {
        let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
        let names = NSArray::from_slice(&[
            unsafe { NSAppearanceNameAqua },
            unsafe { NSAppearanceNameDarkAqua },
        ]);
        let dark = app
            .effectiveAppearance()
            .bestMatchFromAppearancesWithNames(&names)
            .is_some_and(|name| &*name == unsafe { NSAppearanceNameDarkAqua });
        if dark {
            Self::dark_theme()
        } else {
            Self::light()
        }
    }

    pub fn color(rgb: (f64, f64, f64)) -> Retained<NSColor> {
        NSColor::colorWithSRGBRed_green_blue_alpha(rgb.0, rgb.1, rgb.2, 1.0)
    }

    pub fn color_alpha(rgb: (f64, f64, f64), alpha: f64) -> Retained<NSColor> {
        NSColor::colorWithSRGBRed_green_blue_alpha(rgb.0, rgb.1, rgb.2, alpha)
    }
}

fn hex(value: u32) -> (f64, f64, f64) {
    let r = ((value >> 16) & 0xff) as f64 / 255.0;
    let g = ((value >> 8) & 0xff) as f64 / 255.0;
    let b = (value & 0xff) as f64 / 255.0;
    (r, g, b)
}

/// Force the app appearance for screenshot / preview runs.
pub fn set_app_appearance(dark: bool, mtm: objc2_foundation::MainThreadMarker) {
    use objc2_app_kit::NSApplication;
    let name = if dark {
        unsafe { NSAppearanceNameDarkAqua }
    } else {
        unsafe { NSAppearanceNameAqua }
    };
    let appearance = NSAppearance::appearanceNamed(name);
    NSApplication::sharedApplication(mtm).setAppearance(appearance.as_deref());
}
