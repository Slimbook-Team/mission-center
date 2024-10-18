/* services_page/services_list_model.rs
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

use std::{cell::Cell, num::NonZeroU32};

use gtk::glib::{self, prelude::*, subclass::prelude::*, ParamSpec, Properties, Value};

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::ServicesListItem)]
    pub struct ServicesListItem {
        #[property(get = Self::name, set = Self::set_name, type = glib::GString)]
        pub name: Cell<glib::GString>,
        #[property(get = Self::description, set = Self::set_description, type = glib::GString)]
        pub description: Cell<glib::GString>,
        #[property(get, set = Self::set_enabled)]
        pub enabled: Cell<bool>,
        #[property(get, set = Self::set_running)]
        pub running: Cell<bool>,
        #[property(get, set = Self::set_failed)]
        pub failed: Cell<bool>,
        #[property(get = Self::icon_name, type = glib::GString)]
        pub icon_name: Cell<glib::GString>,
        #[property(get = Self::pid, set = Self::set_pid, type = glib::GString)]
        pub pid: Cell<glib::GString>,
        #[property(get = Self::user, set = Self::set_user, type = glib::GString)]
        pub user: Cell<glib::GString>,
        #[property(get = Self::group, set = Self::set_group, type = glib::GString)]
        pub group: Cell<glib::GString>,
    }

    impl Default for ServicesListItem {
        fn default() -> Self {
            Self {
                name: Cell::new(glib::GString::default()),
                description: Cell::new(glib::GString::default()),
                enabled: Cell::new(false),
                running: Cell::new(false),
                failed: Cell::new(false),
                icon_name: Cell::new("service-disabled".into()),
                pid: Cell::new(glib::GString::default()),
                user: Cell::new(glib::GString::default()),
                group: Cell::new(glib::GString::default()),
            }
        }
    }

    impl ServicesListItem {
        fn update_icon(&self) {
            self.icon_name.set(if self.running.get() {
                "service-running".into()
            } else {
                if self.failed.get() {
                    "service-failed".into()
                } else if self.enabled.get() {
                    "service-stopped".into()
                } else {
                    "service-disabled".into()
                }
            });
            self.obj().notify_icon_name();
        }
    }

    impl ServicesListItem {
        pub fn name(&self) -> glib::GString {
            let name = self.name.take();
            let result = name.clone();
            self.name.set(name);

            result
        }

        pub fn set_name(&self, name: &str) {
            let current_name = self.name.take();
            if current_name == name {
                self.name.set(current_name);
                return;
            }

            self.name.set(glib::GString::from(name));
        }

        pub fn description(&self) -> glib::GString {
            let description = self.description.take();
            let result = description.clone();
            self.description.set(description);

            result
        }

        pub fn set_description(&self, description: &str) {
            let current_description = self.description.take();
            if current_description == description {
                self.description.set(current_description);
                return;
            }

            self.description.set(glib::GString::from(description));
        }

        pub fn set_enabled(&self, enabled: bool) {
            let current_enabled = self.enabled.get();
            if current_enabled == enabled {
                return;
            }

            self.enabled.set(enabled);

            self.update_icon();
        }

        pub fn set_running(&self, running: bool) {
            let current_running = self.running.get();
            if current_running == running {
                return;
            }

            self.running.set(running);

            self.update_icon();
        }

        pub fn set_failed(&self, failed: bool) {
            let current_failed = self.failed.get();
            if current_failed == failed {
                return;
            }

            self.failed.set(failed);

            self.update_icon();
        }

        pub fn icon_name(&self) -> glib::GString {
            let icon_name = self.icon_name.take();
            let result = icon_name.clone();
            self.icon_name.set(icon_name);

            result
        }

        pub fn pid(&self) -> glib::GString {
            let pid = self.pid.take();
            let result = pid.clone();
            self.pid.set(pid);

            result
        }

        pub fn set_pid(&self, pid: &str) {
            let current_pid = self.pid.take();
            if current_pid == pid {
                self.pid.set(current_pid);
                return;
            }

            self.pid.set(glib::GString::from(pid));
        }

        pub fn user(&self) -> glib::GString {
            let user = self.user.take();
            let result = user.clone();
            self.user.set(user);

            result
        }

        pub fn set_user(&self, user: &str) {
            let current_user = self.user.take();
            if current_user == user {
                self.user.set(current_user);
                return;
            }

            self.user.set(glib::GString::from(user));
        }

        pub fn group(&self) -> glib::GString {
            let group = self.group.take();
            let result = group.clone();
            self.group.set(group);

            result
        }

        pub fn set_group(&self, group: &str) {
            let current_group = self.group.take();
            if current_group == group {
                self.group.set(current_group);
                return;
            }

            self.group.set(glib::GString::from(group));
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServicesListItem {
        const NAME: &'static str = "ServicesListItem";
        type Type = super::ServicesListItem;
    }

    impl ObjectImpl for ServicesListItem {
        fn constructed(&self) {
            self.parent_constructed();
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
}

pub struct ServicesListItemBuilder {
    name: glib::GString,
    description: glib::GString,
    enabled: bool,
    running: bool,
    failed: bool,
    pid: Option<NonZeroU32>,
    user: glib::GString,
    group: glib::GString,
}

impl ServicesListItemBuilder {
    pub fn new() -> Self {
        Self {
            name: "".into(),
            description: "".into(),
            enabled: false,
            running: false,
            failed: false,
            pid: None,
            user: "".into(),
            group: "".into(),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.into();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn running(mut self, running: bool) -> Self {
        self.running = running;
        self
    }

    pub fn failed(mut self, failed: bool) -> Self {
        self.failed = failed;
        self
    }

    pub fn pid(mut self, pid: NonZeroU32) -> Self {
        self.pid = Some(pid);
        self
    }

    pub fn user(mut self, user: &str) -> Self {
        self.user = user.into();
        self
    }

    pub fn group(mut self, group: &str) -> Self {
        self.group = group.into();
        self
    }

    pub fn build(self) -> ServicesListItem {
        let this = ServicesListItem::new();

        {
            let this = this.imp();
            this.name.set(self.name);
            this.description.set(self.description);
            this.set_enabled(self.enabled);
            this.set_running(self.running);
            this.set_failed(self.failed);
            this.pid.set(
                self.pid
                    .map_or_else(|| "".into(), |pid| pid.to_string().into()),
            );
            this.user.set(self.user);
            this.group.set(self.group);
        }

        this
    }
}

glib::wrapper! {
    pub struct ServicesListItem(ObjectSubclass<imp::ServicesListItem>);
}

impl ServicesListItem {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
