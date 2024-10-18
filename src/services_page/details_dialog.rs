/* services_page/details_dialog.rs
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

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib::{self, g_warning, ParamSpec, Properties, SignalHandlerId, Value};

use crate::{app, i18n::*};

use super::services_list_item::ServicesListItem;

mod imp {
    use super::*;

    #[derive(Properties)]
    #[properties(wrapper_type = super::DetailsDialog)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/services_page/details_dialog.ui")]
    pub struct DetailsDialog {
        #[template_child]
        group_state: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        box_buttons: TemplateChild<gtk::Box>,
        #[template_child]
        restart: TemplateChild<gtk::Button>,
        #[template_child]
        label_name: TemplateChild<gtk::Label>,
        #[template_child]
        label_description: TemplateChild<gtk::Label>,
        #[template_child]
        label_running: TemplateChild<gtk::Label>,
        #[template_child]
        switch_enabled: TemplateChild<adw::SwitchRow>,

        #[template_child]
        group_process: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        label_pid: TemplateChild<gtk::Label>,
        #[template_child]
        label_user: TemplateChild<gtk::Label>,
        #[template_child]
        label_group: TemplateChild<gtk::Label>,

        #[template_child]
        logs_overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        logs_expander: TemplateChild<gtk::Expander>,
        #[template_child]
        logs_buffer: TemplateChild<gtk::TextBuffer>,

        #[property(get, set)]
        pub enabled: Cell<bool>,

        copy_logs_button: gtk::Button,

        list_item_running_notify: Cell<u64>,
        list_item_enabled_notify: Cell<u64>,
        list_item_enabled_user_change: Cell<bool>,
    }

    impl Default for DetailsDialog {
        fn default() -> Self {
            Self {
                group_state: TemplateChild::default(),
                box_buttons: TemplateChild::default(),
                restart: TemplateChild::default(),
                label_name: TemplateChild::default(),
                label_description: TemplateChild::default(),
                label_running: TemplateChild::default(),
                switch_enabled: TemplateChild::default(),

                group_process: TemplateChild::default(),
                label_pid: TemplateChild::default(),
                label_user: TemplateChild::default(),
                label_group: TemplateChild::default(),

                logs_overlay: TemplateChild::default(),
                logs_expander: TemplateChild::default(),
                logs_buffer: TemplateChild::default(),

                enabled: Cell::new(false),

                copy_logs_button: gtk::Button::new(),

                list_item_running_notify: Cell::new(0),
                list_item_enabled_notify: Cell::new(0),
                list_item_enabled_user_change: Cell::new(true),
            }
        }
    }

    impl DetailsDialog {
        fn list_item(&self) -> ServicesListItem {
            unsafe {
                self.obj()
                    .data::<ServicesListItem>("list-item")
                    .map(|li| li.as_ref())
                    .cloned()
                    .unwrap()
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DetailsDialog {
        const NAME: &'static str = "DetailsDialog";
        type Type = super::DetailsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DetailsDialog {
        fn properties() -> &'static [ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
            self.derived_set_property(id, value, pspec)
        }

        fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            if let Some(_) = std::env::var_os("SNAP_CONTEXT") {
                self.switch_enabled.set_sensitive(false);
                self.box_buttons.set_visible(false);
                self.restart.set_visible(false);
            }

            self.switch_enabled.connect_active_notify({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();

                        if !this.list_item_enabled_user_change.get() {
                            this.list_item_enabled_user_change.set(true);
                            return;
                        }

                        let list_item = this.list_item();
                        match app!().sys_info().and_then(move |sys_info| {
                            match this.switch_enabled.is_active() {
                                // Emitted after the switch is toggled
                                true => sys_info.enable_service(&list_item.name()),
                                false => sys_info.disable_service(&list_item.name()),
                            }

                            Ok(())
                        }) {
                            Err(e) => {
                                g_warning!(
                                    "MissionCenter::DetailsDialog",
                                    "Failed to get `sys_info`: {}",
                                    e
                                );
                            }
                            _ => {}
                        }
                    }
                }
            });

            self.copy_logs_button.set_margin_top(14);
            self.copy_logs_button.set_margin_end(2);
            self.copy_logs_button.set_valign(gtk::Align::Start);
            self.copy_logs_button.set_halign(gtk::Align::End);
            self.copy_logs_button.add_css_class("flat");
            self.copy_logs_button.set_icon_name("edit-copy-symbolic");

            self.copy_logs_button.connect_clicked({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        let clipboard = this.clipboard();

                        let this = this.imp();
                        let logs = this.logs_buffer.property::<glib::GString>("text");

                        clipboard.set_text(logs.as_str());
                    }
                }
            });

            self.logs_overlay.add_overlay(&self.copy_logs_button);
        }
    }

    impl WidgetImpl for DetailsDialog {
        fn realize(&self) {
            self.parent_realize();

            self.logs_buffer.set_text("");
            self.logs_expander.set_visible(false);

            self.logs_expander.set_expanded(false);

            self.list_item_enabled_user_change.set(false);

            let list_item = self.list_item();

            self.label_name.set_text(&list_item.name());
            self.label_description.set_text(&list_item.description());
            let running = if list_item.running() {
                i18n("Running")
            } else if list_item.failed() {
                i18n("Failed")
            } else {
                i18n("Stopped")
            };
            self.label_running.set_text(&running);
            self.switch_enabled.set_active(list_item.enabled());

            let mut group_empty = true;
            let pid = list_item.pid();
            if !pid.is_empty() {
                group_empty = false;
                self.label_pid.set_text(&list_item.pid());
            } else {
                self.label_pid.set_text(&i18n("N/A"));
            }

            let user = list_item.user();
            if !user.is_empty() {
                group_empty = false;
                self.label_user.set_text(&list_item.user());
            } else {
                self.label_user.set_text(&i18n("N/A"));
            }

            let group = list_item.group();
            if !group.is_empty() {
                group_empty = false;
                self.label_group.set_text(&list_item.group());
            } else {
                self.label_group.set_text(&i18n("N/A"));
            }

            if group_empty {
                self.group_process.set_visible(false);
            } else {
                self.group_process.set_visible(true);
            }

            let logs = app!().sys_info().and_then(|sys_info| {
                Ok(sys_info.service_logs(
                    &list_item.name(),
                    NonZeroU32::new(pid.parse::<u32>().unwrap_or(0)),
                ))
            });

            match logs {
                Ok(logs) => {
                    if !logs.is_empty() {
                        self.logs_buffer.set_text(&logs);
                        self.logs_expander.set_visible(true);
                    }
                }
                Err(e) => {
                    g_warning!(
                        "MissionCenter::DetailsDialog",
                        "Failed to get `sys_info`: {}",
                        e
                    );
                }
            }

            let notify = list_item.connect_running_notify({
                let this = self.obj().downgrade();
                move |li| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();
                        let text = if li.running() {
                            i18n("Running")
                        } else if li.failed() {
                            i18n("Failed")
                        } else {
                            i18n("Stopped")
                        };
                        this.label_running.set_text(&text);
                    }
                }
            });
            self.list_item_running_notify.set(from_signal_id(notify));

            let notify = list_item.connect_enabled_notify({
                let this = self.obj().downgrade();
                move |li| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();

                        if li.enabled() != this.switch_enabled.is_active() {
                            this.list_item_enabled_user_change.set(false);
                            this.switch_enabled.set_active(this.list_item().enabled());
                        }
                    }
                }
            });
            self.list_item_enabled_notify.set(from_signal_id(notify));
        }
    }

    impl AdwDialogImpl for DetailsDialog {
        fn closed(&self) {
            let list_item = self.list_item();
            list_item.disconnect(to_signal_id(self.list_item_running_notify.get()));
            list_item.disconnect(to_signal_id(self.list_item_enabled_notify.get()));
        }
    }
}

glib::wrapper! {
    pub struct DetailsDialog(ObjectSubclass<imp::DetailsDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

fn to_signal_id(id: u64) -> SignalHandlerId {
    unsafe { std::mem::transmute(id) }
}

fn from_signal_id(id: SignalHandlerId) -> u64 {
    unsafe { id.as_raw() }
}
