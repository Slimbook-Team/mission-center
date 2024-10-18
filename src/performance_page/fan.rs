/* performance_page/fan.rs
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

use adw;
use adw::subclass::prelude::*;
use glib::{ParamSpec, Properties, Value};
use gtk::{gio, glib, prelude::*};

use super::widgets::GraphWidget;
use crate::application::INTERVAL_STEP;
use crate::i18n::*;
use crate::performance_page::PageExt;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePageFan)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/fan.ui")]
    pub struct PerformancePageFan {
        #[template_child]
        pub title_fan_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub title_temp_name: TemplateChild<gtk::Label>,
        #[template_child]
        pub speed_graph: TemplateChild<GraphWidget>,
        #[template_child]
        pub temp_graph: TemplateChild<GraphWidget>,

        #[template_child]
        pub speed_max_y: TemplateChild<gtk::Label>,
        #[template_child]
        pub speed_graph_max_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub speed_box: TemplateChild<gtk::Box>,

        #[template_child]
        pub temp_max_y: TemplateChild<gtk::Label>,
        #[template_child]
        pub temp_graph_max_duration: TemplateChild<gtk::Label>,
        #[template_child]
        pub temp_graph_box: TemplateChild<gtk::Box>,

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

        pub legend_speed: OnceCell<gtk::Picture>,
        pub speed: OnceCell<gtk::Label>,
        pub pwm_legend_box: OnceCell<gtk::Box>,
        pub temp_label_box: OnceCell<gtk::Box>,
        pub legend_pwm: OnceCell<gtk::Picture>,
        pub pwm: OnceCell<gtk::Label>,
        pub temp: OnceCell<gtk::Label>,
    }

    impl Default for PerformancePageFan {
        fn default() -> Self {
            Self {
                title_fan_name: Default::default(),
                title_temp_name: Default::default(),
                speed_graph: Default::default(),
                temp_graph: Default::default(),
                speed_max_y: Default::default(),
                speed_graph_max_duration: Default::default(),
                speed_box: Default::default(),
                temp_max_y: Default::default(),
                temp_graph_max_duration: Default::default(),
                temp_graph_box: Default::default(),
                context_menu: Default::default(),

                name: Cell::new(String::new()),
                base_color: Cell::new(gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)),
                summary_mode: Cell::new(false),

                infobar_content: Default::default(),

                legend_speed: Default::default(),
                speed: Default::default(),
                pwm_legend_box: Default::default(),
                temp_label_box: Default::default(),
                legend_pwm: Default::default(),
                pwm: Default::default(),
                temp: Default::default(),
            }
        }
    }

    impl PerformancePageFan {
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

    impl PerformancePageFan {
        fn configure_actions(this: &super::PerformancePageFan) {
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

        fn configure_context_menu(this: &super::PerformancePageFan) {
            let right_click_controller = gtk::GestureClick::new();
            right_click_controller.set_button(3); // Secondary click (AKA right click)
            right_click_controller.connect_released({
                let this = this.downgrade();
                move |_click, _n_press, x, y| {
                    if let Some(this) = this.upgrade() {
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
                }
            });
            this.add_controller(right_click_controller);
        }
    }

    impl PerformancePageFan {
        pub fn set_static_information(
            this: &super::PerformancePageFan,
            fan: &crate::sys_info_v2::FanInfo,
        ) -> bool {
            let t = this.clone();

            let this = this.imp();

            this.speed_graph.connect_local("resize", true, move |_| {
                let this = t.imp();

                {
                    let width = this.speed_graph.width() as f32;
                    let height = this.speed_graph.height() as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }

                    this.speed_graph
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
                }

                {
                    let width = this.temp_graph.width() as f32;
                    let height = this.temp_graph.height() as f32;

                    let mut a = width;
                    let mut b = height;
                    if width > height {
                        a = height;
                        b = width;
                    }

                    this.temp_graph
                        .set_vertical_line_count((width * (a / b) / 30.).round().max(5.) as u32);
                }

                None
            });

            this.title_fan_name.set_text(&*fan.fan_label);

            if let Some(legend_send) = this.legend_speed.get() {
                legend_send
                    .set_resource(Some("/io/missioncenter/MissionCenter/line-solid-net.svg"));
            }

            if let Some(legend_pwm) = this.legend_pwm.get() {
                legend_pwm
                    .set_resource(Some("/io/missioncenter/MissionCenter/line-dashed-net.svg"));
            }

            this.title_temp_name.set_text(&fan.temp_name);

            this.speed_graph.set_filled(1, false);
            this.speed_graph.set_dashed(1, true);

            if fan.percent_vroomimg < 0. {
                if let Some(pwm_legend_box) = this.pwm_legend_box.get() {
                    pwm_legend_box.set_visible(false);
                }

                if let Some(legend_speed) = this.legend_speed.get() {
                    legend_speed.set_visible(false);
                }
            }

            if fan.temp_amount == i64::MIN {
                this.temp_graph_box.set_visible(false);

                if let Some(sidebar_temp_box) = this.temp_label_box.get() {
                    sidebar_temp_box.set_visible(false);
                }
            }

            if fan.max_speed > 0 {
                this.speed_max_y.set_text(&format!("{}", fan.max_speed));
            }
            true
        }

        pub fn update_readings(
            this: &super::PerformancePageFan,
            fan: &crate::sys_info_v2::FanInfo,
        ) -> bool {
            let this = this.imp();

            this.title_fan_name
                .set_text(&i18n_f("{}", &[&fan.fan_label]));

            if let Some(speed_send) = this.speed.get() {
                speed_send.set_text(&i18n_f("{} RPM", &[&format!("{}", fan.rpm)]));
            }

            if let Some(pwm) = this.pwm.get() {
                pwm.set_text(&i18n_f(
                    "{}%",
                    &[&format!("{:.0}", fan.percent_vroomimg * 100.0)],
                ));
            }

            let fan_temp_c = fan.temp_amount as f32 / 1000.0;
            if let Some(temp) = this.temp.get() {
                temp.set_text(&i18n_f("{} °C", &[&format!("{:.1}", fan_temp_c)]));
            }

            this.speed_graph.add_data_point(0, fan.rpm as f32);
            if fan.percent_vroomimg >= 0. {
                this.speed_graph
                    .add_data_point(1, fan.percent_vroomimg * 100.);
            }

            if fan.max_speed <= 0 {
                this.speed_max_y.set_text(&i18n_f(
                    "{} RPM",
                    &[&this
                        .speed_graph
                        .max_all_time(0)
                        .unwrap_or(fan.rpm as f32)
                        .to_string()],
                ));
            }

            this.temp_graph.add_data_point(0, fan_temp_c);
            this.temp_max_y.set_text(&format!(
                "{} °C",
                this.temp_graph
                    .max_all_time(0)
                    .unwrap_or(fan_temp_c.round())
            ));

            true
        }

        fn data_summary(&self) -> String {
            let unsupported = i18n("Unsupported");
            let unsupported = glib::GString::from(unsupported);

            format!(
                r#"Fan

    {}
    {}

    Speed:               {}
    PWM Percentage:      {}
    Current Temperature: {}"#,
                self.title_fan_name.text(),
                self.title_temp_name.text(),
                self.speed
                    .get()
                    .map(|s| s.text())
                    .unwrap_or(unsupported.clone()),
                self.pwm
                    .get()
                    .and_then(|pwm| if !pwm.is_visible() { None } else { Some(pwm) })
                    .map(|s| s.text())
                    .unwrap_or(unsupported.clone()),
                self.temp
                    .get()
                    .and_then(|temp| if !temp.is_visible() { None } else { Some(temp) })
                    .map(|s| s.text())
                    .unwrap_or(unsupported)
            )
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePageFan {
        const NAME: &'static str = "PerformancePageFan";
        type Type = super::PerformancePageFan;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePageFan {
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
            let this = obj.upcast_ref::<super::PerformancePageFan>().clone();

            Self::configure_actions(&this);
            Self::configure_context_menu(&this);

            let sidebar_content_builder = gtk::Builder::from_resource(
                "/io/missioncenter/MissionCenter/ui/performance_page/fan_details.ui",
            );

            let _ = self.infobar_content.set(
                sidebar_content_builder
                    .object::<gtk::Box>("root")
                    .expect("Could not find `root` object in details pane"),
            );

            let _ = self.legend_speed.set(
                sidebar_content_builder
                    .object::<gtk::Picture>("legend_speed")
                    .expect("Could not find `legend_speed` object in details pane"),
            );
            let _ = self.legend_pwm.set(
                sidebar_content_builder
                    .object::<gtk::Picture>("legend_pwm")
                    .expect("Could not find `legend_pwm` object in details pane"),
            );

            let _ = self.pwm_legend_box.set(
                sidebar_content_builder
                    .object::<gtk::Box>("pwm_legend_box")
                    .expect("Could not find `pwm_legend_box` object in details pane"),
            );

            let _ = self.temp_label_box.set(
                sidebar_content_builder
                    .object::<gtk::Box>("temp_label_box")
                    .expect("Could not find `temp_legend_box` object in details pane"),
            );

            let _ = self.speed.set(
                sidebar_content_builder
                    .object::<gtk::Label>("speed")
                    .expect("Could not find `speed` object in details pane"),
            );
            let _ = self.pwm.set(
                sidebar_content_builder
                    .object::<gtk::Label>("pwm")
                    .expect("Could not find `pwm` object in details pane"),
            );
            let _ = self.temp.set(
                sidebar_content_builder
                    .object::<gtk::Label>("temp")
                    .expect("Could not find `temp` object in details pane"),
            );
        }
    }

    impl WidgetImpl for PerformancePageFan {}

    impl BoxImpl for PerformancePageFan {}
}

glib::wrapper! {
    pub struct PerformancePageFan(ObjectSubclass<imp::PerformancePageFan>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PageExt for PerformancePageFan {
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

impl PerformancePageFan {
    pub fn new(name: &str, settings: &gio::Settings) -> Self {
        let this: Self = glib::Object::builder().property("name", name).build();

        fn update_refresh_rate_sensitive_labels(
            this: &PerformancePageFan,
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
            let time_string = &*format!(
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
            );

            this.speed_graph_max_duration.set_text(time_string);
            this.speed_graph.set_data_points(data_points);
            this.speed_graph.set_smooth_graphs(smooth);

            this.temp_graph_max_duration.set_text(time_string);
            this.temp_graph.set_data_points(data_points);
            this.temp_graph.set_smooth_graphs(smooth);
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

    pub fn set_static_information(&self, fan_info: &crate::sys_info_v2::FanInfo) -> bool {
        imp::PerformancePageFan::set_static_information(self, fan_info)
    }

    pub fn update_readings(&self, fan_info: &crate::sys_info_v2::FanInfo) -> bool {
        imp::PerformancePageFan::update_readings(self, fan_info)
    }
}
