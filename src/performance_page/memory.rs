/* performance_page/memory.rs
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

use super::{
    widgets::{GraphWidget, MemoryCompositionWidget},
    PageExt,
};
use crate::{application::INTERVAL_STEP, i18n::*, settings};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageMemory)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/memory.ui")]
    pub struct PerformancePageMemory {
        #[template_child]
        pub total_ram: TemplateChild<gtk::Label>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub graph_max_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub mem_composition: TemplateChild<MemoryCompositionWidget>,
        #[template_child]
        pub mem_composition_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub context_menu: TemplateChild<gtk::Popover>,
        #[template_child]
        pub big_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub swap_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub mem_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub swap_usage_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub total_swap: TemplateChild<gtk::Label>,

        #[property(get, set)]
        base_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        memory_color: Cell<gtk::gdk::RGBA>,
        #[property(get, set)]
        summary_mode: Cell<bool>,

        #[property(get = Self::infobar_content, type = Option < gtk::Widget >)]
        pub infobar_content: OnceCell<gtk::Box>,

        pub tooltip_widget: OnceCell<gtk::Box>,
        pub tt_label_in_use: OnceCell<gtk::Label>,
        pub tt_label_modified: OnceCell<gtk::Label>,
        pub tt_label_standby: OnceCell<gtk::Label>,
        pub tt_label_free: OnceCell<gtk::Label>,

        pub in_use: OnceCell<gtk::Label>,
        pub available: OnceCell<gtk::Label>,
        pub committed: OnceCell<gtk::Label>,
        pub cached: OnceCell<gtk::Label>,
        pub swap_available: OnceCell<gtk::Label>,
        pub swap_used: OnceCell<gtk::Label>,
        pub speed: OnceCell<gtk::Label>,
        pub slots_used: OnceCell<gtk::Label>,
        pub form_factor: OnceCell<gtk::Label>,
        pub ram_type: OnceCell<gtk::Label>,

        pub legend_used: OnceCell<gtk::Picture>,
        pub legend_commited: OnceCell<gtk::Picture>,
    }

    impl Default for PerformancePageMemory {
        fn default() -> Self {
            Self {
                total_ram: Default::default(),
                toast_overlay: Default::default(),
                usage_graph: Default::default(),
                graph_max_duration: Default::default(),
                mem_composition: Default::default(),
                mem_composition_box: Default::default(),
                context_menu: Default::default(),
                big_box: Default::default(),
                swap_box: Default::default(),
                mem_box: Default::default(),
                swap_usage_graph: Default::default(),
                total_swap: Default::default(),

                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                memory_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                infobar_content: Default::default(),

                tooltip_widget: Default::default(),
                tt_label_in_use: Default::default(),
                tt_label_modified: Default::default(),
                tt_label_standby: Default::default(),
                tt_label_free: Default::default(),

                in_use: Default::default(),
                available: Default::default(),
                committed: Default::default(),
                cached: Default::default(),
                swap_available: Default::default(),
                swap_used: Default::default(),
                speed: Default::default(),
                slots_used: Default::default(),
                form_factor: Default::default(),
                ram_type: Default::default(),

                legend_used: Default::default(),
                legend_commited: Default::default(),
            }
        }
    }

    impl PerformancePageMemory {
        fn infobar_content(&self) -> Option<gtk::Widget> {
            self.infobar_content.get().map(|ic| ic.clone().into())
        }
    }

    impl PerformancePageMemory {
        fn configure_actions(this: &super::PerformancePageMemory) {
            use gtk::glib::*;
            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("graph", Some(&actions));

            let settings = settings!();
            let show_memory_composition =
                settings.boolean("performance-page-memory-composition-visible");
            let show_swap = settings.boolean("performance-page-memory-swap-visible");

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

            let action = gio::SimpleAction::new_stateful(
                "memory_composition",
                None,
                &glib::Variant::from(show_memory_composition),
            );
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let mem_composition = &this.mem_composition_box;

                    let visible = !action
                        .state()
                        .and_then(|v| v.get::<bool>())
                        .unwrap_or(false);

                    mem_composition.set_visible(visible);

                    action.set_state(&glib::Variant::from(visible));

                    settings!()
                        .set_boolean("performance-page-memory-composition-visible", visible)
                        .unwrap_or_else(|_| {
                            g_critical!(
                                "MissionCenter::PerformancePage",
                                "Failed to save show composition graph"
                            );
                        });
                }
            });
            actions.add_action(&action);

            let action = gio::SimpleAction::new_stateful(
                "swap_usage",
                None,
                &glib::Variant::from(show_swap),
            );
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let visible = !action
                        .state()
                        .and_then(|v| v.get::<bool>())
                        .unwrap_or(false);

                    this.big_box.set_homogeneous(visible);
                    this.big_box.set_spacing(if visible { 10 } else { 0 });
                    this.swap_box.set_visible(visible);
                    this.swap_usage_graph.set_visible(visible);

                    action.set_state(&glib::Variant::from(visible));

                    settings!()
                        .set_boolean("performance-page-memory-swap-visible", visible)
                        .unwrap_or_else(|_| {
                            g_critical!(
                                "MissionCenter::PerformancePage",
                                "Failed to save show swap graph"
                            );
                        });
                }
            });
            actions.add_action(&action);
        }

        fn configure_context_menu(this: &super::PerformancePageMemory) {
            let right_click_controller = gtk::GestureClick::new();
            right_click_controller.set_button(3); // Secondary click (AKA right click)
            right_click_controller.connect_released({
                let this = this.downgrade();
                move |_click, _n_press, x, y| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.context_menu
                        .set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            x.round() as i32,
                            y.round() as i32,
                            1,
                            1,
                        )));
                    this.context_menu.popup();
                }
            });
            this.add_controller(right_click_controller);
        }
    }

    impl PerformancePageMemory {
        pub fn set_static_information(
            this: &super::PerformancePageMemory,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let this = this.imp();

            let settings = settings!();
            let show_memory_composition =
                settings.boolean("performance-page-memory-composition-visible");
            let show_swap = settings.boolean("performance-page-memory-swap-visible");

            this.usage_graph
                .set_value_range_max(readings.mem_info.mem_total as f32);
            this.swap_usage_graph
                .set_value_range_max(readings.mem_info.swap_total as f32);
            let t = this.obj().clone();
            let swap_total = readings.mem_info.swap_total;

            if swap_total == 0 || !show_swap {
                this.big_box.set_homogeneous(false);
                this.big_box.set_spacing(0);
                this.swap_box.set_visible(false);
                this.swap_usage_graph.set_visible(false);
            }
            if !show_memory_composition {
                this.mem_composition_box.set_visible(false);
            }

            if let Some(legend_commited) = this.legend_commited.get() {
                legend_commited
                    .set_resource(Some("/io/missioncenter/MissionCenter/line-dashed-mem.svg"));
            }
            if let Some(legend_used) = this.legend_used.get() {
                legend_used
                    .set_resource(Some("/io/missioncenter/MissionCenter/line-solid-mem.svg"));
            }

            this.usage_graph.connect_local("resize", true, move |_| {
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

                this.swap_usage_graph
                    .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);

                None
            });

            let total_mem = crate::to_human_readable(readings.mem_info.mem_total as _, 1024.);
            this.total_ram.set_text(&format!(
                "{:.2} {}{}B",
                total_mem.0,
                total_mem.1,
                if total_mem.1.is_empty() { "" } else { "i" }
            ));
            let total_swap = crate::to_human_readable(readings.mem_info.swap_total as _, 1024.);
            this.total_swap.set_text(&format!(
                "{:.2} {}{}B",
                total_swap.0,
                total_swap.1,
                if total_swap.1.is_empty() { "" } else { "i" }
            ));

            true
        }

        pub fn update_readings(
            this: &super::PerformancePageMemory,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let this = this.imp();
            let mem_info = &readings.mem_info;

            {
                let used = mem_info
                    .mem_total
                    .saturating_sub(mem_info.mem_available + mem_info.dirty);
                let dirty = mem_info.mem_total.saturating_sub(mem_info.mem_available);
                let standby = mem_info.mem_total.saturating_sub(used + mem_info.mem_free);
                this.usage_graph.add_data_point(0, mem_info.committed as _);
                this.usage_graph.add_data_point(1, dirty as _);
                this.usage_graph.add_data_point(2, used as _);

                let swap_used = mem_info.swap_total - mem_info.swap_free;
                this.swap_usage_graph.add_data_point(0, swap_used as _);

                this.mem_composition.update_memory_information(mem_info);

                let used = crate::to_human_readable(used as _, 1024.);
                if let Some(iu) = this.in_use.get() {
                    iu.set_text(&format!(
                        "{:.2} {}{}B",
                        used.0,
                        used.1,
                        if used.1.is_empty() { "" } else { "i" }
                    ));
                }

                let available = crate::to_human_readable(mem_info.mem_available as _, 1024.);
                if let Some(av) = this.available.get() {
                    av.set_text(&format!(
                        "{:.2} {}{}B",
                        available.0,
                        available.1,
                        if available.1.is_empty() { "" } else { "i" }
                    ));
                }

                let committed = crate::to_human_readable(mem_info.committed as _, 1024.);
                if let Some(cm) = this.committed.get() {
                    cm.set_text(&format!(
                        "{:.2} {}{}B",
                        committed.0,
                        committed.1,
                        if committed.1.is_empty() { "" } else { "i" }
                    ));
                }

                let cached = crate::to_human_readable(mem_info.cached as _, 1024.);
                if let Some(ch) = this.cached.get() {
                    ch.set_text(&format!(
                        "{:.2} {}{}B",
                        cached.0,
                        cached.1,
                        if cached.1.is_empty() { "" } else { "i" }
                    ));
                }

                let swap_available = crate::to_human_readable(mem_info.swap_total as _, 1024.);
                if let Some(sa) = this.swap_available.get() {
                    sa.set_text(&format!(
                        "{:.2} {}{}B",
                        swap_available.0,
                        swap_available.1,
                        if swap_available.1.is_empty() { "" } else { "i" }
                    ));
                }

                let swap_used = crate::to_human_readable(
                    (mem_info.swap_total - mem_info.swap_free) as _,
                    1024.,
                );
                if let Some(su) = this.swap_used.get() {
                    su.set_text(&format!(
                        "{:.2} {}{}B",
                        swap_used.0,
                        swap_used.1,
                        if swap_used.1.is_empty() { "" } else { "i" }
                    ));
                }

                let free = crate::to_human_readable(mem_info.mem_free as _, 1024.);
                let dirty = crate::to_human_readable(mem_info.dirty as _, 1024.);
                let standby = crate::to_human_readable(standby as _, 1024.);

                if let Some(l) = this.tt_label_in_use.get() {
                    l.set_text(&format!(
                        "{:.2} {}{}B",
                        used.0,
                        used.1,
                        if used.1.is_empty() { "" } else { "i" }
                    ))
                }

                if let Some(l) = this.tt_label_modified.get() {
                    l.set_text(&format!(
                        "{:.2} {}{}B",
                        dirty.0,
                        dirty.1,
                        if dirty.1.is_empty() { "" } else { "i" }
                    ))
                }

                if let Some(l) = this.tt_label_standby.get() {
                    l.set_text(&format!(
                        "{:.2} {}{}B",
                        standby.0,
                        standby.1,
                        if standby.1.is_empty() { "" } else { "i" }
                    ))
                }

                if let Some(l) = this.tt_label_free.get() {
                    l.set_text(&format!(
                        "{:.2} {}{}B",
                        free.0,
                        free.1,
                        if free.1.is_empty() { "" } else { "i" }
                    ))
                }
            }

            true
        }

        fn data_summary(&self) -> String {
            let unknown = i18n("Unknown");
            let unknown = unknown.as_str();

            format!(
                r#"Memory

    {}

    In use:         {}
    Available:      {}
    Committed:      {}
    Cached:         {}
    Swap available: {}
    Swap used:      {}

    Speed:       {}
    Slots used:  {}
    Form factor: {}
    Type:        {}"#,
                self.total_ram.label(),
                self.in_use
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.available
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.committed
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.cached
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.swap_available
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.swap_used
                    .get()
                    .map(|l| l.label())
                    .unwrap_or(unknown.into()),
                self.speed
                    .get()
                    .map(|l| {
                        if !l.uses_markup() {
                            l.label()
                        } else {
                            unknown.into()
                        }
                    })
                    .unwrap_or(unknown.into()),
                self.slots_used
                    .get()
                    .map(|l| {
                        if !l.uses_markup() {
                            l.label()
                        } else {
                            unknown.into()
                        }
                    })
                    .unwrap_or(unknown.into()),
                self.form_factor
                    .get()
                    .map(|l| {
                        if !l.uses_markup() {
                            l.label()
                        } else {
                            unknown.into()
                        }
                    })
                    .unwrap_or(unknown.into()),
                self.ram_type
                    .get()
                    .map(|l| {
                        if !l.uses_markup() {
                            l.label()
                        } else {
                            unknown.into()
                        }
                    })
                    .unwrap_or(unknown.into()),
            )
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

            let this = self.obj().clone();

            self.usage_graph.set_filled(0, false);
            self.usage_graph.set_dashed(0, true);
            self.usage_graph.set_filled(1, false);

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);

            self.mem_box.connect_query_tooltip({
                let this = self.obj().downgrade();
                move |_, _, _, _, tooltip| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return false,
                    };
                    let this = this.imp();

                    if let Some(tooltip_widget) = this.tooltip_widget.get() {
                        tooltip.set_custom(Some(tooltip_widget));
                        return true;
                    }

                    false
                }
            });

            let tooltip_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/memory_info_tooltip.ui",
            );

            let _ = self.tooltip_widget.set(
                tooltip_content_builder
                    .object::<gtk::Box>("root")
                    .expect("Could not find `root` object in tooltip widget"),
            );

            let _ = self.tt_label_in_use.set(
                tooltip_content_builder
                    .object::<gtk::Label>("label_in_use")
                    .expect("Could not find `label_in_use` object in tooltip widget"),
            );

            let _ = self.tt_label_modified.set(
                tooltip_content_builder
                    .object::<gtk::Label>("label_modified")
                    .expect("Could not find `label_modified` object in tooltip widget"),
            );

            let _ = self.tt_label_standby.set(
                tooltip_content_builder
                    .object::<gtk::Label>("label_standby")
                    .expect("Could not find `label_standby` object in tooltip widget"),
            );

            let _ = self.tt_label_free.set(
                tooltip_content_builder
                    .object::<gtk::Label>("label_free")
                    .expect("Could not find `label_free` object in tooltip widget"),
            );

            let sidebar_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/memory_details.ui",
            );
            let _ = self.infobar_content.set(
                sidebar_content_builder
                    .object::<gtk::Box>("root")
                    .expect("Could not find `root` object in details pane"),
            );

            let _ = self.in_use.set(
                sidebar_content_builder
                    .object::<gtk::Label>("in_use")
                    .expect("Could not find `in_use` object in details pane"),
            );
            let _ = self.available.set(
                sidebar_content_builder
                    .object::<gtk::Label>("available")
                    .expect("Could not find `available` object in details pane"),
            );
            let _ = self.committed.set(
                sidebar_content_builder
                    .object::<gtk::Label>("committed")
                    .expect("Could not find `committed` object in details pane"),
            );
            let _ = self.cached.set(
                sidebar_content_builder
                    .object::<gtk::Label>("cached")
                    .expect("Could not find `cached` object in details pane"),
            );
            let _ = self.swap_available.set(
                sidebar_content_builder
                    .object::<gtk::Label>("swap_available")
                    .expect("Could not find `swap_available` object in details pane"),
            );
            let _ = self.swap_used.set(
                sidebar_content_builder
                    .object::<gtk::Label>("swap_used")
                    .expect("Could not find `swap_used` object in details pane"),
            );
            let _ = self.legend_used.set(
                sidebar_content_builder
                    .object::<gtk::Picture>("legend_used")
                    .expect("Could not find `legend_used` object in details pane"),
            );
            let _ = self.legend_commited.set(
                sidebar_content_builder
                    .object::<gtk::Picture>("legend_commited")
                    .expect("Could not find `legend_commited` object in details pane"),
            );

            // Get memory device information using `load_memory_device_info()` from
            // sys_info_v2::MemInfo and update the appropriate labels
            fn update_memory_hw_info(this: &super::PerformancePageMemory) {
                unsafe { glib::gobject_ffi::g_object_ref(this.as_ptr() as *mut _) };
                let ptr = this.as_ptr() as usize;

                // Do this in another thread to avoid stalling the GUI
                let _ = std::thread::spawn(move || {
                    // Load memory device info
                    use crate::sys_info_v2::MemInfo;
                    let memory_device_info = MemInfo::load_memory_device_info();

                    glib::idle_add_once(move || {
                        use glib::translate::from_glib_none;

                        let this: gtk::Widget =
                            unsafe { from_glib_none(ptr as *mut gtk::ffi::GtkWidget) };
                        unsafe { glib::gobject_ffi::g_object_unref(ptr as *mut _) };

                        let this = this.downcast_ref::<super::PerformancePageMemory>().unwrap();

                        // Update the labels with the retrieved information
                        match memory_device_info {
                            Some(memory_device_info) => {
                                // Only update info if we have more than 0 modules
                                let mem_module_count = memory_device_info.len();
                                if mem_module_count > 0 {
                                    if let Some(sp) = this.imp().speed.get() {
                                        if memory_device_info[0].speed != 0 {
                                            sp.set_text(&format!(
                                                "{} MT/s",
                                                memory_device_info[0].speed
                                            ));
                                        }
                                    }
                                    if let Some(su) = this.imp().slots_used.get() {
                                        su.set_text(&format!("{}", mem_module_count));
                                    }
                                    if let Some(ff) = this.imp().form_factor.get() {
                                        if memory_device_info[0].form_factor != "" {
                                            ff.set_text(&format!(
                                                "{}",
                                                memory_device_info[0].form_factor
                                            ));
                                        }
                                    }
                                    if let Some(rt) = this.imp().ram_type.get() {
                                        if memory_device_info[0].ram_type != "" {
                                            rt.set_text(&format!(
                                                "{}",
                                                memory_device_info[0].ram_type
                                            ));
                                        }
                                    }
                                }
                            }
                            _ => this // If getting memory device info failed, show error toast
                                .imp()
                                .toast_overlay
                                .add_toast(adw::Toast::new(&i18n(
                                    "Getting additional memory information failed",
                                ))),
                        }
                    });
                });
            }

            let default_label = format!("{}", i18n("Unknown"));
            let default_label = default_label.as_str();

            let speed: gtk::Label = sidebar_content_builder
                .object("speed")
                .expect("Could not find `speed` object in details pane");
            speed.set_label(default_label);
            let _ = self.speed.set(speed);

            let slots_used: gtk::Label = sidebar_content_builder
                .object("slots_used")
                .expect("Could not find `slots_used` object in details pane");
            slots_used.set_label(default_label);
            let _ = self.slots_used.set(slots_used);

            let form_factor: gtk::Label = sidebar_content_builder
                .object::<gtk::Label>("form_factor")
                .expect("Could not find `form_factor` object in details pane");
            form_factor.set_label(default_label);
            let _ = self.form_factor.set(form_factor);

            let ram_type: gtk::Label = sidebar_content_builder
                .object("ram_type")
                .expect("Could not find `ram_type` object in details pane");
            ram_type.set_label(default_label);
            let _ = self.ram_type.set(ram_type);

            update_memory_hw_info(&this);
        }
    }

    impl WidgetImpl for PerformancePageMemory {
        fn realize(&self) {
            self.parent_realize();
        }
    }

    impl BoxImpl for PerformancePageMemory {}
}

glib::wrapper! {
    pub struct PerformancePageMemory(ObjectSubclass<imp::PerformancePageMemory>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PageExt for PerformancePageMemory {
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

impl PerformancePageMemory {
    pub fn new(settings: &gio::Settings) -> Self {
        let this: Self = glib::Object::builder().build();

        fn update_refresh_rate_sensitive_labels(
            this: &PerformancePageMemory,
            settings: &gio::Settings,
        ) {
            let this = this.imp();

            let data_points = settings.int("performance-page-data-points") as u32;
            let smooth = settings.boolean("performance-smooth-graphs");
            let graph_max_duration = (((settings.uint64("app-update-interval-u64") as f64)
                * INTERVAL_STEP)
                * (data_points as f64))
                .round() as u32;

            let mins = graph_max_duration / 60;
            let seconds_to_string = &i18n_f(
                "{} second{}",
                &[
                    &format!("{}", graph_max_duration % 60),
                    if (graph_max_duration % 60) != 1 {
                        "s"
                    } else {
                        ""
                    },
                ],
            );
            let mins_to_string = &i18n_f(
                "{} minute{} ",
                &[&format!("{:}", mins), if mins > 1 { "s" } else { "" }],
            );
            this.graph_max_duration.set_text(&*format!(
                "{}{}",
                if mins > 0 {
                    mins_to_string.clone()
                } else {
                    "".to_string()
                },
                if graph_max_duration % 60 > 0 {
                    seconds_to_string.clone()
                } else {
                    "".to_string()
                }
            ));

            this.usage_graph.set_data_points(data_points);
            this.swap_usage_graph.set_data_points(data_points);
            this.usage_graph.set_smooth_graphs(smooth);
            this.swap_usage_graph.set_smooth_graphs(smooth);
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

        fn set_hidden(this: &PerformancePageMemory, settings: &gio::Settings) {
            let visible = settings.boolean("performance-page-memory-swap-visible");

            let this = this.imp();

            this.big_box.set_homogeneous(visible);
            this.big_box.set_spacing(if visible { 10 } else { 0 });
            this.swap_box.set_visible(visible);
            this.swap_usage_graph.set_visible(visible);
            this.mem_box.set_has_tooltip(true);
        }

        settings.connect_changed(Some("performance-page-memory-swap-visible"), {
            let this = this.downgrade();
            move |settings, _| {
                if let Some(this) = this.upgrade() {
                    set_hidden(&this, settings);
                }
            }
        });

        this
    }

    pub fn set_static_information(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePageMemory::set_static_information(self, readings)
    }

    pub fn update_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePageMemory::update_readings(self, readings)
    }
}
