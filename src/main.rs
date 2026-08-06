// Entry point for the macOS WD-40 status-item popover app.
// Owns the status item, app state, and Objective-C action handler.
// Deps: objc2 AppKit; UI in `popover` + views; work in `tasks`.

mod actions;
mod autostart;
mod cache_names;
mod can;
mod checkbox;
mod clean_view;
mod controls;
mod crust;
mod disk_gauge;
mod done_view;
mod drift;
mod grip;
mod header;
mod hover_row;
mod icon;
mod legend;
mod live;
mod medal;
mod menu_rows;
mod metal;
mod motion;
mod mainthread;
mod names;
mod pace;
mod plate;
mod popover;
mod reveal;
mod scan_rows;
mod scrolling;
mod scan_view;
#[cfg(debug_assertions)]
mod screenshot;
mod selection;
mod settings_roots;
mod settings_row;
mod settings_view;
mod spray;
mod state;
mod style;
mod tasks;
mod tasks_clean;
mod theme;
mod treemap;
mod updater;
mod widgets;

use state::{with_state, AppState, UiScreen};
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSButton, NSSlider, NSStatusBar,
};
use objc2_foundation::{MainThreadMarker, NSObject};
use std::cell::RefCell;
use updater::Updater;
use wd40::config::Config;

thread_local! {
    pub(crate) static HANDLER: RefCell<Option<Retained<MenuHandler>>> = const { RefCell::new(None) };
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[name = "MenuHandler"]
    pub struct MenuHandler;

    impl MenuHandler {
        #[unsafe(method(togglePopover:))]
        fn toggle_popover(&self, _sender: &AnyObject) {
            popover::toggle(self.mtm());
        }

        #[unsafe(method(handleToggleItem:))]
        fn handle_toggle_item(&self, sender: &NSButton) {
            actions::toggle_item(sender.tag(), self.mtm());
        }

        #[unsafe(method(handleToggleGroup:))]
        fn handle_toggle_group(&self, sender: &NSButton) {
            let index = (sender.tag() - scan_view::TAG_GROUP_BASE) as usize;
            let Some(&group) = wd40::scanner::ArtifactGroup::ALL.get(index) else { return };
            with_state(|state| {
                let empty = state.empty_targets();
                selection::toggle_group(&state.targets, &mut state.selected, group, &empty);
            });
            if !live::selection_changed(self.mtm()) {
                popover::refresh(self.mtm());
            }
        }

        #[unsafe(method(handleRevealItem:))]
        fn handle_reveal_item(&self, sender: &NSButton) {
            reveal::reveal_item(sender.tag());
        }

        #[unsafe(method(handleCleanSelected:))]
        fn handle_clean_selected(&self, _sender: &AnyObject) {
            tasks::spawn_clean_selected();
        }

        #[unsafe(method(handleStopClean:))]
        fn handle_stop_clean(&self, _sender: &AnyObject) {
            tasks::request_stop();
        }

        /// The run is over; the screen is only still up to be read.
        #[unsafe(method(handleShowResult:))]
        fn handle_show_result(&self, _sender: &AnyObject) {
            tasks::show_result_now(self.mtm());
        }

        #[unsafe(method(cleanHoldTick:))]
        fn clean_hold_tick(&self, _sender: *mut AnyObject) {
            tasks::show_result(self.mtm());
        }

        #[unsafe(method(handleDoneAck:))]
        fn handle_done_ack(&self, _sender: &AnyObject) {
            actions::done_ack(self.mtm());
        }

        #[unsafe(method(handleShowMore:))]
        fn handle_show_more(&self, _sender: &AnyObject) {
            actions::show_more(self.mtm());
        }

        #[unsafe(method(handleRescan:))]
        fn handle_rescan(&self, _sender: &AnyObject) {
            with_state(|state| state.screen = UiScreen::Scan);
            tasks::start_scan(false);
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: &AnyObject) {
            actions::open_settings(self.mtm());
        }

        #[unsafe(method(settingsCycleInterval:))]
        fn settings_cycle_interval(&self, _sender: &AnyObject) {
            actions::cycle_interval(self.mtm());
        }

        #[unsafe(method(settingsSetMaxAge:))]
        fn settings_set_max_age(&self, sender: &NSSlider) {
            let days = sender.doubleValue().round().clamp(0.0, 30.0) as u64;
            actions::set_max_age(days, self.mtm());
        }

        #[unsafe(method(settingsCycleDepth:))]
        fn settings_cycle_depth(&self, _sender: &AnyObject) {
            actions::cycle_depth(self.mtm());
        }

        #[unsafe(method(settingsToggleGroup:))]
        fn settings_toggle_group(&self, sender: &NSButton) {
            actions::toggle_scan_group(sender, self.mtm());
        }

        #[unsafe(method(settingsToggleMenuBarSize:))]
        fn settings_toggle_menu_bar_size(&self, _sender: &AnyObject) {
            actions::toggle_menu_bar_size(self.mtm());
        }

        #[unsafe(method(settingsAddRoot:))]
        fn settings_add_root(&self, _sender: &AnyObject) {
            actions::add_scan_root(self.mtm());
        }

        #[unsafe(method(settingsRemoveRoot:))]
        fn settings_remove_root(&self, sender: &NSButton) {
            actions::remove_scan_root(sender, self.mtm());
        }

        #[unsafe(method(settingsBack:))]
        fn settings_back(&self, _sender: &AnyObject) {
            actions::close_settings(self.mtm());
        }

        #[unsafe(method(settingsToggleLoginItem:))]
        fn settings_toggle_login_item(&self, _sender: &AnyObject) {
            actions::toggle_login(self.mtm());
        }

        #[unsafe(method(settingsToggleAutoUpdate:))]
        fn settings_toggle_auto_update(&self, _sender: &AnyObject) {
            actions::toggle_auto_update(self.mtm());
        }

        #[unsafe(method(settingsCheckForUpdates:))]
        fn settings_check_for_updates(&self, _sender: &AnyObject) {
            actions::check_updates();
        }

        #[unsafe(method(autoCleanTick:))]
        fn auto_clean_tick(&self, _sender: *mut AnyObject) {
            if !tasks::is_busy() {
                tasks::start_scan(true);
            }
        }

        #[unsafe(method(autoScanTick:))]
        fn auto_scan_tick(&self, _sender: *mut AnyObject) {
            if !tasks::is_busy() {
                tasks::start_scan(false);
            }
        }

        #[unsafe(method(scanDone:))]
        fn scan_done(&self, _sender: *mut AnyObject) {
            tasks::on_scan_done(self.mtm());
        }

        #[unsafe(method(sizesTick:))]
        fn sizes_tick(&self, _sender: *mut AnyObject) {
            tasks::on_sizes_tick(self.mtm());
        }

        #[unsafe(method(reclaimDone:))]
        fn reclaim_done(&self, _sender: *mut AnyObject) {
            tasks::on_reclaim_done(self.mtm());
        }

        #[unsafe(method(sizesDone:))]
        fn sizes_done(&self, _sender: *mut AnyObject) {
            tasks::on_sizes_done(self.mtm());
        }

        #[unsafe(method(cleanDone:))]
        fn clean_done(&self, _sender: *mut AnyObject) {
            tasks::on_clean_done(self.mtm());
        }

        #[unsafe(method(cleanProgress:))]
        fn clean_progress(&self, _sender: *mut AnyObject) {
            tasks::on_progress(self.mtm());
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: &AnyObject) {
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

fn main() {
    let mtm = MainThreadMarker::new().expect("must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    // Force AppKit appearance to match the requested theme so layer-backed
    // system controls (and our token pick) agree in screenshots.
    if let Ok(value) = std::env::var("WD40_APPEARANCE") {
        theme::set_app_appearance(value.eq_ignore_ascii_case("dark"), mtm);
    }

    let status_item = NSStatusBar::systemStatusBar().statusItemWithLength(-1.0);
    // Level two first: the scan that starts a moment from now is the one it
    // exists to spare.
    wd40::cache::load();
    let config = Config::load();
    let auto_hours = config.auto_clean_hours;

    HANDLER.with(|cell| *cell.borrow_mut() = Some(MenuHandler::new(mtm)));
    state::install(AppState {
        config,
        targets: Vec::new(),
        measured: Default::default(),
        selected: Default::default(),
        show_all: false,
        screen: UiScreen::Scan,
        cleaning: None,
        done: None,
        status_item,
        updater: Updater::start(),
        reclaim: None,
    });

    popover::attach(mtm);
    tasks::start_scan(false);
    tasks::start_auto_scan();
    if auto_hours > 0 {
        tasks::start_auto_clean(auto_hours);
    }

    app.run();
}
