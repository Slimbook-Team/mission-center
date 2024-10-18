/* window.rs
 *
 * Copyright 2024 Romeo Calota
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cell::Cell;

use adw::{prelude::*, subclass::prelude::*};
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib};

use crate::{app, settings, sys_info_v2::Readings};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::MissionCenterWindow)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/window.ui")]
    pub struct MissionCenterWindow {
        #[template_child]
        pub breakpoint: TemplateChild<adw::Breakpoint>,
        #[template_child]
        pub split_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub window_content: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub bottom_bar: TemplateChild<adw::ViewSwitcherBar>,
        #[template_child]
        pub sidebar_edit_mode_enable_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub sidebar_edit_mode_disable_all: TemplateChild<gtk::Button>,
        #[template_child]
        pub sidebar_edit_mode_reset: TemplateChild<gtk::Button>,
        #[template_child]
        pub toggle_sidebar_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub sidebar: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub performance_page: TemplateChild<crate::performance_page::PerformancePage>,
        #[template_child]
        pub apps_page: TemplateChild<crate::apps_page::AppsPage>,
        #[template_child]
        pub services_stack_page: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub services_page: TemplateChild<crate::services_page::ServicesPage>,
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub header_tabs: TemplateChild<adw::ViewSwitcher>,
        #[template_child]
        pub header_search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub loading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub loading_spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub stack: TemplateChild<adw::ViewStack>,

        #[property(get)]
        performance_page_active: Cell<bool>,
        #[property(get)]
        apps_page_active: Cell<bool>,
        #[property(get)]
        services_page_active: Cell<bool>,
        #[property(get)]
        user_hid_sidebar: Cell<bool>,

        #[property(name = "info-button-visible", get = Self::info_button_visible, type = bool)]
        _info_button_visible: [u8; 0],
        #[property(name = "search-button-visible", get = Self::search_button_visible, type = bool)]
        _search_button_visible: [u8; 0],

        #[property(get, set)]
        summary_mode: Cell<bool>,
        #[property(get, set)]
        collapse_threshold: Cell<i32>,
    }

    impl Default for MissionCenterWindow {
        fn default() -> Self {
            Self {
                breakpoint: TemplateChild::default(),
                split_view: TemplateChild::default(),
                window_content: TemplateChild::default(),
                bottom_bar: TemplateChild::default(),
                sidebar_edit_mode_enable_all: TemplateChild::default(),
                sidebar_edit_mode_disable_all: TemplateChild::default(),
                sidebar_edit_mode_reset: TemplateChild::default(),
                toggle_sidebar_button: TemplateChild::default(),
                sidebar: TemplateChild::default(),
                performance_page: TemplateChild::default(),
                apps_page: TemplateChild::default(),
                services_stack_page: TemplateChild::default(),
                services_page: TemplateChild::default(),
                header_bar: TemplateChild::default(),
                header_stack: TemplateChild::default(),
                header_tabs: TemplateChild::default(),
                header_search_entry: TemplateChild::default(),
                search_button: TemplateChild::default(),
                loading_box: TemplateChild::default(),
                loading_spinner: TemplateChild::default(),
                stack: TemplateChild::default(),

                performance_page_active: Cell::new(true),
                apps_page_active: Cell::new(false),
                services_page_active: Cell::new(false),
                user_hid_sidebar: Cell::new(false),

                _info_button_visible: [0; 0],
                _search_button_visible: [0; 0],

                summary_mode: Cell::new(false),
                collapse_threshold: Cell::new(0),
            }
        }
    }

    impl MissionCenterWindow {
        fn info_button_visible(&self) -> bool {
            if self.performance_page.is_bound() {
                self.performance_page_active.get() && self.performance_page.info_button_visible()
            } else {
                false
            }
        }

        fn search_button_visible(&self) -> bool {
            self.apps_page_active.get() || self.services_page_active.get()
        }
    }

    impl MissionCenterWindow {
        fn update_active_page(&self) {
            use glib::g_critical;

            let visible_child_name = self.stack.visible_child_name().unwrap_or("".into());

            if visible_child_name == "performance-page" {
                if self.performance_page_active.get() {
                    return;
                }

                self.performance_page_active.set(true);
                self.obj().notify_performance_page_active();

                self.apps_page_active.set(false);
                self.obj().notify_apps_page_active();

                self.services_page_active.set(false);
                self.obj().notify_services_page_active();
            }
            if visible_child_name == "apps-page" {
                if self.apps_page_active.get() {
                    return;
                }

                self.performance_page_active.set(false);
                self.obj().notify_performance_page_active();

                self.apps_page_active.set(true);
                self.obj().notify_apps_page_active();

                self.services_page_active.set(false);
                self.obj().notify_services_page_active();
            } else if visible_child_name == "services-page" {
                if self.services_page_active.get() {
                    return;
                }

                self.performance_page_active.set(false);
                self.obj().notify_performance_page_active();

                self.apps_page_active.set(false);
                self.obj().notify_apps_page_active();

                self.services_page_active.set(true);
                self.obj().notify_services_page_active();
            }

            self.obj().notify_info_button_visible();
            self.obj().notify_search_button_visible();

            settings!()
                .set_string("window-selected-page", &visible_child_name)
                .unwrap_or_else(|_| {
                    g_critical!(
                        "MissionCenter",
                        "Failed to set window-selected-page setting"
                    );
                });
        }
    }

    impl MissionCenterWindow {
        fn configure_actions(&self) {
            let toggle_search =
                gio::SimpleAction::new_stateful("toggle-search", None, &false.to_variant());
            toggle_search.connect_activate({
                let this = self.obj().downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let new_state = !action.state().and_then(|v| v.get::<bool>()).unwrap_or(true);
                    action.set_state(&new_state.to_variant());
                    this.search_button.set_active(new_state);

                    if new_state {
                        this.header_stack.set_visible_child_name("search-entry");
                        this.header_search_entry.grab_focus();
                        this.header_search_entry.select_region(-1, -1);

                        this.header_stack.set_visible(true);
                    } else {
                        if this.window_width_below_threshold() {
                            this.header_stack.set_visible(false);
                        }

                        this.header_search_entry.set_text("");
                        this.header_stack.set_visible_child_name("view-switcher");
                    }
                }
            });
            self.obj().add_action(&toggle_search);
        }

        #[inline]
        fn window_width_below_threshold(&self) -> bool {
            let window_width =
                adw::LengthUnit::from_px(adw::LengthUnit::Sp, self.obj().width() as _, None);
            let collapse_threshold = self.collapse_threshold.get() as f64;

            window_width < collapse_threshold
        }

        #[inline]
        pub(crate) fn should_hide_sidebar(&self) -> bool {
            let performance_page_active = self.performance_page_active.get();
            let summary_mode = self.summary_mode.get();
            let user_hid_sidebar = self.user_hid_sidebar.get();

            !performance_page_active
                || user_hid_sidebar
                || summary_mode
                || self.window_width_below_threshold()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MissionCenterWindow {
        const NAME: &'static str = "MissionCenterWindow";
        type Type = super::MissionCenterWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            use crate::{
                apps_page::AppsPage, performance_page::PerformancePage, services_page::ServicesPage,
            };

            PerformancePage::ensure_type();
            AppsPage::ensure_type();
            ServicesPage::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MissionCenterWindow {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            use glib::*;

            self.parent_constructed();

            self.configure_actions();

            idle_add_local_once({
                let this = self.obj().downgrade();
                move || {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        this.update_active_page();
                    }
                }
            });

            self.sidebar.connect_row_activated({
                let this = self.obj().downgrade();
                move |_, _| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        this.stack.set_visible_child_name("apps-page");
                        this.stack.set_visible_child_name("performance-page");
                    }
                }
            });

            self.sidebar_edit_mode_enable_all.connect_clicked({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        this.performance_page.sidebar_enable_all();
                    }
                }
            });

            self.sidebar_edit_mode_disable_all.connect_clicked({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        this.performance_page.sidebar_disable_all();
                    }
                }
            });

            self.sidebar_edit_mode_reset.connect_clicked({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        this.performance_page.sidebar_reset_to_default();
                    }
                }
            });

            self.stack.connect_visible_child_notify({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    if this.search_button.is_active() {
                        let _ = WidgetExt::activate_action(
                            this.obj().as_ref(),
                            "win.toggle-search",
                            None,
                        );
                    }
                    this.update_active_page();
                }
            });

            let evt_ctrl_key = gtk::EventControllerKey::new();
            evt_ctrl_key.connect_key_pressed({
                let this = self.obj().downgrade();
                move |controller, _, _, _| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        if this.services_page_active.get() && this.services_page.dialog_visible() {
                            controller.forward(&this.services_page.get());
                        } else {
                            controller.forward(&this.header_search_entry.get());
                        }
                    }

                    Propagation::Proceed
                }
            });
            self.obj().add_controller(evt_ctrl_key);

            self.header_search_entry
                .set_key_capture_widget(Some(&self.header_search_entry.get()));

            self.header_search_entry.connect_search_started({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();

                        if this.apps_page_active.get() || this.services_page_active.get() {
                            let _ = WidgetExt::activate_action(
                                this.obj().as_ref(),
                                "win.toggle-search",
                                None,
                            );
                        }
                    }
                }
            });

            self.header_search_entry.connect_stop_search({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        if this.apps_page_active.get() || this.services_page_active.get() {
                            let _ = WidgetExt::activate_action(
                                this.obj().as_ref(),
                                "win.toggle-search",
                                None,
                            );
                        }
                    }
                }
            });

            // Triggered when user interacts with the sidebar toggle button
            // via any means (clicking, keyboard shortcut, etc.)
            self.toggle_sidebar_button.connect_clicked({
                let this = self.obj().downgrade();
                move |button| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let user_hid_sidebar = !button.is_active();
                    if user_hid_sidebar != this.user_hid_sidebar.get() {
                        this.user_hid_sidebar.set(user_hid_sidebar);
                        if this.performance_page_active.get() {
                            if !this.window_width_below_threshold() {
                                this.split_view.set_collapsed(false);
                            }
                        }
                        this.obj().notify_user_hid_sidebar();
                    }
                }
            });

            self.breakpoint.set_condition(Some(
                &adw::BreakpointCondition::parse(&format!(
                    "max-width: {}sp",
                    self.collapse_threshold.get()
                ))
                .unwrap(),
            ));
            self.breakpoint.connect_apply({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.bottom_bar.set_reveal(true);
                    if !this.search_button.is_active() {
                        this.header_stack.set_visible(false);
                    }

                    this.services_page.collapse();

                    if !this.performance_page_active.get() {
                        return;
                    }

                    this.split_view.set_collapsed(this.should_hide_sidebar());
                }
            });
            self.breakpoint.connect_unapply({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.header_stack.set_visible(true);
                    this.bottom_bar.set_reveal(false);

                    this.services_page.expand();

                    this.split_view.set_collapsed(this.should_hide_sidebar());
                }
            });

            self.obj().connect_performance_page_active_notify({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    if this.performance_page_active.get() {
                        let should_hide_sidebar = this.should_hide_sidebar();
                        this.split_view.set_show_sidebar(!should_hide_sidebar);
                        this.split_view.set_collapsed(should_hide_sidebar);
                    } else {
                        this.split_view.set_show_sidebar(false);
                        this.split_view.set_collapsed(true);
                    }
                }
            });

            self.obj().connect_summary_mode_notify({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    if this.summary_mode.get() {
                        this.split_view.set_show_sidebar(false);
                    } else if !this.window_width_below_threshold() {
                        this.split_view.set_collapsed(false);
                        this.split_view
                            .set_show_sidebar(!this.user_hid_sidebar.get());
                    }
                }
            });

            self.performance_page.connect_info_button_visible_notify({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        this.notify_info_button_visible();
                    }
                }
            });
        }
    }

    impl WidgetImpl for MissionCenterWindow {
        fn realize(&self) {
            self.parent_realize();

            self.stack
                .set_visible_child_name(settings!().string("window-selected-page").as_str());
        }
    }

    impl WindowImpl for MissionCenterWindow {}

    impl ApplicationWindowImpl for MissionCenterWindow {}

    impl AdwApplicationWindowImpl for MissionCenterWindow {}
}

glib::wrapper! {
    pub struct MissionCenterWindow(ObjectSubclass<imp::MissionCenterWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MissionCenterWindow {
    pub fn new<P: IsA<gtk::Application>>(
        application: &P,
        settings: &gio::Settings,
        sys_info: &crate::sys_info_v2::SysInfoV2,
    ) -> Self {
        use gtk::glib::*;

        let this: Self = Object::builder()
            .property("application", application)
            .build();

        sys_info.set_update_speed(settings.uint64("app-update-interval-u64"));
        sys_info.set_core_count_affects_percentages(
            settings.boolean("apps-page-core-count-affects-percentages"),
        );

        settings.connect_changed(Some("app-update-interval-u64"), |settings, _| {
            let update_speed = settings.uint64("app-update-interval-u64");
            match app!().sys_info() {
                Ok(sys_info) => {
                    sys_info.set_update_speed(update_speed);
                }
                Err(e) => {
                    g_critical!(
                        "MissionCenter",
                        "Failed to get sys_info from MissionCenterApplication: {}",
                        e
                    );
                }
            };
        });

        this
    }

    pub fn set_initial_readings(&self, mut readings: Readings) {
        use gtk::glib::*;

        let ok = self.imp().performance_page.set_initial_readings(&readings);
        if !ok {
            g_critical!(
                "MissionCenter",
                "Failed to set initial readings for performance page"
            );
        }

        let ok = self.imp().apps_page.set_initial_readings(&mut readings);
        if !ok {
            g_critical!(
                "MissionCenter",
                "Failed to set initial readings for apps page"
            );
        }

        if readings.services.is_empty() {
            g_critical!("MissionCenter", "No services found, hiding services page");
            self.imp().services_stack_page.set_visible(false);
        } else {
            let ok = self.imp().services_page.set_initial_readings(&mut readings);
            if !ok {
                g_critical!(
                    "MissionCenter",
                    "Failed to set initial readings for services page"
                );
            }
        }

        self.imp().loading_spinner.set_spinning(false);
        self.imp().loading_box.set_visible(false);
        self.imp().header_bar.set_visible(true);
        self.imp().stack.set_visible(true);

        self.imp().bottom_bar.set_visible(true);
        self.bind_property("summary-mode", &self.imp().bottom_bar.get(), "visible")
            .flags(BindingFlags::INVERT_BOOLEAN)
            .build();

        self.imp()
            .split_view
            .set_collapsed(self.imp().should_hide_sidebar());

        if let Ok(sys_info) = app!().sys_info() {
            sys_info.continue_reading();
        } else {
            g_critical!(
                "MissionCenter",
                "Failed to get sys_info from MissionCenterApplication"
            );
        }
    }

    pub fn update_readings(&self, readings: &mut Readings) -> bool {
        let mut result = true;

        result &= self.imp().performance_page.update_readings(readings);
        result &= self.imp().apps_page.update_readings(readings);
        result &= self.imp().services_page.update_readings(readings);

        result
    }
}
