/* performance_page/summary_graph.rs
 *
 * Copyright 2024 Romeo Calota
 * Copyright 2024 jojo2357
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

use std::collections::HashSet;

use adw::subclass::prelude::*;
use glib::{ParamSpec, Properties, Value};
use gtk::{gdk, glib, prelude::*};

use super::widgets::{GraphWidget, SidebarDropHint};
use crate::settings;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::SummaryGraph)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/summary_graph.ui")]
    #[allow(dead_code)]
    pub struct SummaryGraph {
        pub drop_hint: SidebarDropHint,

        #[template_child]
        pub drag_handle_icon: TemplateChild<gtk::Image>,
        #[template_child]
        pub graph_widget: TemplateChild<GraphWidget>,
        #[template_child]
        label_heading: TemplateChild<gtk::Label>,
        #[template_child]
        label_info1: TemplateChild<gtk::Label>,
        #[template_child]
        label_info2: TemplateChild<gtk::Label>,
        #[template_child]
        pub enabled_switch: TemplateChild<gtk::Switch>,

        #[property(get = Self::is_enabled, set = Self::set_enabled, type = bool)]
        is_enabled: [u8; 0],

        #[property(get = Self::base_color, set = Self::set_base_color, type = gdk::RGBA)]
        base_color: [u8; 0],
        #[property(get = Self::heading, set = Self::set_heading, type = String)]
        heading: [u8; 0],
        #[property(get = Self::info1, set = Self::set_info1, type = String)]
        info1: [u8; 0],
        #[property(get = Self::info2, set = Self::set_info2, type = String)]
        info2: [u8; 0],
    }

    impl Default for SummaryGraph {
        fn default() -> Self {
            Self {
                drop_hint: SidebarDropHint::new(),
                drag_handle_icon: Default::default(),
                graph_widget: Default::default(),
                label_heading: Default::default(),
                label_info1: Default::default(),
                label_info2: Default::default(),
                enabled_switch: Default::default(),

                is_enabled: [0; 0],

                base_color: [0; 0],
                heading: [0; 0],
                info1: [0; 0],
                info2: [0; 0],
            }
        }
    }

    impl SummaryGraph {
        fn is_enabled(&self) -> bool {
            self.enabled_switch.is_active()
        }

        fn set_enabled(&self, enabled: bool) {
            self.enabled_switch.set_active(enabled);
        }

        fn base_color(&self) -> gdk::RGBA {
            self.graph_widget.base_color()
        }

        fn set_base_color(&self, base_color: gdk::RGBA) {
            self.graph_widget.set_base_color(base_color);
        }

        fn heading(&self) -> String {
            self.label_heading.text().to_string()
        }

        fn set_heading(&self, heading: String) {
            self.label_heading.set_text(&heading);
        }

        fn info1(&self) -> String {
            self.label_info1.text().to_string()
        }

        fn set_info1(&self, info1: String) {
            self.label_info1.set_text(&info1);
            if info1.is_empty() {
                self.label_info1.set_visible(false);
            } else {
                self.label_info1.set_visible(true);
            }
        }

        fn info2(&self) -> String {
            self.label_info2.text().to_string()
        }

        fn set_info2(&self, info2: String) {
            self.label_info2.set_text(&info2);
            if info2.is_empty() {
                self.label_info2.set_visible(false);
            } else {
                self.label_info2.set_visible(true);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SummaryGraph {
        const NAME: &'static str = "SummaryGraph";
        type Type = super::SummaryGraph;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SummaryGraph {
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

            self.enabled_switch.connect_active_notify({
                let this = self.obj().downgrade();
                move |switch| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    let settings = settings!();

                    let hidden_graphs = settings.string("performance-sidebar-hidden-graphs");
                    let mut hidden_graphs = hidden_graphs
                        .split(";")
                        .filter(|g| !g.is_empty())
                        .collect::<HashSet<_>>();
                    let name = this.widget_name();
                    let name = name.as_str();

                    if switch.is_active() && hidden_graphs.contains(name) {
                        hidden_graphs.remove(name);
                    } else if !switch.is_active() && !hidden_graphs.contains(name) {
                        hidden_graphs.insert(name);
                    }

                    let mut output = String::new();
                    for graph in hidden_graphs {
                        output.push_str(graph);
                        output.push(';');
                    }
                    let output = if !output.is_empty() {
                        &output[..output.len() - 1]
                    } else {
                        ""
                    };

                    settings
                        .set_string("performance-sidebar-hidden-graphs", output)
                        .unwrap_or_else(|_| {
                            glib::g_warning!(
                                "MissionCenter::PerformancePage",
                                "Failed to set performance-sidebar-hidden-graphs setting"
                            );
                        });

                    this.notify_is_enabled();
                }
            });
        }
    }

    impl WidgetImpl for SummaryGraph {}

    impl BoxImpl for SummaryGraph {}
}

glib::wrapper! {
    pub struct SummaryGraph(ObjectSubclass<imp::SummaryGraph>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Buildable;
}

impl SummaryGraph {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_edit_mode(&self, edit_mode: bool) {
        self.imp().drag_handle_icon.set_visible(edit_mode);
        self.imp().enabled_switch.set_visible(edit_mode);
        if let Some(parent) = self.parent() {
            parent.set_visible(edit_mode || self.is_enabled());
        }
    }

    pub fn graph_widget(&self) -> GraphWidget {
        self.imp().graph_widget.clone()
    }

    pub fn show_drop_hint_top(&self) {
        self.hide_drop_hint();

        self.imp().drop_hint.set_margin_bottom(20);

        self.prepend(&self.imp().drop_hint);
    }

    pub fn show_drop_hint_bottom(&self) {
        self.hide_drop_hint();

        self.imp().drop_hint.set_margin_top(20);

        self.append(&self.imp().drop_hint);
    }

    pub fn hide_drop_hint(&self) {
        if !self.is_drop_hint_visible() {
            return;
        }

        self.imp().drop_hint.set_margin_top(0);
        self.imp().drop_hint.set_margin_bottom(0);

        self.remove(&self.imp().drop_hint);
    }

    pub fn is_drop_hint_visible(&self) -> bool {
        self.imp().drop_hint.parent().is_some()
    }

    pub fn is_drop_hint_top(&self) -> bool {
        if !self.is_drop_hint_visible() {
            return false;
        }

        self.imp().drop_hint.prev_sibling().is_none()
    }

    pub fn is_drop_hint_bottom(&self) -> bool {
        if !self.is_drop_hint_visible() {
            return false;
        }

        self.imp().drop_hint.next_sibling().is_none()
    }
}
