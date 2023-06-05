/* performance_page/memory.rs
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
use gtk::{gio, glib, prelude::*};

use super::widgets::{GraphWidget, MemoryCompositionWidget};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageMemory)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/memory.ui")]
    pub struct PerformancePageMemory {
        #[template_child]
        pub admin_banner: TemplateChild<adw::Banner>,
        #[template_child]
        pub total_ram: TemplateChild<gtk::Label>,
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub mem_composition: TemplateChild<MemoryCompositionWidget>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub in_use: TemplateChild<gtk::Label>,
        #[template_child]
        pub available: TemplateChild<gtk::Label>,
        #[template_child]
        pub committed: TemplateChild<gtk::Label>,
        #[template_child]
        pub cached: TemplateChild<gtk::Label>,
        #[template_child]
        pub swap_available: TemplateChild<gtk::Label>,
        #[template_child]
        pub swap_used: TemplateChild<gtk::Label>,
        #[template_child]
        pub system_info: TemplateChild<gtk::Box>,
        #[template_child]
        pub speed: TemplateChild<gtk::Label>,
        #[template_child]
        pub slots_used: TemplateChild<gtk::Label>,
        #[template_child]
        pub form_factor: TemplateChild<gtk::Label>,
        #[template_child]
        pub ram_type: TemplateChild<gtk::Label>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,

        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
    }

    impl Default for PerformancePageMemory {
        fn default() -> Self {
            Self {
                admin_banner: Default::default(),
                total_ram: Default::default(),
                usage_graph: Default::default(),
                mem_composition: Default::default(),
                toast_overlay: Default::default(),
                in_use: Default::default(),
                available: Default::default(),
                committed: Default::default(),
                cached: Default::default(),
                swap_available: Default::default(),
                swap_used: Default::default(),
                system_info: Default::default(),
                speed: Default::default(),
                slots_used: Default::default(),
                form_factor: Default::default(),
                ram_type: Default::default(),
                context_menu: Default::default(),

                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),
            }
        }
    }

    impl PerformancePageMemory {}

    impl PerformancePageMemory {
        fn configure_actions(this: &super::PerformancePageMemory) {
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));
        }

        fn configure_context_menu(this: &super::PerformancePageMemory) {
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
    }

    impl PerformancePageMemory {
        pub fn set_static_information(
            this: &super::PerformancePageMemory,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let this = this.imp();

            this.usage_graph
                .set_value_range_max(readings.mem_info.mem_total as f32);
            this.usage_graph
                .connect_resize(|graph_widget, width, height| {
                    let width = width as f32;
                    let height = height as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }

                    graph_widget
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
                });

            let total_mem = crate::to_human_readable(readings.mem_info.mem_total as _, 1024.);
            this.total_ram
                .set_text(&format!("{:.2} {}iB", total_mem.0.round(), total_mem.1));

            true
        }

        pub fn update_readings(
            this: &super::PerformancePageMemory,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let this = this.imp();
            let mem_info = &readings.mem_info;

            {
                let used = mem_info.mem_total - mem_info.mem_available;
                this.usage_graph.add_data_point(0, used as _);

                this.mem_composition.update_memory_information(mem_info);

                let used = crate::to_human_readable(used as _, 1024.);
                this.in_use.set_text(&format!("{:.2} {}iB", used.0, used.1));

                let available = crate::to_human_readable(mem_info.mem_available as _, 1024.);
                this.available
                    .set_text(&format!("{:.2} {}iB", available.0, available.1));

                let committed = crate::to_human_readable(mem_info.committed_as as _, 1024.);
                this.committed
                    .set_text(&format!("{:.2} {}iB", committed.0, committed.1));

                let cached = crate::to_human_readable(mem_info.cached as _, 1024.);
                this.cached
                    .set_text(&format!("{:.2} {}iB", cached.0, cached.1));

                let swap_available = crate::to_human_readable(mem_info.swap_total as _, 1024.);
                this.swap_available
                    .set_text(&format!("{:.2} {}iB", swap_available.0, swap_available.1));

                let swap_used = crate::to_human_readable(
                    (mem_info.swap_total - mem_info.swap_free) as _,
                    1024.,
                );
                this.swap_used
                    .set_text(&format!("{:.2} {}iB", swap_used.0, swap_used.1));
            }

            true
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageMemory {
        const NAME: &'static str = "PerformancePageMemory";
        type Type = super::PerformancePageMemory;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            MemoryCompositionWidget::ensure_type();
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageMemory {
        fn constructed(&self) {
            use gettextrs::*;

            self.parent_constructed();

            let obj = self.obj();
            let this = obj.upcast_ref::<super::PerformancePageMemory>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);

            self.admin_banner
                .connect_button_clicked(clone!(@weak this => move |_| {
                    let ptr = this.as_ptr() as usize;
                    let _ = std::thread::spawn(move || {
                        use crate::sys_info_v2::MemInfo;

                        let memory_device_info = MemInfo::load_memory_device_info();
                        glib::idle_add_once(move || {
                            use glib::translate::from_glib_none;

                            let this: gtk::Widget = unsafe { from_glib_none(ptr as *mut gtk::ffi::GtkWidget) };
                            let this = this.downcast_ref::<super::PerformancePageMemory>().unwrap();

                            match memory_device_info {
                                Some(memory_device_info) => {
                                    this.imp().admin_banner.set_revealed(false);
                                    this.imp().system_info.set_visible(true);

                                    let mem_module_count = memory_device_info.len();
                                    if mem_module_count > 0 {
                                        this.imp().speed.set_text(&format!("{} MT/s", memory_device_info[0].speed));
                                        this.imp().slots_used.set_text(&format!("{}", mem_module_count));
                                        this.imp().form_factor.set_text(&format!("{}", memory_device_info[0].form_factor));
                                        this.imp().ram_type.set_text(&format!("{}", memory_device_info[0].ram_type));
                                    } else {
                                        let unknown = gettext("Unknown");
                                        this.imp().speed.set_text(&unknown);
                                        this.imp().slots_used.set_text(&unknown);
                                        this.imp().form_factor.set_text(&unknown);
                                        this.imp().ram_type.set_text(&unknown);
                                    }
                                }
                                _ => this.imp().toast_overlay.add_toast(adw::Toast::new("Authentication failed"))
                            }
                        });
                    });
                }));
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

    impl WidgetImpl for PerformancePageMemory {
        fn realize(&self) {
            self.parent_realize();

            let this = self
                .obj()
                .upcast_ref::<super::PerformancePageMemory>()
                .clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                this.imp().admin_banner.set_revealed(true);
            });
        }
    }

    impl BoxImpl for PerformancePageMemory {}
}

glib::wrapper! {
    pub struct PerformancePageMemory(ObjectSubclass<imp::PerformancePageMemory>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePageMemory {
    pub fn new() -> Self {
        let this: Self = unsafe {
            glib::Object::new_internal(Self::static_type(), &mut [])
                .downcast()
                .unwrap()
        };

        this
    }

    pub fn set_static_information(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePageMemory::set_static_information(self, readings)
    }

    pub fn update_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePageMemory::update_readings(self, readings)
    }
}
