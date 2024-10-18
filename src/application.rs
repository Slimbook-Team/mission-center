/* application.rs
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

use std::cell::{BorrowError, Cell, Ref, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::{
    gio,
    glib::{self, g_critical, property::PropertySet},
};

use crate::{config::VERSION, i18n::i18n, sys_info_v2::Readings};

pub const INTERVAL_STEP: f64 = 0.05;
pub const BASE_INTERVAL: f64 = 1f64;

#[macro_export]
macro_rules! app {
    () => {{
        use ::gtk::glib::object::Cast;
        ::gtk::gio::Application::default()
            .and_then(|app| app.downcast::<$crate::MissionCenterApplication>().ok())
            .expect("Failed to get MissionCenterApplication instance")
    }};
}

#[macro_export]
macro_rules! settings {
    () => {
        $crate::app!().settings()
    };
}

mod imp {
    use super::*;

    pub struct MissionCenterApplication {
        pub settings: Cell<Option<gio::Settings>>,
        pub sys_info: RefCell<Option<crate::sys_info_v2::SysInfoV2>>,
        pub window: RefCell<Option<crate::MissionCenterWindow>>,
    }

    impl Default for MissionCenterApplication {
        fn default() -> Self {
            Self {
                settings: Cell::new(None),
                sys_info: RefCell::new(None),
                window: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MissionCenterApplication {
        const NAME: &'static str = "MissioncenterApplication";
        type Type = super::MissionCenterApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for MissionCenterApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.set_default();

            self.settings
                .set(Some(gio::Settings::new("io.missioncenter.MissionCenter")));

            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for MissionCenterApplication {
        fn activate(&self) {
            use gtk::glib::*;

            let application = self.obj();
            // Get the current window or create one if necessary
            let window = if let Some(window) = application.window() {
                window
            } else {
                let settings = unsafe { self.settings.take().unwrap_unchecked() };
                self.settings.set(Some(settings.clone()));

                let sys_info = crate::sys_info_v2::SysInfoV2::new();

                let window = crate::MissionCenterWindow::new(&*application, &settings, &sys_info);

                window.connect_default_height_notify({
                    move |window| {
                        let settings = settings!();
                        settings
                            .set_int("window-height", window.default_height())
                            .unwrap_or_else(|err| {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to save window height: {}",
                                    err
                                );
                            });
                    }
                });
                window.connect_default_width_notify({
                    move |window| {
                        let settings = settings!();
                        settings
                            .set_int("window-width", window.default_width())
                            .unwrap_or_else(|err| {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to save window width: {}",
                                    err
                                );
                            });
                    }
                });

                window
                    .set_default_size(settings.int("window-width"), settings.int("window-height"));

                sys_info.set_core_count_affects_percentages(
                    settings.boolean("apps-page-core-count-affects-percentages"),
                );

                settings.connect_changed(
                    Some("apps-page-core-count-affects-percentages"),
                    move |settings, _| {
                        let app = app!();
                        match app.sys_info() {
                            Ok(sys_info) => {
                                sys_info.set_core_count_affects_percentages(
                                    settings.boolean("apps-page-core-count-affects-percentages"),
                                );
                            }
                            Err(e) => {
                                g_critical!(
                                    "MissionCenter",
                                    "Failed to get sys_info from MissionCenterApplication: {}",
                                    e
                                );
                            }
                        };
                    },
                );

                self.sys_info.set(Some(sys_info));

                let provider = gtk::CssProvider::new();
                provider.load_from_bytes(&Bytes::from_static(include_bytes!(
                    "../resources/ui/style.css"
                )));

                gtk::style_context_add_provider_for_display(
                    &gtk::gdk::Display::default().expect("Could not connect to a display."),
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );

                window.upcast()
            };

            window.present();

            self.window
                .set(window.downcast_ref::<crate::MissionCenterWindow>().cloned());
        }
    }

    impl GtkApplicationImpl for MissionCenterApplication {}

    impl AdwApplicationImpl for MissionCenterApplication {}
}

glib::wrapper! {
    pub struct MissionCenterApplication(ObjectSubclass<imp::MissionCenterApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl MissionCenterApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        use glib::g_message;

        let this: Self = glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build();

        g_message!(
            "MissionCenter::Application",
            "Starting Mission Center v{}",
            env!("CARGO_PKG_VERSION")
        );

        this
    }

    pub fn set_initial_readings(&self, readings: Readings) {
        use gtk::glib::*;

        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return;
        };

        window.set_initial_readings(readings)
    }

    pub fn refresh_readings(&self, readings: &mut Readings) -> bool {
        use gtk::glib::*;

        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to refresh data"
            );
            return false;
        };

        window.update_readings(readings)
    }

    pub fn settings(&self) -> gio::Settings {
        unsafe { (&*self.imp().settings.as_ptr()).as_ref().unwrap_unchecked() }.clone()
    }

    pub fn sys_info(&self) -> Result<Ref<crate::sys_info_v2::SysInfoV2>, BorrowError> {
        match self.imp().sys_info.try_borrow() {
            Ok(sys_info_ref) => Ok(Ref::map(sys_info_ref, |sys_info_opt| match sys_info_opt {
                Some(sys_info) => sys_info,
                None => {
                    panic!("MissionCenter::Application::sys_info() called before sys_info was initialized");
                }
            })),
            Err(e) => Err(e),
        }
    }

    pub fn window(&self) -> Option<crate::MissionCenterWindow> {
        unsafe { &*self.imp().window.as_ptr() }.clone()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| {
                app.show_preferences();
            })
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        self.add_action_entries([quit_action, preferences_action, about_action]);
    }

    fn show_preferences(&self) {
        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to show preferences"
            );
            return;
        };

        let preferences = crate::preferences::PreferencesDialog::new();
        preferences.present(Some(&window));
    }

    fn show_about(&self) {
        let Some(window) = self.window() else {
            g_critical!(
                "MissionCenter::Application",
                "No active window, when trying to show about dialog"
            );
            return;
        };

        let about = adw::AboutDialog::builder()
            .application_name("Mission Center")
            .application_icon("io.missioncenter.MissionCenter")
            .developer_name("Mission Center Developers")
            .developers(["Romeo Calota", "QwertyChouskie", "jojo2357", "Jan Luca"])
            .translator_credits(i18n("translator-credits"))
            .version(VERSION)
            .issue_url("https://gitlab.com/mission-center-devs/mission-center/-/issues")
            .copyright("© 2024 Mission Center Developers")
            .license_type(gtk::License::Gpl30)
            .website("https://missioncenter.io")
            .release_notes(
                r#"<ul><li>Add a new Fan Page that monitors system fans and reports RPM, PWM and temperature information</li>
<li>Add support for hiding and rearranging devices in the Performance sidebar</li>
<li>Overhaul the Memory Page to convey more information</li>
<li>Update GPU page UI to better reflect what aspects of the GPU can be monitored</li>
<li>Add GTT information to the GPU page (AMD only)</li>
<li>Add network total data transfer information to the Network Page</li>
<li>Show application icons for processes that belong to a known application</li>
<li>Switch to AdwDialog for the About and Preferences dialogs</li>
<li>Update NVTOP for better GPU support</li>
<li>Initial support for Snap</li>
<li>Update to latest GNOME (47) Platform for all supported packaging formats</li>
<li>Add support for `zenpower` when monitoring AMD CPU temperature</li>
<li>Fix a seeming CPU usage spike when the app starts</li>
<li>Improved app detection</li>
<li>Supress some network device related errors that were flooding the SystemD journal</li>
<li>Hide CPU frequency governor and power preference information when not supported</li>
<li>Update CPU frequency governor and power preference information while the app is running</li>
<li>Add an option to use bytes instead of bits for network data transfer information</li>
<li>Add support for more device types in the Network Page</li>
<li>Detect SystemD more reliably using D-Bus instead of searching for a library on disk</li>
<li>Fix a memory leak that occurred when filtering apps and processes</li>
<li>Clean up graph labels to be more consistent across different pages</li></ul>"#,
            )
            .build();

        about.add_credit_section(
            Some("Standing on the shoulders of giants"),
            &[
                "GTK https://www.gtk.org/",
                "GNOME https://www.gnome.org/",
                "Libadwaita https://gitlab.gnome.org/GNOME/libadwaita",
                "Blueprint Compiler https://jwestman.pages.gitlab.gnome.org/blueprint-compiler/",
                "NVTOP https://github.com/Syllo/nvtop",
                "Workbench https://github.com/sonnyp/Workbench",
                "And many more... Thank you all!",
            ],
        );

        about.present(Some(&window));
    }
}
