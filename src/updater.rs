// Sparkle auto-update bridge for WD-40.
// Sparkle is loaded from the bundle at runtime, so unbundled dev builds still run.
// Exports: `Updater`, `bundle_version`.
// Deps: objc2, objc2_foundation.

use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::msg_send;
use objc2_foundation::{NSBundle, NSString};

const CONTROLLER_CLASS: &std::ffi::CStr = c"SPUStandardUpdaterController";

/// Owns Sparkle's updater controller. Menu items target it directly, so the
/// `checkForUpdates:` action needs no Rust-side handler.
pub struct Updater {
    controller: Retained<AnyObject>,
}

impl Updater {
    /// Load `Contents/Frameworks/Sparkle.framework` and start background checks.
    /// Returns `None` when the framework is absent — the app then runs without
    /// update support instead of failing to launch.
    pub fn start() -> Option<Self> {
        let class = load_sparkle()?;
        let controller: Retained<AnyObject> = unsafe {
            let allocated: Allocated<AnyObject> = msg_send![class, alloc];
            msg_send![
                allocated,
                initWithStartingUpdater: Bool::YES,
                updaterDelegate: Option::<&AnyObject>::None,
                userDriverDelegate: Option::<&AnyObject>::None
            ]
        };
        Some(Self { controller })
    }

    /// Action target for a "Check for Updates…" menu item.
    #[allow(dead_code)]
    pub fn target(&self) -> &AnyObject {
        &self.controller
    }

    #[allow(dead_code)]
    pub fn can_check(&self) -> bool {
        let updater = self.updater();
        let value: Bool = unsafe { msg_send![&*updater, canCheckForUpdates] };
        value.as_bool()
    }

    pub fn automatic_checks(&self) -> bool {
        let updater = self.updater();
        let value: Bool = unsafe { msg_send![&*updater, automaticallyChecksForUpdates] };
        value.as_bool()
    }

    pub fn set_automatic_checks(&self, enabled: bool) {
        let updater = self.updater();
        unsafe {
            let _: () = msg_send![
                &*updater,
                setAutomaticallyChecksForUpdates: Bool::new(enabled)
            ];
        }
    }

    /// Same action the menu item sends, for the Settings window's button.
    pub fn check_now(&self) {
        unsafe {
            let _: () = msg_send![&*self.controller, checkForUpdates: Option::<&AnyObject>::None];
        }
    }

    fn updater(&self) -> Retained<AnyObject> {
        unsafe { msg_send![&*self.controller, updater] }
    }
}

fn load_sparkle() -> Option<&'static AnyClass> {
    if let Some(class) = AnyClass::get(CONTROLLER_CLASS) {
        return Some(class);
    }
    let frameworks = NSBundle::mainBundle().privateFrameworksPath()?;
    let path = NSString::from_str(&format!("{frameworks}/Sparkle.framework"));
    let bundle = NSBundle::bundleWithPath(&path)?;
    if !unsafe { bundle.load() } {
        eprintln!("wd-40: Sparkle.framework failed to load; updates disabled");
        return None;
    }
    AnyClass::get(CONTROLLER_CLASS)
}

/// `CFBundleShortVersionString` when bundled, else the compiled crate version.
pub fn bundle_version() -> String {
    let key = NSString::from_str("CFBundleShortVersionString");
    let value = NSBundle::mainBundle().objectForInfoDictionaryKey(&key);
    match value {
        Some(object) => {
            let string: Retained<NSString> = unsafe { msg_send![&*object, description] };
            string.to_string()
        }
        None => env!("CARGO_PKG_VERSION").to_string(),
    }
}
