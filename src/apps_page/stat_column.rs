/* apps_page/stat_column.rs
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

use gtk::{
    glib,
    glib::prelude::*,
    glib::{ParamSpec, Properties, Value},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::StatColumn)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/stat_column.ui")]
    pub struct StatColumn {
        #[template_child]
        label: TemplateChild<gtk::Label>,

        css_provider: Cell<gtk::CssProvider>,

        #[property(get = Self::unit, set = Self::set_unit, type = glib::GString)]
        unit: Cell<glib::GString>,
        #[property(set = Self::set_content_type, type = u8)]
        content_type: Cell<crate::apps_page::row_model::ContentType>,
        #[property(get, set)]
        value: Cell<f32>,
        #[property(set = Self::set_usage_percent)]
        usage_percent: Cell<f32>,
    }

    impl Default for StatColumn {
        fn default() -> Self {
            use crate::apps_page::row_model::ContentType;

            Self {
                label: TemplateChild::default(),
                css_provider: Cell::new(gtk::CssProvider::new()),
                unit: Cell::new(glib::GString::from("")),
                content_type: Cell::new(ContentType::SectionHeader),
                value: Cell::new(0.),
                usage_percent: Cell::new(0.),
            }
        }
    }

    impl StatColumn {
        fn unit(&self) -> glib::GString {
            unsafe { &*self.unit.as_ptr() }.clone()
        }

        fn set_unit(&self, unit: &str) {
            self.unit.set(glib::GString::from(unit));
        }

        fn set_content_type(&self, v: u8) {
            use crate::apps_page::row_model::ContentType;

            let content_type = match v {
                0 => ContentType::SectionHeader,
                1 => ContentType::App,
                2 => ContentType::Process,
                _ => unreachable!(),
            };

            self.content_type.set(content_type);
        }

        fn set_usage_percent(&self, usage_percent: f32) {
            use crate::apps_page::{
                CSS_CELL_USAGE_HIGH, CSS_CELL_USAGE_LOW, CSS_CELL_USAGE_MEDIUM,
            };

            self.usage_percent.set(usage_percent);

            let css_provider = unsafe { &*self.css_provider.as_ptr() };
            if usage_percent >= 90.0 {
                css_provider.load_from_bytes(&glib::Bytes::from_static(CSS_CELL_USAGE_HIGH));
            } else if usage_percent >= 80.0 {
                css_provider.load_from_bytes(&glib::Bytes::from_static(CSS_CELL_USAGE_MEDIUM));
            } else if usage_percent >= 70.0 {
                css_provider.load_from_bytes(&glib::Bytes::from_static(CSS_CELL_USAGE_LOW));
            } else {
                css_provider.load_from_bytes(&glib::Bytes::from_static(b""));
            }
        }
    }

    impl StatColumn {
        fn update_label(&self) {
            use crate::apps_page::row_model::ContentType;

            if self.content_type.get() == ContentType::SectionHeader {
                self.label.set_text("");
                return;
            }

            let prop_unit = unsafe { &*self.unit.as_ptr() }.as_str();

            let value = self.value.get();
            if prop_unit == "%" {
                self.label
                    .set_text(&format!("{}{}", value.round(), prop_unit));
            } else if prop_unit == "bps" {
                let (value, unit, dec_to_display) = crate::to_human_readable(value, 1024.);
                self.label.set_text(&format!(
                    "{0:.2$} {1}{3}",
                    value, unit, dec_to_display, prop_unit
                ));
            } else {
                let (value, unit, dec_to_display) = crate::to_human_readable_adv(value, 1024., 2);
                self.label.set_text(&format!(
                    "{0:.2$} {1}{3}{4}",
                    value,
                    unit,
                    dec_to_display,
                    if unit.is_empty() { "" } else { "i" },
                    prop_unit
                ));
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for StatColumn {
        const NAME: &'static str = "StatColumn";
        type Type = super::StatColumn;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StatColumn {
        fn constructed(&self) {
            let this = self.obj().downgrade();
            self.obj().connect_value_notify(move |_| {
                let this = this.upgrade();
                if let Some(this) = this {
                    this.imp().update_label();
                }
            });
        }

        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for StatColumn {
        fn realize(&self) {
            self.parent_realize();

            if let Some(tree_expander) = self.obj().parent() {
                if let Some(column_view_cell) = tree_expander.parent() {
                    let style_provider = unsafe { &*self.css_provider.as_ptr() };
                    // FIXME: Deprecated in GTK 4.10, removed in GTK 5.0, unclear what the replacement is
                    #[allow(deprecated)]
                    {
                        column_view_cell
                            .style_context()
                            .add_provider(style_provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
                    }
                }
            }

            self.update_label();
        }
    }

    impl BoxImpl for StatColumn {}
}

glib::wrapper! {
    pub struct StatColumn(ObjectSubclass<imp::StatColumn>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
