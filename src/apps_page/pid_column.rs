/* apps_page/pid_column.rs
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
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::PidColumn)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/pid_column.ui")]
    pub struct PidColumn {
        #[template_child]
        label: TemplateChild<gtk::Label>,

        #[property(set = Self::set_content_type, type = u8)]
        content_type: Cell<crate::apps_page::row_model::ContentType>,
        #[property(get, set = Self::set_value)]
        value: Cell<u32>,
    }

    impl Default for PidColumn {
        fn default() -> Self {
            use crate::apps_page::row_model::ContentType;

            Self {
                label: TemplateChild::default(),
                content_type: Cell::new(ContentType::SectionHeader),
                value: Cell::new(0),
            }
        }
    }

    impl PidColumn {
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

        fn set_value(&self, v: u32) {
            self.value.set(v);
        }
    }

    impl PidColumn {
        fn update_label(&self) {
            use crate::apps_page::row_model::ContentType;

            if self.content_type.get() != ContentType::Process
                && self.content_type.get() != ContentType::App
            {
                self.label.set_text("");
                return;
            }

            self.label.set_text(&format!("{}", self.value.get()))
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PidColumn {
        const NAME: &'static str = "PidColumn";
        type Type = super::PidColumn;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PidColumn {
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

    impl WidgetImpl for PidColumn {
        fn realize(&self) {
            self.parent_realize();
            self.update_label();

            self.obj().connect_value_notify(|this| {
                this.imp().update_label();
            });

            self.obj().connect_content_type_notify(|this| {
                this.imp().update_label();
            });
        }
    }

    impl BoxImpl for PidColumn {}
}

glib::wrapper! {
    pub struct PidColumn(ObjectSubclass<imp::PidColumn>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
