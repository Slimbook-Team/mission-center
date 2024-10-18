/* services_page/context_menu_button.rs
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

use gtk::{
    glib::{self, gobject_ffi, prelude::*, ParamSpec, Properties, Value, Variant},
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ContextMenuButton)]
    #[derive(gtk::CompositeTemplate)]
    #[template(
        resource = "/io/missioncenter/MissionCenter/ui/services_page/context_menu_button.ui"
    )]
    pub struct ContextMenuButton {
        #[template_child]
        pub button: TemplateChild<gtk::Button>,

        #[property(name = "item-name", set = Self::set_item_name, type = glib::GString)]
        _item_name: [u8; 0],
    }

    impl Default for ContextMenuButton {
        fn default() -> Self {
            Self {
                button: TemplateChild::default(),
                _item_name: [0; 0],
            }
        }
    }

    impl ContextMenuButton {
        pub fn set_item_name(&self, name: &str) {
            // Create a weak reference to the object and pass it to the action
            let weak_self = unsafe {
                let weak_ref = Box::leak(Box::<gobject_ffi::GWeakRef>::new(core::mem::zeroed()));
                gobject_ffi::g_weak_ref_init(weak_ref, self.obj().as_ptr() as *mut _);

                weak_ref as *mut _ as u64
            };

            self.button
                .set_action_name(Some("services-page.show-context-menu"));
            self.button.set_action_target_value(Some(&Variant::from((
                name.to_owned(),
                weak_self,
                -1_f64,
                -1_f64,
            ))));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContextMenuButton {
        const NAME: &'static str = "ContextMenuButton";
        type Type = super::ContextMenuButton;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContextMenuButton {
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

    impl WidgetImpl for ContextMenuButton {}

    impl BoxImpl for ContextMenuButton {}
}

glib::wrapper! {
    pub struct ContextMenuButton(ObjectSubclass<imp::ContextMenuButton>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}
