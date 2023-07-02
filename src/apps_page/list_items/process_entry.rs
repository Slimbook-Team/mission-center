/* apps_page/list_items/process_entry.rs
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

use gtk::{
    glib,
    glib::{ParamSpec, Properties, Value},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ProcessEntry)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/process_entry.ui")]
    pub struct ProcessEntry {
        #[template_child]
        pub name: TemplateChild<gtk::Label>,

        #[allow(dead_code)]
        #[property(name = "name", get = Self::name, set = Self::set_name, type = glib::GString)]
        name_property: [u8; 0],
    }

    impl Default for ProcessEntry {
        fn default() -> Self {
            Self {
                name: TemplateChild::default(),
                name_property: [0; 0],
            }
        }
    }

    impl ProcessEntry {
        pub fn name(&self) -> glib::GString {
            self.name.label()
        }

        pub fn set_name(&self, name: &str) {
            self.name.set_text(name)
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProcessEntry {
        const NAME: &'static str = "ProcessEntry";
        type Type = super::ProcessEntry;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProcessEntry {
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

    impl WidgetImpl for ProcessEntry {}

    impl BoxImpl for ProcessEntry {}
}

glib::wrapper! {
    pub struct ProcessEntry(ObjectSubclass<imp::ProcessEntry>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl ProcessEntry {
    pub fn new(_: &gtk::TreeExpander) -> Self {
        let this: Self = glib::Object::builder().build();
        this
    }
}
