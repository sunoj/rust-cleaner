// Entry point for the macOS WD-40 menu bar app.
// Owns the status item, app state, and the Objective-C menu action handler.
// Deps: objc2 AppKit bindings; background work lives in `tasks`.
mod autostart;
mod controls;
mod icon;
mod menu;
mod rules_menu;
mod state;
mod settings_window;
mod style;
mod tasks;
mod updater;

use menu::refresh_menu;
use state::{with_state, with_state_ret, AppState};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, MainThreadOnly};
use objc2_app_kit::{
    NSAlert, NSAlertStyle, NSApplication, NSApplicationActivationPolicy, NSControlStateValueOn,
    NSMenuItem, NSStatusBar,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSString, NSTimer};
use std::cell::RefCell;
use updater::Updater;
use wd40::config::Config;
use wd40::scanner::ArtifactGroup;

thread_local! {
    pub(crate) static HANDLER: RefCell<Option<Retained<MenuHandler>>> = const { RefCell::new(None) };
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

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: &NSMenuItem) {
            let mtm = self.mtm();
            with_state(|state| {
                let auto_update = state.updater.as_ref().map(|updater| updater.automatic_checks());
                let target: &AnyObject =
                    unsafe { &*(self as *const Self as *const AnyObject) };
                settings_window::show(
                    &state.config,
                    auto_update,
                    &updater::bundle_version(),
                    target,
                    mtm,
                );
            });
        }

        /// Any popup or artifact-type checkbox changed: read the whole window back.
        #[unsafe(method(settingsChanged:))]
        fn settings_changed(&self, _sender: &NSMenuItem) {
            let mtm = self.mtm();
            let Some((hours, days, types)) = settings_window::read_controls(mtm) else { return };
            with_state(|state| {
                let interval_changed = state.config.auto_clean_hours != hours;
                state.config.auto_clean_hours = hours;
                state.config.max_age_days = days;
                state.config.artifact_types = types;
                state.config.save();
                if interval_changed {
                    if hours > 0 {
                        tasks::start_auto_clean(hours);
                    } else {
                        tasks::stop_auto_clean();
                    }
                }
                refresh_menu(state, mtm);
            });
        }

        #[unsafe(method(settingsToggleLoginItem:))]
        fn settings_toggle_login_item(&self, sender: &NSMenuItem) {
            let mtm = self.mtm();
            let enable = sender.state() == NSControlStateValueOn;
            if let Err(err) = autostart::set_enabled(enable) {
                show_alert(mtm, "Launch at Login", &err);
            }
            with_state(|state| refresh_menu(state, mtm));
        }

        #[unsafe(method(settingsToggleAutoUpdate:))]
        fn settings_toggle_auto_update(&self, sender: &NSMenuItem) {
            let enabled = sender.state() == NSControlStateValueOn;
            with_state(|state| {
                if let Some(updater) = state.updater.as_ref() {
                    updater.set_automatic_checks(enabled);
                }
            });
        }

        #[unsafe(method(settingsCheckForUpdates:))]
        fn settings_check_for_updates(&self, _sender: &NSMenuItem) {
            with_state(|state| {
                if let Some(updater) = state.updater.as_ref() {
                    updater.check_now();
                }
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
    state::install(AppState {
        config,
        targets: Vec::new(),
        status_item,
        updater: Updater::start(),
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
