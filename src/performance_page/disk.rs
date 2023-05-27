/* performance_page/disk.rs
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

use std::cell::Cell;

use adw;
use adw::subclass::prelude::*;
use glib::{clone, ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*, Snapshot};

use super::widgets::GraphWidget;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageDisk)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/disk.ui")]
    pub struct PerformancePageDisk {
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub disk_transfer_rate_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub capacity: TemplateChild<gtk::Label>,
        #[template_child]
        pub formatted: TemplateChild<gtk::Label>,
        #[template_child]
        pub system_disk: TemplateChild<gtk::Label>,
        #[template_child]
        pub disk_type: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get = Self::name, set = Self::set_name, type = String)]
        name: Cell<String>,
        #[property(get, set)]
        refresh_interval: Cell<u32>,
        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
    }

    impl Default for PerformancePageDisk {
        fn default() -> Self {
            Self {
                usage_graph: Default::default(),
                disk_transfer_rate_graph: Default::default(),
                capacity: Default::default(),
                formatted: Default::default(),
                system_disk: Default::default(),
                disk_type: Default::default(),
                context_menu: Default::default(),

                name: Cell::new(String::new()),
                refresh_interval: Cell::new(1000),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),
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
            self.update_static_information();
        }
    }

    impl PerformancePageDisk {
        fn configure_actions(this: &super::PerformancePageDisk) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));
        }

        fn configure_context_menu(this: &super::PerformancePageDisk) {
            let right_click_controller = gtk::GestureClick::new();
            right_click_controller.set_button(3); // Secondary click (AKA right click)
            right_click_controller.connect_released(
                clone!(@weak this => move |_click, _n_press, x, y| {
                    this
                        .imp()
                        .context_menu
                        .set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            x.round() as i32,
                            y.round() as i32,
                            1,
                            1,
                        )));
                    this.imp().context_menu.popup();
                }),
            );
            this.add_controller(right_click_controller);
        }

        fn update_view(this: &super::PerformancePageDisk) {
            use crate::SYS_INFO;

            let this = this.clone();
            let sys_info = SYS_INFO.read().expect("Failed to acquire read lock");

            Some(glib::source::timeout_add_local_once(
                std::time::Duration::from_millis(this.refresh_interval() as _),
                move || {
                    Self::update_view(&this);
                },
            ));
        }

        fn update_static_information(&self) {
            use crate::{sys_info::*, SYS_INFO};

            let sys_info = SYS_INFO.read().expect("Failed to acquire read lock");
            let disk_info = sys_info.disk_info();
            if let Some(disk) = disk_info
                .disks()
                .iter()
                .filter(|d| d.name == self.obj().upcast_ref::<super::PerformancePageDisk>().name())
                .take(1)
                .next()
            {
                use gettextrs::gettext;

                let capacity = crate::to_human_readable(disk.capacity as f32, 1024.);
                self.capacity
                    .set_text(&format!("{:.2} {}iB", capacity.0, capacity.1));

                let formatted = crate::to_human_readable(disk.formatted as f32, 1024.);
                self.formatted
                    .set_text(&format!("{:.2} {}iB", formatted.0, formatted.1));

                let is_system_disk = if disk.system_disk {
                    gettext("Yes")
                } else {
                    gettext("No")
                };
                self.system_disk.set_text(&is_system_disk);

                self.disk_type.set_text(match disk.r#type {
                    DiskType::HDD => "HDD",
                    DiskType::SSD => "SSD",
                    DiskType::NVMe => "NVMe",
                    DiskType::iSCSI => "iSCSI",
                    DiskType::Unknown => "Unknown",
                });
            }
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
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageDisk>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);
            self.update_static_information();
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for PerformancePageDisk {
        fn realize(&self) {
            self.parent_realize();

            Self::update_view(self.obj().upcast_ref());
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            self.parent_snapshot(snapshot);

            let graph_width = self.obj().allocated_width() as u32;
            self.usage_graph.set_vertical_line_count(graph_width / 50);
            self.disk_transfer_rate_graph
                .set_vertical_line_count(graph_width / 40);
            self.disk_transfer_rate_graph
                .set_horizontal_line_count(graph_width / 70);
        }
    }

    impl BoxImpl for PerformancePageDisk {}
}

glib::wrapper! {
    pub struct PerformancePageDisk(ObjectSubclass<imp::PerformancePageDisk>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageDisk {
    pub fn new(name: &str) -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [("name", name.into())])
                .downcast()
                .unwrap()
        };

        this
    }

    pub fn set_initial_values(&self, values: Vec<f32>) {
        self.imp().usage_graph.set_data(0, values);
    }
}
