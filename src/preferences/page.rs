/* preferences/page.rs
 *
 * Copyright 2023 Romeo Calota
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

use adw::{prelude::*, subclass::prelude::*, SpinRow, SwitchRow};
use gtk::{gio, glib, Scale};

use crate::settings;

const MAX_INTERVAL_TICKS: u64 = 200;
const MIN_INTERVAL_TICKS: u64 = 10;

const MAX_POINTS: i32 = 600;
const MIN_POINTS: i32 = 10;

macro_rules! connect_switch_to_setting {
    ($this: expr, $switch_row: expr, $setting: literal) => {
        $switch_row.connect_active_notify({
            move |switch_row| {
                if let Err(e) = settings!().set_boolean($setting, switch_row.is_active()) {
                    gtk::glib::g_critical!(
                        "MissionCenter::Preferences",
                        "Failed to set {} setting: {}",
                        $setting,
                        e
                    );
                }
            }
        });
    };
}

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/preferences/page.ui")]
    pub struct PreferencesPage {
        #[template_child]
        pub update_interval: TemplateChild<SpinRow>,
        #[template_child]
        pub data_points: TemplateChild<Scale>,

        #[template_child]
        pub smooth_graphs: TemplateChild<SwitchRow>,
        #[template_child]
        pub network_bytes: TemplateChild<SwitchRow>,
        #[template_child]
        pub network_dynamic_scaling: TemplateChild<SwitchRow>,
        #[template_child]
        pub show_cpu: TemplateChild<SwitchRow>,
        #[template_child]
        pub show_memory: TemplateChild<SwitchRow>,
        #[template_child]
        pub show_disks: TemplateChild<SwitchRow>,
        #[template_child]
        pub show_network: TemplateChild<SwitchRow>,
        #[template_child]
        pub show_gpus: TemplateChild<SwitchRow>,
        #[template_child]
        pub show_fans: TemplateChild<SwitchRow>,

        #[template_child]
        pub merged_process_stats: TemplateChild<SwitchRow>,
        #[template_child]
        pub remember_sorting: TemplateChild<SwitchRow>,
        #[template_child]
        pub core_count_affects_percentages: TemplateChild<SwitchRow>,
    }

    impl Default for PreferencesPage {
        fn default() -> Self {
            Self {
                update_interval: Default::default(),
                data_points: Default::default(),

                smooth_graphs: Default::default(),
                network_bytes: Default::default(),
                network_dynamic_scaling: Default::default(),
                show_cpu: Default::default(),
                show_memory: Default::default(),
                show_disks: Default::default(),
                show_network: Default::default(),
                show_gpus: Default::default(),
                show_fans: Default::default(),

                merged_process_stats: Default::default(),
                remember_sorting: Default::default(),
                core_count_affects_percentages: Default::default(),
            }
        }
    }

    impl PreferencesPage {
        pub fn configure_update_speed(&self) {
            use crate::application::INTERVAL_STEP;
            use glib::g_critical;

            let settings = settings!();

            let new_interval = (self.update_interval.value() / INTERVAL_STEP).round() as u64;
            let new_points = self.data_points.value() as i32;

            if new_interval <= MAX_INTERVAL_TICKS && new_interval >= MIN_INTERVAL_TICKS {
                if settings
                    .set_uint64("app-update-interval-u64", new_interval)
                    .is_err()
                {
                    g_critical!(
                        "MissionCenter::Preferences",
                        "Failed to set update interval setting",
                    );
                }
            } else {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Update interval out of bounds",
                );
            }

            if new_points <= MAX_POINTS && new_points >= MIN_POINTS {
                if settings
                    .set_int("performance-page-data-points", new_points)
                    .is_err()
                {
                    g_critical!(
                        "MissionCenter::Preferences",
                        "Failed to set update points setting",
                    );
                }
            } else {
                g_critical!(
                    "MissionCenter::Preferences",
                    "Points interval out of bounds",
                );
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesPage {
        const NAME: &'static str = "PreferencesPage";
        type Type = super::PreferencesPage;
        type ParentType = adw::PreferencesPage;

        fn class_init(klass: &mut Self::Class) {
            SwitchRow::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PreferencesPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.data_points
                .downcast_ref::<Scale>()
                .unwrap()
                .connect_value_changed({
                    let this = self.obj().downgrade();
                    move |_| {
                        if let Some(this) = this.upgrade() {
                            this.imp().configure_update_speed();
                        }
                    }
                });

            self.update_interval
                .downcast_ref::<SpinRow>()
                .unwrap()
                .connect_changed({
                    let this = self.obj().downgrade();
                    move |_| {
                        if let Some(this) = this.upgrade() {
                            this.imp().configure_update_speed();
                        }
                    }
                });

            connect_switch_to_setting!(self, self.smooth_graphs, "performance-smooth-graphs");
            connect_switch_to_setting!(
                self,
                self.network_bytes,
                "performance-page-network-use-bytes"
            );
            connect_switch_to_setting!(
                self,
                self.network_dynamic_scaling,
                "performance-page-network-dynamic-scaling"
            );
            connect_switch_to_setting!(self, self.show_cpu, "performance-show-cpu");
            connect_switch_to_setting!(self, self.show_memory, "performance-show-memory");
            connect_switch_to_setting!(self, self.show_disks, "performance-show-disks");
            connect_switch_to_setting!(self, self.show_network, "performance-show-network");
            connect_switch_to_setting!(self, self.show_gpus, "performance-show-gpus");
            connect_switch_to_setting!(self, self.show_fans, "performance-show-fans");

            connect_switch_to_setting!(
                self,
                self.merged_process_stats,
                "apps-page-merged-process-stats"
            );
            connect_switch_to_setting!(self, self.remember_sorting, "apps-page-remember-sorting");
            connect_switch_to_setting!(
                self,
                self.core_count_affects_percentages,
                "apps-page-core-count-affects-percentages"
            );
        }
    }

    impl WidgetImpl for PreferencesPage {}

    impl PreferencesPageImpl for PreferencesPage {}
}

glib::wrapper! {
    pub struct PreferencesPage(ObjectSubclass<imp::PreferencesPage>)
        @extends adw::PreferencesPage, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PreferencesPage {
    pub fn new() -> Self {
        let this: Self = glib::Object::builder().build();

        this.set_initial_update_speed();

        let imp = this.imp();
        let settings = settings!();

        imp.smooth_graphs
            .set_active(settings.boolean("performance-smooth-graphs"));
        imp.network_bytes
            .set_active(settings.boolean("performance-page-network-use-bytes"));
        imp.network_dynamic_scaling
            .set_active(settings.boolean("performance-page-network-dynamic-scaling"));
        imp.show_cpu
            .set_active(settings.boolean("performance-show-cpu"));
        imp.show_memory
            .set_active(settings.boolean("performance-show-memory"));
        imp.show_disks
            .set_active(settings.boolean("performance-show-disks"));
        imp.show_network
            .set_active(settings.boolean("performance-show-network"));
        imp.show_gpus
            .set_active(settings.boolean("performance-show-gpus"));
        imp.show_fans
            .set_active(settings.boolean("performance-show-fans"));

        imp.merged_process_stats
            .set_active(settings.boolean("apps-page-merged-process-stats"));
        imp.remember_sorting
            .set_active(settings.boolean("apps-page-remember-sorting"));
        imp.core_count_affects_percentages
            .set_active(settings.boolean("apps-page-core-count-affects-percentages"));

        this
    }

    fn set_initial_update_speed(&self) {
        use crate::application::INTERVAL_STEP;

        let settings = settings!();

        let data_points = settings.int("performance-page-data-points");
        let update_interval_s = (settings.uint64("app-update-interval-u64") as f64) * INTERVAL_STEP;
        let this = self.imp();

        this.data_points.set_value(data_points as f64);
        this.update_interval.set_value(update_interval_s);
    }
}
