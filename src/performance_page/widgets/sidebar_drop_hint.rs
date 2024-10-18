/* performance_page/widgets/sidebar_drop_hint.rs
 *
 * Copyright 2024 Mission Center Developers
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

use glib::{ParamSpec, Properties, Value};
use gtk::prelude::WidgetExt;
use gtk::{gdk, gdk::prelude::*, glib, prelude::*, subclass::prelude::*, Snapshot};

mod imp {
    use super::*;
    use gtk::{graphene, gsk};

    #[derive(Properties)]
    #[properties(wrapper_type = super::SidebarDropHint)]
    pub struct SidebarDropHint {
        #[property(get)]
        color: Cell<gdk::RGBA>,
    }

    impl Default for SidebarDropHint {
        fn default() -> Self {
            Self {
                color: Cell::new(gdk::RGBA::new(0.2, 0.51, 0.89, 1.)),
            }
        }
    }

    impl SidebarDropHint {}

    #[glib::object_subclass]
    impl ObjectSubclass for SidebarDropHint {
        const NAME: &'static str = "SidebarDropHint";
        type Type = super::SidebarDropHint;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for SidebarDropHint {
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

            let this = self.obj();

            let style_manager = adw::StyleManager::default();
            // self.color.set(style_manager.accent_color_rgba());
            this.notify_color();

            this.set_sensitive(false);
        }
    }

    impl WidgetImpl for SidebarDropHint {
        fn realize(&self) {
            self.parent_realize();
        }

        fn snapshot(&self, snapshot: &Snapshot) {
            let this = self.obj();

            this.set_height_request(3);

            let radius = graphene::Size::new(2., 2.);
            let bounds = gsk::RoundedRect::new(
                graphene::Rect::new(0., 0., this.width() as _, this.height() as _),
                radius,
                radius,
                radius,
                radius,
            );

            snapshot.push_rounded_clip(&bounds);

            let path_builder = gsk::PathBuilder::new();
            path_builder.add_rect(&graphene::Rect::new(
                0.,
                0.,
                this.width() as _,
                this.height() as _,
            ));
            snapshot.append_fill(
                &path_builder.to_path(),
                gsk::FillRule::Winding,
                &this.color(),
            );

            snapshot.pop();
        }
    }
}

glib::wrapper! {
    pub struct SidebarDropHint(ObjectSubclass<imp::SidebarDropHint>)
        @extends gtk::Widget,
        @implements gtk::Buildable;
}

impl SidebarDropHint {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
