/* performance_page/disk.rs
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

use std::cell::{Cell, OnceCell};

use adw::{self, subclass::prelude::*};
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use super::{widgets::GraphWidget, PageExt};
use crate::application::INTERVAL_STEP;
use crate::i18n::*;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageDisk)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk.ui")]
    pub struct PerformancePageDisk {
        #[template_child]
        pub disk_id: TemplateChild<gtk::Label>,
        #[template_child]
        pub model: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub max_y: TemplateChild<gtk::Label>,
        #[template_child]
        pub graph_max_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub disk_transfer_rate_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get = Self::name, set = Self::set_name, type = String)]
        name: Cell<String>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,

        #[property(get = Self::infobar_content, type = Option < gtk::Widget >)]
        pub infobar_content: OnceCell<gtk::Box>,

        pub active_time: OnceCell<gtk::Label>,
        pub avg_response_time: OnceCell<gtk::Label>,
        pub legend_read: OnceCell<gtk::Picture>,
        pub read_speed: OnceCell<gtk::Label>,
        pub legend_write: OnceCell<gtk::Picture>,
        pub write_speed: OnceCell<gtk::Label>,
        pub capacity: OnceCell<gtk::Label>,
        pub formatted: OnceCell<gtk::Label>,
        pub system_disk: OnceCell<gtk::Label>,
        pub disk_type: OnceCell<gtk::Label>,
    }

    impl Default for PerformancePageDisk {
        fn default() -> Self {
            Self {
                disk_id: Default::default(),
                model: Default::default(),
                usage_graph: Default::default(),
                max_y: Default::default(),
                graph_max_duration: Default::default(),
                disk_transfer_rate_graph: Default::default(),
                context_menu: Default::default(),

                name: Cell::new(String::new()),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                infobar_content: Default::default(),

                active_time: Default::default(),
                avg_response_time: Default::default(),
                legend_read: Default::default(),
                read_speed: Default::default(),
                legend_write: Default::default(),
                write_speed: Default::default(),
                capacity: Default::default(),
                formatted: Default::default(),
                system_disk: Default::default(),
                disk_type: Default::default(),
            }
        }
    }

    impl PerformancePageDisk {
        fn name(&self) -> String {
            unsafe { &*self.name.as_ptr() }.clone()
        }

        fn set_name(&self, name: String) {
            {
                let if_name = unsafe { &*self.name.as_ptr() };
                if if_name == &name {
                    return;
                }
            }

            self.name.replace(name);
        }

        fn infobar_content(&self) -> Option<gtk::Widget> {
            self.infobar_content.get().map(|ic| ic.clone().into())
        }
    }

    impl PerformancePageDisk {
        fn configure_actions(this: &super::PerformancePageDisk) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let action = gio::SimpleAction::new("copy", None);
            action.connect_activate({
                let this = this.downgrade();
                move |_, _| {
                    if let Some(this) = this.upgrade() {
                        let clipboard = this.clipboard();
                        clipboard.set_text(this.imp().data_summary().as_str());
                    }
                }
            });
            actions.add_action(&action);
        }

        fn configure_context_menu(this: &super::PerformancePageDisk) {
            let right_click_controller = gtk::GestureClick::new();
            right_click_controller.set_button(3); // Secondary click (AKA right click)
            right_click_controller.connect_released({
                let this = this.downgrade();
                move |_click, _n_press, x, y| {
                    if let Some(this) = this.upgrade() {
                        this.imp()
                            .context_menu
                            .set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                                x.round() as i32,
                                y.round() as i32,
                                1,
                                1,
                            )));
                        this.imp().context_menu.popup();
                    }
                }
            });
            this.add_controller(right_click_controller);
        }
    }

    impl PerformancePageDisk {
        pub fn set_static_information(
            this: &super::PerformancePageDisk,
            index: Option<i32>,
            disk: &crate::sys_info_v2::DiskInfo,
        ) -> bool {
            use crate::sys_info_v2::DiskType;

            let t = this.clone();
            this.imp()
                .usage_graph
                .connect_local("resize", true, move |_| {
                    let this = t.imp();

                    let width = this.usage_graph.width() as f32;
                    let height = this.usage_graph.height() as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }

                    this.usage_graph
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    this.disk_transfer_rate_graph
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                    None
                });

            let this = this.imp();

            if index.is_some() {
                this.disk_id.set_text(&i18n_f(
                    "Disk {} ({})",
                    &[&format!("{}", index.unwrap()), &disk.id],
                ));
            } else {
                this.disk_id.set_text(&i18n_f("Drive ({})", &[&disk.id]));
            }
            this.model.set_text(&disk.model);

            this.disk_transfer_rate_graph.set_dashed(1, true);
            this.disk_transfer_rate_graph.set_filled(1, false);

            if let Some(legend_read) = this.legend_read.get() {
                legend_read
                    .set_resource(Some("/io/missioncenter/MissionCenter/line-solid-disk.svg"));
            }
            if let Some(legend_write) = this.legend_write.get() {
                legend_write
                    .set_resource(Some("/io/missioncenter/MissionCenter/line-dashed-disk.svg"));
            }

            let cap = crate::to_human_readable(disk.capacity as f32, 1024.);
            if let Some(capacity) = this.capacity.get() {
                capacity.set_text(&format!(
                    "{:.2} {}{}B",
                    cap.0,
                    cap.1,
                    if cap.1.is_empty() { "" } else { "i" }
                ));
            }

            let fmt = crate::to_human_readable(disk.formatted as f32, 1024.);
            if let Some(formatted) = this.formatted.get() {
                formatted.set_text(&format!(
                    "{:.2} {}{}B",
                    fmt.0,
                    fmt.1,
                    if fmt.1.is_empty() { "" } else { "i" }
                ));
            }

            let is_system_disk = if disk.system_disk {
                i18n("Yes")
            } else {
                i18n("No")
            };
            if let Some(system_disk) = this.system_disk.get() {
                system_disk.set_text(&is_system_disk);
            }

            if let Some(disk_type) = this.disk_type.get() {
                disk_type.set_text(match disk.r#type {
                    DiskType::HDD => "HDD",
                    DiskType::SSD => "SSD",
                    DiskType::NVMe => "NVMe",
                    DiskType::eMMC => "eMMC",
                    DiskType::SD => "SD",
                    DiskType::iSCSI => "iSCSI",
                    DiskType::Optical => "Optical",
                    DiskType::Unknown => "Unknown",
                });
            }
            true
        }

        pub fn update_readings(
            this: &super::PerformancePageDisk,
            index: Option<usize>,
            disk: &crate::sys_info_v2::DiskInfo,
        ) -> bool {
            let this = this.imp();

            if index.is_some() {
                this.disk_id.set_text(&i18n_f(
                    "Drive {} ({})",
                    &[&format!("{}", index.unwrap()), &disk.id],
                ));
            } else {
                this.disk_id.set_text(&i18n_f("Drive ({})", &[&disk.id]));
            }

            let max_y =
                crate::to_human_readable(this.disk_transfer_rate_graph.value_range_max(), 1024.);
            let i = if max_y.1.is_empty() { "" } else { "i" };
            this.max_y.set_text(&i18n_f(
                "{} {}{}B/s",
                &[
                    &format!("{}", max_y.0.round()),
                    &format!("{}", max_y.1),
                    &format!("{}", i),
                ],
            ));

            this.usage_graph.add_data_point(0, disk.busy_percent);

            if let Some(active_time) = this.active_time.get() {
                active_time.set_text(&format!("{}%", disk.busy_percent.round() as u8));
            }

            if let Some(avg_response_time) = this.avg_response_time.get() {
                avg_response_time.set_text(&format!("{:.2} ms", disk.response_time_ms));
            }

            this.disk_transfer_rate_graph
                .add_data_point(0, disk.read_speed as f32);
            let rsp = crate::to_human_readable(disk.read_speed as f32, 1024.);
            let i = if rsp.1.is_empty() { "" } else { "i" };
            if let Some(read_speed) = this.read_speed.get() {
                read_speed.set_text(&format!("{0:.2$} {1}{3}B/s", rsp.0, rsp.1, rsp.2, i,));
            }

            this.disk_transfer_rate_graph
                .add_data_point(1, disk.write_speed as f32);
            let wsp = crate::to_human_readable(disk.write_speed as f32, 1024.);
            let i = if wsp.1.is_empty() { "" } else { "i" };
            if let Some(write_speed) = this.write_speed.get() {
                write_speed.set_text(&format!("{0:.2$} {1}{3}B/s", wsp.0, wsp.1, wsp.2, i,));
            }

            true
        }

        fn data_summary(&self) -> String {
            let unknown = i18n("Unknown");
            let unknown = unknown.as_str();

            format!(
                r#"{}

    {}

    Capacity:    {}
    Formatted:   {}
    System disk: {}
    Type:        {}

    Read speed:            {}
    Write speed:           {}
    Active time:           {}
    Average response time: {}"#,
                self.disk_id.label(),
                self.model.label(),
                self.capacity
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.formatted
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.system_disk
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.disk_type
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.read_speed
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.write_speed
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.active_time
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.avg_response_time
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
            )
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageDisk {
        const NAME: &'static str = "PerformancePageDisk";
        type Type = super::PerformancePageDisk;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageDisk {
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
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageDisk>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);

            let sidebar_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/disk_details.ui",
            );

            let _ = self.infobar_content.set(
                sidebar_content_builder
                    .object::<gtk::Box>("root")
                    .expect("Could not find `root` object in details pane"),
            );

            let _ = self.active_time.set(
                sidebar_content_builder
                    .object::<gtk::Label>("active_time")
                    .expect("Could not find `active_time` object in details pane"),
            );
            let _ = self.avg_response_time.set(
                sidebar_content_builder
                    .object::<gtk::Label>("avg_response_time")
                    .expect("Could not find `avg_response_time` object in details pane"),
            );
            let _ = self.legend_read.set(
                sidebar_content_builder
                    .object::<gtk::Picture>("legend_read")
                    .expect("Could not find `legend_read` object in details pane"),
            );
            let _ = self.read_speed.set(
                sidebar_content_builder
                    .object::<gtk::Label>("read_speed")
                    .expect("Could not find `read_speed` object in details pane"),
            );
            let _ = self.legend_write.set(
                sidebar_content_builder
                    .object::<gtk::Picture>("legend_write")
                    .expect("Could not find `legend_write` object in details pane"),
            );
            let _ = self.write_speed.set(
                sidebar_content_builder
                    .object::<gtk::Label>("write_speed")
                    .expect("Could not find `write_speed` object in details pane"),
            );
            let _ = self.capacity.set(
                sidebar_content_builder
                    .object::<gtk::Label>("capacity")
                    .expect("Could not find `capacity` object in details pane"),
            );
            let _ = self.formatted.set(
                sidebar_content_builder
                    .object::<gtk::Label>("formatted")
                    .expect("Could not find `formatted` object in details pane"),
            );
            let _ = self.system_disk.set(
                sidebar_content_builder
                    .object::<gtk::Label>("system_disk")
                    .expect("Could not find `system_disk` object in details pane"),
            );
            let _ = self.disk_type.set(
                sidebar_content_builder
                    .object::<gtk::Label>("disk_type")
                    .expect("Could not find `disk_type` object in details pane"),
            );
        }
    }

    impl WidgetImpl for PerformancePageDisk {}

    impl BoxImpl for PerformancePageDisk {}
}

glib::wrapper! {
    pub struct PerformancePageDisk(ObjectSubclass<imp::PerformancePageDisk>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PageExt for PerformancePageDisk {
    fn infobar_collapsed(&self) {
        self.imp()
            .infobar_content
            .get()
            .and_then(|ic| Some(ic.set_margin_top(10)));
    }

    fn infobar_uncollapsed(&self) {
        self.imp()
            .infobar_content
            .get()
            .and_then(|ic| Some(ic.set_margin_top(65)));
    }
}

impl PerformancePageDisk {
    pub fn new(name: &str, settings: &gio::Settings) -> Self {
        let this: Self = glib::Object::builder().property("name", name).build();

        fn update_refresh_rate_sensitive_labels(
            this: &PerformancePageDisk,
            settings: &gio::Settings,
        ) {
            let data_points = settings.int("performance-page-data-points") as u32;
            let smooth = settings.boolean("performance-smooth-graphs");
            let graph_max_duration = (((settings.uint64("app-update-interval-u64") as f64)
                * INTERVAL_STEP)
                * (data_points as f64))
                .round() as u32;

            let this = this.imp();

            let mins = graph_max_duration / 60;
            let seconds_to_string = format!(
                "{} second{}",
                graph_max_duration % 60,
                if (graph_max_duration % 60) != 1 {
                    "s"
                } else {
                    ""
                }
            );
            let mins_to_string = format!("{:} minute{} ", mins, if mins > 1 { "s" } else { "" });
            this.graph_max_duration.set_text(&*format!(
                "{}{}",
                if mins > 0 {
                    mins_to_string
                } else {
                    "".to_string()
                },
                if graph_max_duration % 60 > 0 {
                    seconds_to_string
                } else {
                    "".to_string()
                }
            ));
            this.usage_graph.set_data_points(data_points);
            this.usage_graph.set_smooth_graphs(smooth);
            this.disk_transfer_rate_graph.set_data_points(data_points);
            this.disk_transfer_rate_graph.set_smooth_graphs(smooth);
        }
        update_refresh_rate_sensitive_labels(&this, settings);

        settings.connect_changed(Some("performance-page-data-points"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    update_refresh_rate_sensitive_labels(&this, settings);
                }
            }
        });

        settings.connect_changed(Some("app-update-interval-u64"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    update_refresh_rate_sensitive_labels(&this, settings);
                }
            }
        });

        settings.connect_changed(Some("performance-smooth-graphs"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    update_refresh_rate_sensitive_labels(&this, settings);
                }
            }
        });

        this
    }

    pub fn set_static_information(
        &self,
        index: Option<i32>,
        disk: &crate::sys_info_v2::DiskInfo,
    ) -> bool {
        imp::PerformancePageDisk::set_static_information(self, index, disk)
    }

    pub fn update_readings(
        &self,
        index: Option<usize>,
        disk: &crate::sys_info_v2::DiskInfo,
    ) -> bool {
        imp::PerformancePageDisk::update_readings(self, index, disk)
    }
}
