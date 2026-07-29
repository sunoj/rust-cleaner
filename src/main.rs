// Entry point for the macOS WD-40 menu bar app.
// Owns the status item, app state, and the Objective-C menu action handler.
// Deps: objc2 AppKit bindings; background work lives in `tasks`.
mod autostart;
mod icon;
mod menu;
mod settings;
mod style;
mod tasks;
mod updater;

use menu::refresh_menu;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSControlStateValueOn,
    NSMenuItem, NSStatusBar, NSStatusItem,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSString, NSTimer};
use std::cell::RefCell;
use std::time::Duration;
use updater::Updater;
use wd40::config::Config;
use wd40::disk::{disk_space, sum_bytes, DiskSpace};
use wd40::scanner::ArtifactGroup;

const SECONDS_PER_DAY: u64 = 86_400;

thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
    pub(crate) static HANDLER: RefCell<Option<Retained<MenuHandler>>> = const { RefCell::new(None) };
}

pub(crate) struct AppState {
    config: Config,
    targets: Vec<wd40::scanner::TargetDir>,
    status_item: Retained<NSStatusItem>,
    updater: Option<Updater>,
}

impl AppState {
    fn total_size(&self) -> u64 {
        sum_bytes(self.targets.iter().map(|t| t.size_bytes))
    }

    fn max_age(&self) -> Duration {
        Duration::from_secs(self.config.max_age_days.saturating_mul(SECONDS_PER_DAY))
    }

    /// Capacity of the volume holding the first scan dir that still exists.
    fn disk_stats(&self) -> Option<DiskSpace> {
        self.config
            .scan_dirs
            .iter()
            .find(|path| path.exists())
            .map(|path| path.as_path())
            .or_else(|| self.targets.first().map(|target| target.path.as_path()))
            .and_then(disk_space)
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "MenuHandler"]
    pub struct MenuHandler;

    impl MenuHandler {
        #[unsafe(method(handleCleanProject:))]
        fn handle_clean_project(&self, sender: &NSMenuItem) {
            let index = sender.tag() as usize;
            let work = with_state_ret(|state| {
                state.targets.get(index).map(|td| (td.path.clone(), td.size_bytes))
            })
            .flatten();
            if let Some((path, size)) = work {
                tasks::spawn_remove(path, size);
            }
        }

        #[unsafe(method(handleCleanAll:))]
        fn handle_clean_all(&self, _sender: &NSMenuItem) {
            let targets = with_state_ret(|state| state.targets.clone()).unwrap_or_default();
            tasks::spawn_clean_all(targets, "Clean All");
        }

        #[unsafe(method(handleCleanOld:))]
        fn handle_clean_old(&self, _sender: &NSMenuItem) {
            if let Some((targets, max_age)) =
                with_state_ret(|state| (state.targets.clone(), state.max_age()))
            {
                tasks::spawn_clean_old(targets, max_age, "Clean Old");
            }
        }

