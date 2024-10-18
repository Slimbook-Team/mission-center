/* services_page/mod.rs
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

use std::cell::Cell;

use adw::{
    glib::{ParamSpec, Propagation, Properties, Value},
    prelude::AdwDialogExt,
};
use gtk::{
    gdk, gio,
    glib::{
        self, g_critical, g_warning, gobject_ffi, translate::from_glib_full, Object, VariantTy,
        WeakRef,
    },
    prelude::*,
    subclass::prelude::*,
    INVALID_LIST_POSITION,
};

use context_menu_button::ContextMenuButton;
use details_dialog::DetailsDialog;
use list_cell::ListCell;
use services_list_item::{ServicesListItem, ServicesListItemBuilder};

use crate::{
    app,
    i18n::*,
    sys_info_v2::{Readings, SysInfoV2},
};

mod context_menu_button;
mod details_dialog;
mod list_cell;
mod services_list_item;

mod imp {
    use super::*;

    pub struct Actions {
        pub start: gio::SimpleAction,
        pub stop: gio::SimpleAction,
        pub restart: gio::SimpleAction,
    }

    #[derive(Properties)]
    #[properties(wrapper_type = super::ContextMenuButton)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/services_page/page.ui")]
    pub struct ServicesPage {
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub h1: TemplateChild<gtk::Label>,
        #[template_child]
        pub h2: TemplateChild<gtk::Label>,
        #[template_child]
        pub start: TemplateChild<gtk::Button>,
        #[template_child]
        start_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub stop: TemplateChild<gtk::Button>,
        #[template_child]
        stop_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub restart: TemplateChild<gtk::Button>,
        #[template_child]
        restart_label: TemplateChild<gtk::Label>,
        #[template_child]
        details_label: TemplateChild<gtk::Label>,
        #[template_child]
        name_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        description_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        context_menu: TemplateChild<gtk::PopoverMenu>,

        #[property(name = "details-dialog", get = Self::details_dialog, set = Self::set_details_dialog, type = Option<DetailsDialog>
        )]
        details_dialog: Cell<Option<DetailsDialog>>,
        pub details_dialog_visible: Cell<bool>,

        pub model: gio::ListStore,
        pub actions: Actions,
    }

    impl Default for ServicesPage {
        fn default() -> Self {
            Self {
                column_view: TemplateChild::default(),
                h1: TemplateChild::default(),
                h2: TemplateChild::default(),
                start: TemplateChild::default(),
                start_label: TemplateChild::default(),
                stop: TemplateChild::default(),
                stop_label: TemplateChild::default(),
                restart: TemplateChild::default(),
                restart_label: TemplateChild::default(),
                details_label: TemplateChild::default(),
                name_column: TemplateChild::default(),
                description_column: TemplateChild::default(),
                context_menu: TemplateChild::default(),

                details_dialog: Cell::new(None),
                details_dialog_visible: Cell::new(false),

                model: gio::ListStore::new::<ServicesListItem>(),
                actions: Actions {
                    start: gio::SimpleAction::new("selected-svc-start", None),
                    stop: gio::SimpleAction::new("selected-svc-stop", None),
                    restart: gio::SimpleAction::new("selected-svc-restart", None),
                },
            }
        }
    }

    impl ServicesPage {
        fn details_dialog(&self) -> Option<DetailsDialog> {
            unsafe { &*self.details_dialog.as_ptr() }.clone()
        }

        fn set_details_dialog(&self, widget: Option<&DetailsDialog>) {
            if let Some(widget) = widget {
                widget.connect_closed({
                    let this = self.obj().downgrade();
                    move |_| {
                        if let Some(this) = this.upgrade() {
                            this.imp().details_dialog_visible.set(false);
                        }
                    }
                });

                unsafe { widget.set_data("list-item", ServicesListItemBuilder::new().build()) };
            }

            self.details_dialog.set(widget.cloned());
        }
    }

    impl ServicesPage {
        pub fn collapse(&self) {
            if let None = std::env::var_os("SNAP_CONTEXT") {
                self.start_label.set_visible(false);
                self.stop_label.set_visible(false);
                self.restart_label.set_visible(false);
            }
            self.details_label.set_visible(false);

            self.h2.set_visible(false);

            self.name_column.set_fixed_width(1);
            self.name_column.set_expand(true);
            self.name_column.set_resizable(false);
            self.description_column.set_visible(false);
        }

        pub fn expand(&self) {
            if let None = std::env::var_os("SNAP_CONTEXT") {
                self.start_label.set_visible(true);
                self.stop_label.set_visible(true);
                self.restart_label.set_visible(true);
            }
            self.details_label.set_visible(true);

            self.h2.set_visible(true);

            self.name_column.set_fixed_width(300);
            self.name_column.set_expand(false);
            self.name_column.set_resizable(true);
            self.description_column.set_visible(true);
        }

        fn configure_actions(&self) {
            let this = self.obj();
            let this = this.as_ref();

            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("services-page", Some(&actions));

            let action = gio::SimpleAction::new("show-context-menu", Some(VariantTy::TUPLE));
            action.connect_activate({
                let this = this.downgrade();
                move |_action, service| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => {
                            g_critical!(
                                "MissionCenter::ServicesPage",
                                "Failed to get ServicesPage instance from show-context-menu action"
                            );
                            return;
                        }
                    };
                    let this = this.imp();

                    let (name, anchor) = match service.and_then(|s| s.get::<(String, u64, f64, f64)>()) {
                        Some((name, ptr, x, y)) => {
                            // We just get a pointer to a weak reference to the object
                            // Do the necessary checks and downcast the object to a Widget
                            let anchor_widget = unsafe {
                                let ptr = gobject_ffi::g_weak_ref_get(ptr as usize as *mut _);
                                if ptr.is_null() {
                                    return;
                                } else {
                                    let obj: Object = from_glib_full(ptr);
                                    match obj.downcast::<gtk::Widget>() {
                                        Ok(w) => w,
                                        Err(_) => {
                                            g_critical!(
                                                    "MissionCenter::ServicesPage",
                                                    "Failed to downcast object to GtkWidget"
                                                );
                                            return;
                                        }
                                    }
                                }
                            };

                            let anchor = if x > 0. && y > 0. {
                                this.context_menu.set_has_arrow(false);

                                match anchor_widget.compute_point(
                                    &*this.obj(),
                                    &gtk::graphene::Point::new(x as _, y as _),
                                ) {
                                    None => {
                                        g_critical!(
                                            "MissionCenter::AppsPage",
                                            "Failed to compute_point, context menu will not be anchored to mouse position"
                                        );
                                        gdk::Rectangle::new(
                                            x.round() as i32,
                                            y.round() as i32,
                                            1,
                                            1,
                                        )
                                    }
                                    Some(p) => {
                                        gdk::Rectangle::new(
                                            p.x().round() as i32,
                                            p.y().round() as i32,
                                            1,
                                            1,
                                        )
                                    }
                                }
                            } else {
                                this.context_menu.set_has_arrow(true);

                                if let Some(bounds) = anchor_widget.compute_bounds(&*this.obj()) {
                                    gdk::Rectangle::new(
                                        bounds.x() as i32,
                                        bounds.y() as i32,
                                        bounds.width() as i32,
                                        bounds.height() as i32,
                                    )
                                } else {
                                    g_warning!(
                                        "MissionCenter::ServicesPage",
                                        "Failed to get bounds for menu button, popup will display in an arbitrary location"
                                    );
                                    gdk::Rectangle::new(0, 0, 0, 0)
                                }
                            };

                            (name, anchor)
                        }

                        None => {
                            g_critical!(
                                "MissionCenter::ServicesPage",
                                "Failed to get service name and button from show-context-menu action"
                            );
                            return;
                        }
                    };

                    let model = match this.column_view.model().as_ref().cloned() {
                        Some(model) => model,
                        None => {
                            g_critical!(
                                "MissionCenter::ServicesPage",
                                "Failed to get model for `show-context-menu` action"
                            );
                            return;
                        }
                    };

                    let list_item_pos = {
                        let mut pos = None;
                        for i in 0..model.n_items() {
                            if let Some(item) =
                                model
                                    .item(i)
                                    .and_then(|i| i.downcast_ref::<ServicesListItem>().cloned())
                            {
                                if item.name() == name {
                                    pos = Some(i);
                                    break;
                                }
                            }
                        }

                        if let Some(pos) = pos {
                            pos
                        } else {
                            g_critical!(
                                "MissionCenter::ServicesPage",
                                "Failed to get ServicesListItem named {} from model",
                                name
                            );
                            return;
                        }
                    };

                    model.select_item(list_item_pos, false);
                    this.context_menu.set_pointing_to(Some(&anchor));
                    this.context_menu.popup();
                }
            });
            actions.add_action(&action);

            fn find_selected_item(
                this: WeakRef<super::ServicesPage>,
            ) -> Option<(super::ServicesPage, ServicesListItem)> {
                let this_obj = match this.upgrade() {
                    Some(this) => this,
                    None => {
                        g_critical!(
                            "MissionCenter::ServicesPage",
                            "Failed to get ServicesPage instance for action"
                        );
                        return None;
                    }
                };
                let this = this_obj.imp();

                let selected_item = match this
                    .column_view
                    .model()
                    .and_then(|m| m.downcast_ref::<gtk::SingleSelection>().cloned())
                    .and_then(|s| s.selected_item())
                    .and_then(|i| i.downcast_ref::<ServicesListItem>().cloned())
                {
                    Some(item) => item,
                    None => {
                        g_critical!(
                            "MissionCenter::ServicesPage",
                            "Failed to find selected item"
                        );
                        return None;
                    }
                };

                Some((this_obj, selected_item))
            }

            fn make_gatherer_request(
                this: WeakRef<super::ServicesPage>,
                request: fn(&SysInfoV2, &str),
            ) {
                let app = app!();

                let (_, selected_item) = match find_selected_item(this) {
                    Some((this, item)) => (this, item),
                    None => {
                        g_critical!(
                            "MissionCenter::ServicesPage",
                            "Failed to get selected item for action"
                        );
                        return;
                    }
                };

                match app.sys_info() {
                    Ok(sys_info) => {
                        request(&sys_info, &selected_item.name());
                    }
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::ServicesPage",
                            "Failed to get sys_info from MissionCenterApplication: {}",
                            e
                        );
                    }
                };
            }

            self.actions.start.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    make_gatherer_request(this.clone(), |sys_info, service_name| {
                        sys_info.start_service(service_name);
                    });
                }
            });

            self.actions.stop.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    make_gatherer_request(this.clone(), |sys_info, service_name| {
                        sys_info.stop_service(service_name);
                    });
                }
            });

            self.actions.restart.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    make_gatherer_request(this.clone(), |sys_info, service_name| {
                        sys_info.restart_service(service_name);
                    });
                }
            });

            let action = gio::SimpleAction::new("details", None);
            action.connect_activate({
                let this = this.downgrade();
                move |_action, _| {
                    match find_selected_item(this.clone()) {
                        Some((this, item)) => {
                            let details_dialog =
                                unsafe { &*this.imp().details_dialog.as_ptr() }.clone();
                            details_dialog.map(move |d| {
                                this.imp().details_dialog_visible.set(true);
                                unsafe {
                                    d.set_data("list-item", item);
                                }
                                d.present(Some(&this));
                            });
                        }
                        None => {
                            g_critical!(
                                "MissionCenter::ServicesPage",
                                "Failed to get selected item for action"
                            );
                            return;
                        }
                    };
                }
            });
            actions.add_action(&action);
        }

        pub fn set_up_filter_model(&self, model: gio::ListModel) -> gtk::FilterListModel {
            let window = match app!().window() {
                Some(window) => window,
                None => {
                    g_critical!(
                        "MissionCenter::ServicesPage",
                        "Failed to get MissionCenterWindow instance"
                    );
                    return gtk::FilterListModel::new(
                        Some(model),
                        Some(gtk::CustomFilter::new(|_| true)),
                    );
                }
            };

            let filter = gtk::CustomFilter::new({
                let window = window.downgrade();
                move |obj| {
                    use textdistance::{Algorithm, Levenshtein};

                    let window = match window.upgrade() {
                        None => return true,
                        Some(w) => w,
                    };
                    let window = window.imp();

                    if !window.search_button.is_active() {
                        return true;
                    }

                    if window.header_search_entry.text().is_empty() {
                        return true;
                    }

                    let list_item = match obj.downcast_ref::<ServicesListItem>() {
                        None => return false,
                        Some(li) => li,
                    };

                    let entry_name = list_item.name().to_lowercase();
                    let pid = list_item.pid().to_string();
                    let search_query = window.header_search_entry.text().to_lowercase();

                    if entry_name.contains(&search_query)
                        || (!pid.is_empty() && pid.contains(&search_query))
                    {
                        return true;
                    }

                    if search_query.contains(&entry_name)
                        || (!pid.is_empty() && search_query.contains(&pid))
                    {
                        return true;
                    }

                    let str_distance = Levenshtein::default()
                        .for_str(&entry_name, &search_query)
                        .ndist();
                    if str_distance <= 0.6 {
                        return true;
                    }

                    false
                }
            });

            window.imp().header_search_entry.connect_search_changed({
                let filter = filter.downgrade();
                let window = window.downgrade();
                move |_| {
                    if let Some(window) = window.upgrade() {
                        if !window.services_page_active() {
                            return;
                        }

                        if let Some(filter) = filter.upgrade() {
                            filter.changed(gtk::FilterChange::Different);
                        }
                    }
                }
            });

            gtk::FilterListModel::new(Some(model), Some(filter))
        }

        pub fn update_model(&self, readings: &mut Readings) {
            let model = &self.model;

            let mut to_remove = Vec::new();
            for i in 0..model.n_items() {
                let item = model.item(i).unwrap();
                if let Some(item) = item.downcast_ref::<ServicesListItem>() {
                    if let Some(service) = readings.services.remove(item.name().as_str()) {
                        item.set_description(service.description.as_ref());
                        item.set_enabled(service.enabled);
                        item.set_running(service.running);
                        item.set_failed(service.failed);
                        if let Some(pid) = service.pid {
                            item.set_pid(pid.to_string());
                        } else {
                            item.set_pid("".to_string());
                        }
                        if let Some(user) = &service.user {
                            item.set_user(user.as_ref());
                        } else {
                            item.set_user("");
                        }
                        if let Some(group) = &service.group {
                            item.set_group(group.as_ref());
                        } else {
                            item.set_group("");
                        }
                    } else {
                        to_remove.push(i);
                    }
                }
            }

            for i in to_remove.iter().rev() {
                model.remove(*i);
            }

            for (_, service) in &readings.services {
                let mut model_item_builder = ServicesListItemBuilder::new()
                    .name(&service.name)
                    .description(&service.description)
                    .enabled(service.enabled)
                    .running(service.running)
                    .failed(service.failed);
                if let Some(pid) = service.pid {
                    model_item_builder = model_item_builder.pid(pid);
                }
                if let Some(user) = &service.user {
                    model_item_builder = model_item_builder.user(user);
                }
                if let Some(group) = &service.group {
                    model_item_builder = model_item_builder.group(group);
                }

                model.append(&model_item_builder.build());
            }

            let total_services = model.n_items();
            let mut running_services = 0;
            let mut failed_services = 0;
            for i in 0..total_services {
                let item = model.item(i).unwrap();
                if let Some(item) = item.downcast_ref::<ServicesListItem>() {
                    if item.running() {
                        running_services += 1;
                    } else if item.failed() {
                        failed_services += 1;
                    }
                }
            }

            self.h1.set_text(&i18n_f(
                "{} Running Services",
                &[&running_services.to_string()],
            ));

            self.h2.set_text(&i18n_f(
                "{} failed services out of a total of {}",
                &[&failed_services.to_string(), &total_services.to_string()],
            ));

            if let Some(selection_model) = self
                .column_view
                .model()
                .and_then(|m| m.downcast_ref::<gtk::SingleSelection>().cloned())
            {
                let selected = selection_model.selected();
                if selected != INVALID_LIST_POSITION {
                    selection_model.set_selected(INVALID_LIST_POSITION);
                    selection_model.set_selected(selected);
                }
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ServicesPage {
        const NAME: &'static str = "ServicesPage";
        type Type = super::ServicesPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            ListCell::ensure_type();
            ContextMenuButton::ensure_type();
            ServicesListItem::ensure_type();
            DetailsDialog::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ServicesPage {
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
                self.start.set_visible(false);
                self.stop.set_visible(false);
                self.restart.set_visible(false);

                let menu = gio::Menu::new();
                menu.append(Some(&i18n("Details")), Some("services-page.details"));

                self.context_menu
                    .set_menu_model(Some(&gio::MenuModel::from(menu)));
            }

            self.configure_actions();

            let evt_key_press = gtk::EventControllerKey::new();
            evt_key_press.connect_key_pressed({
                let this = self.obj().downgrade();
                move |_, key, _, _| {
                    if let Some(this) = this.upgrade() {
                        let this = this.imp();

                        if key == gdk::Key::Escape {
                            unsafe { &*this.details_dialog.as_ptr() }
                                .clone()
                                .map(|d| d.force_close());
                        }
                    }

                    Propagation::Proceed
                }
            });
            self.obj().add_controller(evt_key_press);
        }
    }

    impl WidgetImpl for ServicesPage {}

    impl BoxImpl for ServicesPage {}
}

glib::wrapper! {
    pub struct ServicesPage(ObjectSubclass<imp::ServicesPage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl ServicesPage {
    pub fn dialog_visible(&self) -> bool {
        self.imp().details_dialog_visible.get()
    }

    #[inline]
    pub fn collapse(&self) {
        self.imp().collapse();
    }

    #[inline]
    pub fn expand(&self) {
        self.imp().expand();
    }

    pub fn set_initial_readings(&self, readings: &mut Readings) -> bool {
        let this = self.imp();

        if let Some(window) = app!().window() {
            window.add_action(&this.actions.start);
            window.add_action(&this.actions.stop);
            window.add_action(&this.actions.restart);
        }

        let filter_model = this.set_up_filter_model(this.model.clone().into());
        let selection_model = gtk::SingleSelection::new(Some(filter_model));
        selection_model.connect_selected_notify({
            let this = self.downgrade();
            move |model| {
                let selected = match model
                    .selected_item()
                    .and_then(|i| i.downcast_ref::<ServicesListItem>().cloned())
                {
                    Some(list_item) => list_item,
                    None => {
                        return;
                    }
                };

                let this = match this.upgrade() {
                    Some(this) => this,
                    None => {
                        g_critical!(
                            "MissionCenter::ServicesPage",
                            "Failed to get ServicesPage instance in `selected_notify` signal"
                        );
                        return;
                    }
                };
                let this = this.imp();

                if selected.running() {
                    this.actions.stop.set_enabled(true);
                    this.actions.start.set_enabled(false);
                    this.actions.restart.set_enabled(true);
                } else {
                    this.actions.stop.set_enabled(false);
                    this.actions.start.set_enabled(true);
                    this.actions.restart.set_enabled(false);
                }
            }
        });
        this.column_view.set_model(Some(&selection_model));

        if let Some(header) = this.column_view.first_child() {
            header.add_css_class("app-list-header");

            // Add 10px padding to the left of the first column header to align it with the content
            if let Some(first_column) = header
                .first_child()
                .and_then(|w| w.first_child())
                .and_then(|w| w.first_child())
            {
                first_column.set_margin_start(10);
            }
        }

        this.update_model(readings);

        true
    }

    pub fn update_readings(&self, readings: &mut Readings) -> bool {
        self.imp().update_model(readings);
        true
    }
}
