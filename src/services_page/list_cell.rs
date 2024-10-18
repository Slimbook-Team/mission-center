/* widgets/mission_center_cell.rs
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

use std::{cell::Cell, rc::Rc};

use adw::{self, subclass::prelude::*};
use gtk::{
    glib::{self, gobject_ffi, ParamSpec, Properties, Value, Variant},
    prelude::*,
};

#[allow(unreachable_code)]
mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ListCell)]
    pub struct ListCell {
        #[property(set = Self::set_item_name, type = glib::GString)]
        item_name: Cell<Rc<str>>,
    }

    impl Default for ListCell {
        fn default() -> Self {
            Self {
                item_name: Cell::new(Rc::<str>::from("")),
            }
        }
    }

    impl ListCell {
        fn set_item_name(&self, item_name: &str) {
            self.item_name.set(Rc::<str>::from(item_name));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ListCell {
        const NAME: &'static str = "ListCell";
        type Type = super::ListCell;
        type ParentType = adw::Bin;

        fn class_init(_klass: &mut Self::Class) {}

        fn instance_init(_obj: &glib::subclass::InitializingObject<Self>) {}
    }

    impl ObjectImpl for ListCell {
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

    impl WidgetImpl for ListCell {
        fn realize(&self) {
            self.parent_realize();

            let this = self.obj();
            if let Some(row_widget) = this.parent().and_then(|p| p.parent()) {
                let gesture_click = gtk::GestureClick::new();
                gesture_click.set_button(3);

                gesture_click.connect_released({
                    let weak_self = unsafe {
                        let weak_ref =
                            Box::leak(Box::<gobject_ffi::GWeakRef>::new(core::mem::zeroed()));
                        gobject_ffi::g_weak_ref_init(weak_ref, this.as_ptr() as *mut _);

                        weak_ref as *mut _ as u64
                    };
                    let this = this.downgrade();
                    move |_, _, x, y| {
                        if let Some(this) = this.upgrade() {
                            let item_name = unsafe { &*this.imp().item_name.as_ptr() }
                                .as_ref()
                                .to_owned();

                            let _ = this.activate_action(
                                "services-page.show-context-menu",
                                Some(&Variant::from((item_name, weak_self, x, y))),
                            );
                        }
                    }
                });
                row_widget.add_controller(gesture_click);
            }
        }
    }

    impl BinImpl for ListCell {}
}

glib::wrapper! {
    pub struct ListCell(ObjectSubclass<imp::ListCell>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ListCell {}