        #[unsafe(method(handleCleanGroup:))]
        fn handle_clean_group(&self, sender: &NSMenuItem) {
            let Some(group) = ArtifactGroup::from_tag(sender.tag()) else { return };
            let targets = with_state_ret(|state| {
                state
                    .targets
                    .iter()
                    .filter(|td| td.kind.group() == group)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
            tasks::spawn_clean_all(targets, group.label());
        }

        #[unsafe(method(handleGroupInfo:))]
        fn handle_group_info(&self, sender: &NSMenuItem) {
            let Some(group) = ArtifactGroup::from_tag(sender.tag()) else { return };
            show_alert(self.mtm(), group.label(), group.description());
        }

        #[unsafe(method(handleRescan:))]
        fn handle_rescan(&self, _sender: &NSMenuItem) {
            tasks::start_scan(false);
        }

        #[unsafe(method(handleSetAutoInterval:))]
        fn handle_set_auto_interval(&self, sender: &NSMenuItem) {
            let hours = sender.tag() as u64;
            let mtm = self.mtm();
            with_state(|state| {
                state.config.auto_clean_hours = hours;
                state.config.save();
                if hours > 0 {
                    tasks::start_auto_clean(hours);
                } else {
                    tasks::stop_auto_clean();
                }
                refresh_menu(state, mtm);
            });
        }

        #[unsafe(method(handleSetMaxAge:))]
        fn handle_set_max_age(&self, sender: &NSMenuItem) {
            let days = sender.tag() as u64;
            let mtm = self.mtm();
            with_state(|state| {
                state.config.max_age_days = days;
                state.config.save();
                refresh_menu(state, mtm);
            });
        }

        #[unsafe(method(handleToggleLoginItem:))]
        fn handle_toggle_login_item(&self, sender: &NSMenuItem) {
            let mtm = self.mtm();
            let enable = sender.state() != NSControlStateValueOn;
            if let Err(err) = autostart::set_enabled(enable) {
                show_alert(mtm, "Launch at Login", &err);
            }
            with_state(|state| refresh_menu(state, mtm));
        }

        #[unsafe(method(handleToggleAutoUpdate:))]
        fn handle_toggle_auto_update(&self, _sender: &NSMenuItem) {
            let mtm = self.mtm();
            with_state(|state| {
                if let Some(updater) = state.updater.as_ref() {
                    updater.set_automatic_checks(!updater.automatic_checks());
                }
                refresh_menu(state, mtm);
            });
        }

        #[unsafe(method(animTick:))]
        fn anim_tick(&self, _sender: &NSTimer) {
            tasks::tick_anim(self.mtm());
        }

        #[unsafe(method(autoCleanTick:))]
        fn auto_clean_tick(&self, _sender: &NSTimer) {
            if !tasks::is_busy() {
                tasks::start_scan(true);
            }
        }

        #[unsafe(method(autoScanTick:))]
        fn auto_scan_tick(&self, _sender: &NSTimer) {
            if !tasks::is_busy() {
                tasks::start_scan(false);
            }
        }

        #[unsafe(method(shineTick:))]
        fn shine_tick(&self, _sender: &NSTimer) {
            tasks::on_shine_done();
        }

        #[unsafe(method(scanDone:))]
        fn scan_done(&self, _sender: *mut AnyObject) {
            tasks::on_scan_done(self.mtm());
        }

        #[unsafe(method(sizesDone:))]
        fn sizes_done(&self, _sender: *mut AnyObject) {
            tasks::on_sizes_done(self.mtm());
        }

        #[unsafe(method(cleanDone:))]
        fn clean_done(&self, _sender: *mut AnyObject) {
            tasks::on_clean_done(self.mtm());
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: &NSMenuItem) {
            NSApplication::sharedApplication(self.mtm()).terminate(None);
        }
    }
);

impl MenuHandler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

fn show_alert(mtm: MainThreadMarker, title: &str, body: &str) {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(body));
    alert.setAlertStyle(NSAlertStyle::Informational);
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    alert.runModal();
}

fn main() {
    let mtm = MainThreadMarker::new().expect("must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let status_item = NSStatusBar::systemStatusBar().statusItemWithLength(-1.0);
    let config = Config::load();
    let auto_hours = config.auto_clean_hours;

    HANDLER.with(|cell| *cell.borrow_mut() = Some(MenuHandler::new(mtm)));
    APP_STATE.with(|cell| {
        *cell.borrow_mut() = Some(AppState {
            config,
            targets: Vec::new(),
            status_item,
            updater: Updater::start(),
        })
    });

    // Always start with a scan — zero-config, like npkill/kondo
    with_state(|state| refresh_menu(state, mtm));
    tasks::start_scan(false);
    tasks::start_auto_scan();
    if auto_hours > 0 {
        tasks::start_auto_clean(auto_hours);
    }

    app.run();
}

pub(crate) fn with_state<F: FnOnce(&mut AppState)>(f: F) {
    APP_STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            f(state);
        }
    });
}

pub(crate) fn with_state_ret<F: FnOnce(&mut AppState) -> R, R>(f: F) -> Option<R> {
    APP_STATE.with(|cell| cell.borrow_mut().as_mut().map(f))
}
