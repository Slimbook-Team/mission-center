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

use std::cell::{Cell, RefCell};

use gtk::{
    glib,
    glib::prelude::*,
    glib::{ParamSpec, Properties, Value},
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

        #[property(get = Self::property_name, set = Self::set_property_name, type = glib::GString)]
        property_name: Cell<glib::GString>,
        #[property(get = Self::unit, set = Self::set_unit, type = glib::GString)]
        unit: Cell<glib::GString>,
        #[property(set = Self::set_content)]
        pub content: RefCell<Option<glib::Object>>,
        #[property(set = Self::set_content_type, type = i32)]
        pub content_type: Cell<crate::apps_page::view_models::ContentType>,
    }

    impl Default for StatColumn {
        fn default() -> Self {
            use crate::apps_page::view_models::ContentType;

            Self {
                property_name: Cell::new(glib::GString::from("")),
                label: TemplateChild::default(),
                unit: Cell::new(glib::GString::from("")),
                content: RefCell::new(None),
                content_type: Cell::new(ContentType::None),
            }
        }
    }

    impl StatColumn {
        fn property_name(&self) -> glib::GString {
            let name = self.property_name.take();
            self.property_name.set(name.clone());

            name
        }

        fn set_property_name(&self, property_name: &str) {
            self.property_name.set(glib::GString::from(property_name));
        }

        fn unit(&self) -> glib::GString {
            unsafe { &*self.unit.as_ptr() }.clone()
        }

        fn set_unit(&self, unit: &str) {
            self.unit.set(glib::GString::from(unit));
        }

        fn set_content(&self, content: Option<glib::Object>) {
            use crate::apps_page::view_models::{
                AppModel, ContentType, ProcessModel, SectionHeaderModel,
            };

            self.content.set(content);

            let content = unsafe { &*self.content.as_ptr() }.as_ref();
            if content.is_none() {
                self.content_type.set(ContentType::None);
                return;
            }

            if content
                .and_then(|c| c.downcast_ref::<ProcessModel>())
                .is_some()
            {
                self.content_type.set(ContentType::Process);
                return;
            }

            if content.and_then(|c| c.downcast_ref::<AppModel>()).is_some() {
                self.content_type.set(ContentType::App);
                return;
            }

            if content
                .and_then(|c| c.downcast_ref::<SectionHeaderModel>())
                .is_some()
            {
                self.content_type.set(ContentType::SectionHeader);
                return;
            }
        }

        fn set_content_type(&self, v: i32) {
            use crate::apps_page::view_models::ContentType;

            let content_type = match v {
                0 => ContentType::None,
                1 => ContentType::SectionHeader,
                2 => ContentType::App,
                3 => ContentType::Process,
                _ => unreachable!(),
            };

            self.content_type.set(content_type);
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
            use crate::apps_page::view_models::ContentType;
            use glib::*;

            self.parent_realize();

            if self.content_type.get() == ContentType::None
                || self.content_type.get() == ContentType::SectionHeader
            {
                return;
            }

            fn update_property(this: &StatColumn) {
                let content = this.content.borrow();
                let content = content.as_ref();
                if content.is_none() {
                    g_critical!("MissionCenter::AppsPage", "Model has no content");
                    return;
                }
                let content = content.unwrap();

                let property_name = unsafe { &*this.property_name.as_ptr() }.as_str();
                let prop_unit = unsafe { &*this.unit.as_ptr() }.as_str();

                let value = content.property::<f32>(property_name);
                if prop_unit == "%" {
                    this.label
                        .set_text(&format!("{}{}", value.round(), prop_unit));
                } else if prop_unit == "bps" {
                    let (value, unit) = crate::to_human_readable(value, 1024.);
                    this.label
                        .set_text(&format!("{} {}{}", value.round(), unit, prop_unit));
                } else {
                    let (value, unit) = crate::to_human_readable(value, 1024.);
                    this.label.set_text(&format!(
                        "{} {}{}{}",
                        value.round(),
                        unit,
                        if unit.is_empty() { "" } else { "i" },
                        prop_unit
                    ));
                }
            }
            update_property(self);

            let content = self.content.borrow();
            let content = content.as_ref();
            if content.is_none() {
                g_critical!("MissionCenter::AppsPage", "Model has no content");
                return;
            }
            let content = content.unwrap();
            let property_name = unsafe { &*self.property_name.as_ptr() }.as_str();
            content.connect_notify_local(
                Some(property_name),
                clone!(
                    @weak self as this => move |_, _| {
                        update_property(&this);
                    }
                ),
            );
        }
    }

    impl BoxImpl for StatColumn {}
}

glib::wrapper! {
    pub struct StatColumn(ObjectSubclass<imp::StatColumn>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl StatColumn {}
