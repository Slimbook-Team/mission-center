/* apps_page/mod.rs
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

use std::{cell::Cell, collections::HashMap, sync::Arc};

use gio::ListStore;
use glib::translate::from_glib_full;
use gtk::{gdk, gio, glib, prelude::*, subclass::prelude::*};

use crate::{
    app,
    apps_page::{list_item::ListItem, row_model::ContentType},
    i18n::*,
    settings,
    sys_info_v2::{App, Process},
};

mod column_header;
mod list_item;
mod pid_column;
mod row_model;
mod stat_column;

pub const CSS_CELL_USAGE_LOW: &[u8] = b"cell { background-color: rgba(246, 211, 45, 0.3); }";
pub const CSS_CELL_USAGE_MEDIUM: &[u8] = b"cell { background-color: rgba(230, 97, 0, 0.3); }";
pub const CSS_CELL_USAGE_HIGH: &[u8] = b"cell { background-color: rgba(165, 29, 45, 0.3); }";

mod imp {
    use super::*;

    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/apps_page/page.ui")]
    pub struct AppsPage {
        #[template_child]
        pub column_view: TemplateChild<gtk::ColumnView>,
        #[template_child]
        pub name_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub pid_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub cpu_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub memory_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub disk_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_usage_column: TemplateChild<gtk::ColumnViewColumn>,
        #[template_child]
        pub gpu_memory_column: TemplateChild<gtk::ColumnViewColumn>,

        #[template_child]
        pub context_menu: TemplateChild<gtk::PopoverMenu>,

        pub column_header_name: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_pid: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_cpu: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_memory: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_disk: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_gpu_usage: Cell<Option<column_header::ColumnHeader>>,
        pub column_header_gpu_memory_usage: Cell<Option<column_header::ColumnHeader>>,

        pub tree_list_sorter: Cell<Option<gtk::TreeListRowSorter>>,

        pub apps_model: Cell<ListStore>,
        pub processes_root_model: Cell<ListStore>,

        pub max_cpu_usage: Cell<f32>,
        pub max_memory_usage: Cell<f32>,
        pub max_disk_usage: Cell<f32>,
        pub max_gpu_memory_usage: Cell<f32>,

        pub apps: Cell<HashMap<Arc<str>, App>>,
        pub process_tree: Cell<Process>,

        pub use_merge_stats: Cell<bool>,
    }

    impl Default for AppsPage {
        fn default() -> Self {
            use crate::sys_info_v2::Process;
            use std::collections::HashMap;

            Self {
                column_view: TemplateChild::default(),
                name_column: TemplateChild::default(),
                pid_column: TemplateChild::default(),
                cpu_column: TemplateChild::default(),
                memory_column: TemplateChild::default(),
                disk_column: TemplateChild::default(),
                gpu_usage_column: TemplateChild::default(),
                gpu_memory_column: TemplateChild::default(),

                context_menu: TemplateChild::default(),

                column_header_name: Cell::new(None),
                column_header_pid: Cell::new(None),
                column_header_cpu: Cell::new(None),
                column_header_memory: Cell::new(None),
                column_header_disk: Cell::new(None),
                column_header_gpu_usage: Cell::new(None),
                column_header_gpu_memory_usage: Cell::new(None),

                tree_list_sorter: Cell::new(None),

                apps_model: Cell::new(ListStore::new::<row_model::RowModel>()),
                processes_root_model: Cell::new(ListStore::new::<row_model::RowModel>()),

                max_cpu_usage: Cell::new(0.0),
                max_memory_usage: Cell::new(0.0),
                max_disk_usage: Cell::new(0.0),
                max_gpu_memory_usage: Cell::new(0.0),

                apps: Cell::new(HashMap::new()),
                process_tree: Cell::new(Process::default()),

                use_merge_stats: Cell::new(false),
            }
        }
    }

    impl AppsPage {
        fn find_process(root_process: &Process, pid: u32) -> Option<&Process> {
            if root_process.pid == pid {
                return Some(root_process);
            }

            match root_process.children.get(&pid) {
                Some(process) => return Some(process),
                None => {}
            }

            for (_, process) in &root_process.children {
                match Self::find_process(process, pid) {
                    Some(process) => return Some(process),
                    None => {}
                }
            }

            None
        }

        fn configure_actions(&self) {
            use crate::sys_info_v2::Process;
            use gtk::glib::*;

            fn find_pid(
                this: Option<super::AppsPage>,
                pid_and_bool: Option<&glib::Variant>,
            ) -> Option<u32> {
                let this = match this {
                    None => return None,
                    Some(this) => this,
                };

                let (mut pid, is_app) = match pid_and_bool.and_then(|p| p.get::<(u32, bool)>()) {
                    Some(pid) => pid,
                    None => {
                        g_critical!(
                            "MissionCenter::AppsPage",
                            "Invalid data encountered for 'pid' and 'is_app' parameter when stopping process",
                        );
                        return None;
                    }
                };

                // For some wierd reason when stopping the bwarp process for Flatpak apps, they
                // just continue running which makes it confusing for the end-user. So go through
                // the children of the bwrap process and find the first child that is not a bwrap
                // and terminate that instead.
                fn find_first_non_bwrap_child(root: &Process) -> Option<&Process> {
                    for (_, child) in &root.children {
                        if child.name.as_ref() != "bwrap" {
                            return Some(child);
                        }
                    }

                    for (_, child) in &root.children {
                        if let Some(child) = find_first_non_bwrap_child(child) {
                            return Some(child);
                        }
                    }

                    None
                }
                if is_app {
                    if let Some(process) =
                        AppsPage::find_process(unsafe { &*this.imp().process_tree.as_ptr() }, pid)
                    {
                        if process.name.as_ref() == "bwrap" {
                            if let Some(child) = find_first_non_bwrap_child(process) {
                                pid = child.pid;
                            }
                        }
                    }
                }

                Some(pid)
            }

            let this = self.obj();
            let this = this.as_ref();

            let actions = gio::SimpleActionGroup::new();
            this.insert_action_group("apps-page", Some(&actions));

            let app = app!();

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

                    let (list_item, pid, anchor) = match service.and_then(|s| s.get::<(u32, u64, f64, f64)>()) {
                        Some((pid, ptr, x, y)) => {
                            // We just get a pointer to a weak reference to the object
                            // Do the necessary checks and downcast the object to a Widget
                            let list_item = unsafe {
                                let ptr = gobject_ffi::g_weak_ref_get(ptr as usize as *mut _);
                                if ptr.is_null() {
                                    return;
                                } else {
                                    let obj: Object = from_glib_full(ptr);
                                    match obj.downcast::<gtk::Widget>() {
                                        Ok(w) => w,
                                        Err(_) => {
                                            g_critical!(
                                                "MissionCenter::AppsPage",
                                                "Failed to downcast object to GtkWidget"
                                            );
                                            return;
                                        }
                                    }
                                }
                            };
                            let list_item = list_item.downcast::<ListItem>().unwrap();
                            if list_item.content_type() == ContentType::SectionHeader as u8 {
                                return;
                            }

                            let anchor = match list_item.compute_point(
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
                            };

                            (list_item, pid, anchor)
                        }

                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to get process/app PID from show-context-menu action"
                            );
                            return;
                        }
                    };

                    list_item.row().and_then(|row| {
                        let _ = row.activate_action("listitem.select", Some(&glib::Variant::from((true, true))));
                        None::<()>
                    });

                    const CONTENT_TYPE_APP: u8 = ContentType::App as _;
                    const CONTENT_TYPE_PROCESS: u8 = ContentType::Process as _;

                    let (stop_label, force_stop_label, is_app) = match list_item.content_type() {
                        CONTENT_TYPE_APP => {
                            (i18n("Stop Application"), i18n("Force Stop Application"), true)
                        }
                        CONTENT_TYPE_PROCESS => {
                            (i18n("Stop Process"), i18n("Force Stop Process"), false)
                        }
                        _ => unreachable!(),
                    };

                    let menu = gio::Menu::new();

                    let mi_stop = gio::MenuItem::new(Some(&stop_label), None);
                    mi_stop.set_action_and_target_value(Some("apps-page.stop"), Some(&Variant::from((pid, is_app))));
                    let mi_force_stop = gio::MenuItem::new(Some(&force_stop_label), None);
                    mi_force_stop.set_action_and_target_value(Some("apps-page.force-stop"), Some(&Variant::from((pid, is_app))));

                    menu.append_item(&mi_stop);
                    menu.append_item(&mi_force_stop);

                    this.context_menu.set_menu_model(Some(&menu));
                    this.context_menu.set_pointing_to(Some(&anchor));
                    this.context_menu.popup();
                }
            });
            actions.add_action(&action);

            let action = gio::SimpleAction::new("stop", Some(VariantTy::TUPLE));
            action.connect_activate({
                let app = app.downgrade();
                let this = self.obj().downgrade();
                move |_action, pid| {
                    let pid = match find_pid(this.upgrade(), pid) {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to terminate app: Failed to find PID",
                            );
                            return;
                        }
                        Some(pid) => pid,
                    };

                    let app = match app.upgrade() {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to terminate app PID '{}': Failed to get app instance",
                                pid,
                            );
                            return;
                        }
                        Some(app) => app,
                    };

                    let sys_info = match app.sys_info() {
                        Ok(si) => si,
                        Err(err) => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to terminate app PID '{}': {}",
                                pid,
                                err,
                            );
                            return;
                        }
                    };

                    sys_info.terminate_process(pid);
                }
            });
            actions.add_action(&action);

            let action = gio::SimpleAction::new("force-stop", Some(VariantTy::TUPLE));
            action.connect_activate({
                let app = app.downgrade();
                let this = self.obj().downgrade();
                move |_action, pid| {
                    let pid = match find_pid(this.upgrade(), pid) {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to kill app: Failed to find PID",
                            );
                            return;
                        }
                        Some(pid) => pid,
                    };

                    let app = match app.upgrade() {
                        None => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to kill app PID '{}': Failed to get app instance",
                                pid,
                            );
                            return;
                        }
                        Some(app) => app,
                    };

                    let sys_info = match app.sys_info() {
                        Ok(si) => si,
                        Err(err) => {
                            g_critical!(
                                "MissionCenter::AppsPage",
                                "Failed to kill app PID '{}': {}",
                                pid,
                                err,
                            );
                            return;
                        }
                    };

                    sys_info.kill_process(pid);
                }
            });
            actions.add_action(&action);
        }

        pub fn update_app_model(&self) {
            use crate::glib_clone;
            use gtk::glib::g_critical;
            use row_model::{ContentType, RowModel, RowModelBuilder};
            use std::collections::BTreeSet;

            fn find_pid_in_process_tree(model: ListStore, pid: u32) -> Option<RowModel> {
                fn fpipt_impl(model: ListStore, pid: u32, result: &mut Option<RowModel>) {
                    let len = model.n_items();
                    for i in 0..len {
                        let current = model.item(i).unwrap().downcast::<RowModel>().unwrap();
                        if current.pid() == pid {
                            *result = Some(current);
                            return;
                        }
                    }

                    for i in 0..len {
                        let current = model.item(i).unwrap().downcast::<RowModel>().unwrap();
                        fpipt_impl(current.children().clone(), pid, result);
                    }
                }

                let mut result = None;
                fpipt_impl(model, pid, &mut result);

                result
            }

            fn update_icons(model: ListStore, icon: &str) {
                let len = model.n_items();
                for i in 0..len {
                    let current = model.item(i).unwrap().downcast::<RowModel>().unwrap();
                    current.set_icon(icon);
                    update_icons(current.children().clone(), icon);
                }
            }

            let model = glib_clone!(self.apps_model);
            let apps = self.apps.take();
            let process_tree = self.process_tree.take();

            let mut to_remove = BTreeSet::new();
            for i in 0..model.n_items() {
                let current = model.item(i).unwrap().downcast::<RowModel>();
                if current.is_err() {
                    continue;
                }
                let current = current.unwrap();

                if !apps.contains_key(current.id().as_str()) {
                    to_remove.insert(i);
                }
            }

            for (i, to_remove_i) in to_remove.iter().enumerate() {
                model.remove((*to_remove_i as usize - i) as _);
            }

            for (app_id, app) in &apps {
                let pos = if model.n_items() > 0 {
                    model.find_with_equal_func(|current| {
                        let current = current.downcast_ref::<RowModel>();
                        if current.is_none() {
                            return false;
                        }
                        let current = current.unwrap();

                        current.id().as_str() == app_id.as_ref()
                    })
                } else {
                    None
                };

                // Find the first process that has any children. This is most likely the root
                // of the App's process tree.
                let (primary_process, primary_pid) = {
                    let mut primary_process = None;
                    let mut primary_pid = 0;
                    for (index, pid) in app.pids.iter().enumerate() {
                        if let Some(process) = Self::find_process(&process_tree, *pid) {
                            if process.children.len() > 0 || index == app.pids.len() - 1 {
                                primary_process = Some(process);
                                primary_pid = process.pid;
                                break;
                            }
                        }
                    }

                    (primary_process, primary_pid)
                };

                let pp = primary_process.cloned().unwrap_or_default();
                let row_model = if pos.is_none() {
                    let row_model = RowModelBuilder::new()
                        .name(app.name.as_ref())
                        .id(app.id.as_ref())
                        .icon(
                            app.icon
                                .as_ref()
                                .map(|i| i.as_ref())
                                .unwrap_or("application-x-executable"),
                        )
                        .pid(primary_pid)
                        .content_type(ContentType::App)
                        .expanded(false)
                        .cpu_usage(pp.merged_usage_stats.cpu_usage)
                        .memory_usage(pp.merged_usage_stats.memory_usage)
                        .disk_usage(pp.merged_usage_stats.disk_usage)
                        .network_usage(pp.merged_usage_stats.network_usage)
                        .gpu_usage(pp.merged_usage_stats.gpu_usage)
                        .gpu_mem_usage(pp.merged_usage_stats.gpu_memory_usage)
                        .max_cpu_usage(self.max_cpu_usage.get())
                        .max_memory_usage(self.max_memory_usage.get())
                        .build();
                    model.append(&row_model);

                    row_model
                } else {
                    let model: gio::ListModel = model.clone().into();
                    let row_model = model
                        .item(pos.unwrap())
                        .unwrap()
                        .downcast::<RowModel>()
                        .unwrap();

                    // The app might have been stopped and restarted between updates, so always
                    // reset the primary PID, and repopulate the list of child processes.
                    row_model.set_pid(primary_pid);
                    row_model.set_cpu_usage(pp.merged_usage_stats.cpu_usage);
                    row_model.set_memory_usage(pp.merged_usage_stats.memory_usage);
                    row_model.set_disk_usage(pp.merged_usage_stats.disk_usage);
                    row_model.set_network_usage(pp.merged_usage_stats.network_usage);
                    row_model.set_gpu_usage(pp.merged_usage_stats.gpu_usage);
                    row_model.set_gpu_memory_usage(pp.merged_usage_stats.gpu_memory_usage);

                    row_model
                };

                let children = row_model.children().clone();
                if let Some(_) = primary_process {
                    let root_model = glib_clone!(self.processes_root_model);
                    if let Some(model) = find_pid_in_process_tree(root_model, primary_pid) {
                        if model.pid() != row_model.pid() || children.n_items() == 0 {
                            children.remove_all();
                            children.append(&model);
                        }

                        let icon = app
                            .icon
                            .as_ref()
                            .map(|i| i.as_ref())
                            .unwrap_or("application-x-executable-symbolic");

                        model.set_icon(icon);
                        update_icons(model.children().clone(), icon);
                    } else {
                        children.remove_all();

                        g_critical!(
                            "MissionCenter::AppsPage",
                            "Failed to find process in process tree, for App {}",
                            app.name.as_ref()
                        );
                    }
                } else {
                    children.remove_all();

                    g_critical!(
                        "MissionCenter::AppsPage",
                        "Failed to find process in process tree, for App {}",
                        app.name.as_ref()
                    );
                }
            }

            self.process_tree.set(process_tree);
            self.apps.set(apps);
        }

        pub fn update_processes_models(&self) {
            use crate::glib_clone;

            let process_tree = self.process_tree.take();
            let processes_root_model = glib_clone!(self.processes_root_model);

            Self::update_process_model(self, processes_root_model, &process_tree);

            self.process_tree.set(process_tree);
        }

        pub fn column_compare_entries_by(
            &self,
            lhs: &glib::Object,
            rhs: &glib::Object,
            compare_fn: fn(&row_model::RowModel, &row_model::RowModel) -> std::cmp::Ordering,
        ) -> std::cmp::Ordering {
            use row_model::{ContentType, RowModel, SectionType};
            use std::cmp::*;

            let lhs = lhs.downcast_ref::<RowModel>();
            if lhs.is_none() {
                return Ordering::Equal.into();
            }
            let lhs = lhs.unwrap();

            let rhs = rhs.downcast_ref::<RowModel>();
            if rhs.is_none() {
                return Ordering::Equal.into();
            }
            let rhs = rhs.unwrap();

            let sort_order = self
                .column_view
                .sorter()
                .and_downcast_ref::<gtk::ColumnViewSorter>()
                .and_then(|sorter| Some(sorter.primary_sort_order()))
                .unwrap_or(gtk::SortType::Ascending);

            let (ord_less, ord_greater) = if sort_order == gtk::SortType::Ascending {
                (Ordering::Less, Ordering::Greater)
            } else {
                (Ordering::Greater, Ordering::Less)
            };

            if lhs.content_type() == ContentType::SectionHeader as u8 {
                if lhs.section_type() == SectionType::Apps as u8 {
                    return ord_less;
                }

                if lhs.section_type() == SectionType::Processes as u8 {
                    return if rhs.content_type() == ContentType::Process as u8 {
                        ord_less
                    } else {
                        ord_greater
                    };
                }
            }

            if rhs.content_type() == ContentType::SectionHeader as u8 {
                if rhs.section_type() == SectionType::Apps as u8 {
                    return ord_greater;
                }

                if rhs.section_type() == SectionType::Processes as u8 {
                    return if lhs.content_type() == ContentType::Process as u8 {
                        ord_greater
                    } else {
                        ord_less
                    };
                }
            }

            if lhs.content_type() == ContentType::App as u8 {
                if rhs.content_type() == ContentType::App as u8 {
                    return compare_fn(lhs, rhs);
                }

                if rhs.content_type() == ContentType::Process as u8 {
                    return ord_less;
                }
            }

            if lhs.content_type() == ContentType::Process as u8 {
                if rhs.content_type() == ContentType::Process as u8 {
                    return compare_fn(lhs, rhs);
                }

                if rhs.content_type() == ContentType::App as u8 {
                    return ord_greater;
                }
            }

            Ordering::Equal
        }

        pub fn set_up_root_model(&self) -> ListStore {
            use row_model::{ContentType, RowModel, RowModelBuilder, SectionType};

            let apps_section_header = RowModelBuilder::new()
                .name(&i18n("Apps"))
                .content_type(ContentType::SectionHeader)
                .section_type(SectionType::Apps)
                .show_expander(false)
                .build();

            let processes_section_header = RowModelBuilder::new()
                .name(&i18n("Processes"))
                .content_type(ContentType::SectionHeader)
                .section_type(SectionType::Processes)
                .show_expander(false)
                .build();

            let root_model = ListStore::new::<RowModel>();
            root_model.append(&apps_section_header);
            root_model.append(&processes_section_header);

            root_model
        }

        pub fn set_up_tree_model(&self, model: gio::ListModel) -> gtk::TreeListModel {
            use crate::glib_clone;

            use row_model::{ContentType, RowModel, SectionType};

            let this = self.obj().downgrade();
            gtk::TreeListModel::new(model, false, true, move |model_entry| {
                let row_model = model_entry.downcast_ref::<RowModel>();
                if row_model.is_none() {
                    return None;
                }
                let row_model = row_model.unwrap();

                let this = this.upgrade();
                if this.is_none() {
                    return None;
                }
                let this = this.unwrap();
                let this = this.imp();

                let content_type = row_model.content_type();

                if content_type == ContentType::SectionHeader as u8 {
                    if row_model.section_type() == SectionType::Apps as u8 {
                        let apps_model = glib_clone!(this.apps_model);
                        return Some(apps_model.into());
                    }

                    if row_model.section_type() == SectionType::Processes as u8 {
                        let processes_model = glib_clone!(this.processes_root_model);
                        return Some(processes_model.into());
                    }

                    return None;
                }

                if content_type == ContentType::Process as u8
                    || content_type == ContentType::App as u8
                {
                    return Some(row_model.children().clone().into());
                }

                None
            })
        }

        pub fn set_up_filter_model(&self, model: gio::ListModel) -> gtk::FilterListModel {
            use glib::g_critical;
            use row_model::{ContentType, RowModel};

            let Some(window) = app!().window() else {
                g_critical!(
                    "MissionCenter::AppsPage",
                    "Failed to get MissionCenterWindow instance; searching and filtering will not function"
                );
                return gtk::FilterListModel::new(Some(model), None::<gtk::CustomFilter>);
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

                    let row_model = match obj
                        .downcast_ref::<gtk::TreeListRow>()
                        .and_then(|row| row.item())
                        .and_then(|item| item.downcast::<RowModel>().ok())
                    {
                        None => return false,
                        Some(vm) => vm,
                    };
                    if row_model.content_type() == ContentType::SectionHeader as u8 {
                        return true;
                    }

                    let entry_name = row_model.name().to_lowercase();
                    let pid = row_model.pid().to_string();
                    let search_query = window.header_search_entry.text().to_lowercase();

                    if entry_name.contains(&search_query) || pid.contains(&search_query) {
                        return true;
                    }

                    if search_query.contains(&entry_name) || search_query.contains(&pid) {
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
                        if !window.apps_page_active() {
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

        pub fn set_up_sort_model(&self, model: gio::ListModel) -> gtk::SortListModel {
            let this = self.obj().downgrade();

            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        lhs.name().to_lowercase().cmp(&rhs.name().to_lowercase())
                    })
                    .into()
            });
            self.name_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = lhs.pid();
                        let rhs = rhs.pid();

                        lhs.cmp(&rhs)
                    })
                    .into()
            });
            self.pid_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = if let Some(merged_stats) = lhs.merged_stats() {
                            merged_stats.cpu_usage
                        } else {
                            lhs.cpu_usage()
                        };
                        let rhs = if let Some(merged_stats) = rhs.merged_stats() {
                            merged_stats.cpu_usage
                        } else {
                            rhs.cpu_usage()
                        };

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.cpu_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = if let Some(merged_stats) = lhs.merged_stats() {
                            merged_stats.memory_usage
                        } else {
                            lhs.memory_usage()
                        };

                        let rhs = if let Some(merged_stats) = rhs.merged_stats() {
                            merged_stats.memory_usage
                        } else {
                            rhs.memory_usage()
                        };

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.memory_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = if let Some(merged_stats) = lhs.merged_stats() {
                            merged_stats.disk_usage
                        } else {
                            lhs.disk_usage()
                        };

                        let rhs = if let Some(merged_stats) = rhs.merged_stats() {
                            merged_stats.disk_usage
                        } else {
                            rhs.disk_usage()
                        };

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.disk_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = if let Some(merged_stats) = lhs.merged_stats() {
                            merged_stats.gpu_usage
                        } else {
                            lhs.gpu_usage()
                        };

                        let rhs = if let Some(merged_stats) = rhs.merged_stats() {
                            merged_stats.gpu_usage
                        } else {
                            rhs.gpu_usage()
                        };

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.gpu_usage_column.set_sorter(Some(&sorter));

            let this = self.obj().downgrade();
            let sorter = gtk::CustomSorter::new(move |lhs, rhs| {
                use std::cmp::Ordering;

                let this = this.upgrade();
                if this.is_none() {
                    return Ordering::Equal.into();
                }
                let this = this.unwrap();

                this.imp()
                    .column_compare_entries_by(lhs, rhs, |lhs, rhs| {
                        let lhs = if let Some(merged_stats) = lhs.merged_stats() {
                            merged_stats.gpu_memory_usage
                        } else {
                            lhs.gpu_memory_usage()
                        };

                        let rhs = if let Some(merged_stats) = rhs.merged_stats() {
                            merged_stats.gpu_memory_usage
                        } else {
                            rhs.gpu_memory_usage()
                        };

                        lhs.partial_cmp(&rhs).unwrap_or(Ordering::Equal)
                    })
                    .into()
            });
            self.gpu_memory_column.set_sorter(Some(&sorter));

            let column_view_sorter = self.column_view.sorter();
            if let Some(column_view_sorter) = column_view_sorter.as_ref() {
                column_view_sorter.connect_changed({
                    let this = self.obj().downgrade();
                    move |sorter, _| {
                        use glib::g_critical;

                        let settings = settings!();

                        let this = match this.upgrade() {
                            None => return,
                            Some(this) => this,
                        };

                        if let Some(sorter) = sorter.downcast_ref::<gtk::ColumnViewSorter>() {
                            let sort_column = sorter
                                .primary_sort_column()
                                .as_ref()
                                .and_then(|c| Some(c.as_ptr() as usize))
                                .unwrap_or_default();

                            let nc = this.imp().name_column.as_ptr() as usize;
                            let pc = this.imp().pid_column.as_ptr() as usize;
                            let cc = this.imp().cpu_column.as_ptr() as usize;
                            let mc = this.imp().memory_column.as_ptr() as usize;
                            let dc = this.imp().disk_column.as_ptr() as usize;
                            let gc = this.imp().gpu_usage_column.as_ptr() as usize;
                            let gm = this.imp().gpu_memory_column.as_ptr() as usize;

                            if let Err(e) = if sort_column == nc {
                                settings.set_enum("apps-page-sorting-column", 0)
                            } else if sort_column == pc {
                                settings.set_enum("apps-page-sorting-column", 1)
                            } else if sort_column == cc {
                                settings.set_enum("apps-page-sorting-column", 2)
                            } else if sort_column == mc {
                                settings.set_enum("apps-page-sorting-column", 3)
                            } else if sort_column == dc {
                                settings.set_enum("apps-page-sorting-column", 4)
                            } else if sort_column == gc {
                                settings.set_enum("apps-page-sorting-column", 5)
                            } else if sort_column == gm {
                                settings.set_enum("apps-page-sorting-column", 6)
                            } else {
                                g_critical!(
                                    "MissionCenter::AppsPage",
                                    "Unknown column sorting encountered"
                                );
                                Ok(())
                            } {
                                g_critical!(
                                    "MissionCenter::AppsPage",
                                    "Failed to save column sorting: {}",
                                    e
                                );
                                return;
                            }

                            let sort_order = sorter.primary_sort_order();
                            if let Err(e) = settings.set_enum(
                                "apps-page-sorting-order",
                                match sort_order {
                                    gtk::SortType::Ascending => 0,
                                    gtk::SortType::Descending => 1,
                                    _ => 0,
                                },
                            ) {
                                g_critical!(
                                    "MissionCenter::AppsPage",
                                    "Failed to save column sorting: {}",
                                    e
                                );
                                return;
                            }
                        }
                    }
                });
            }
            let tree_list_sorter = gtk::TreeListRowSorter::new(column_view_sorter);
            self.tree_list_sorter.set(Some(tree_list_sorter.clone()));

            gtk::SortListModel::new(Some(model), Some(tree_list_sorter))
        }

        pub fn set_up_model(&self) {
            let root_model = self.set_up_root_model();
            let tree_model = self.set_up_tree_model(root_model.into());
            let filter_model = self.set_up_filter_model(tree_model.into());
            let sort_model = self.set_up_sort_model(filter_model.into());

            self.column_view
                .set_model(Some(&gtk::SingleSelection::new(Some(sort_model))));

            let settings = settings!();

            let remember_sorting = settings.boolean("apps-page-remember-sorting");
            if remember_sorting {
                let column = settings.enum_("apps-page-sorting-column");
                let order = settings.enum_("apps-page-sorting-order");

                let column = match column {
                    0 => &self.name_column,
                    1 => &self.pid_column,
                    2 => &self.cpu_column,
                    3 => &self.memory_column,
                    4 => &self.disk_column,
                    5 => &self.gpu_usage_column,
                    6 => &self.gpu_memory_column,
                    255 => return,
                    _ => {
                        glib::g_critical!(
                            "MissionCenter::AppsPage",
                            "Unknown column retrieved from settings, sorting by name as a fallback"
                        );
                        &self.name_column
                    }
                };

                let order = match order {
                    0 => gtk::SortType::Ascending,
                    1 => gtk::SortType::Descending,
                    255 => return,
                    _ => {
                        glib::g_critical!(
                            "MissionCenter::AppsPage",
                            "Unknown column sorting order retrieved from settings, sorting in ascending order as a fallback"
                        );
                        gtk::SortType::Ascending
                    }
                };

                self.column_view.sort_by_column(Some(column), order);
            }
        }

        pub fn configure_column_header(
            &self,
            column_header: &gtk::Widget,
            name: &str,
            heading: &str,
            align: gtk::Align,
        ) -> (Option<gtk::Widget>, column_header::ColumnHeader) {
            let column_view_box = column_header
                .first_child()
                .unwrap()
                .downcast::<gtk::Box>()
                .unwrap();
            column_view_box.first_child().unwrap().set_visible(false);

            let header = column_header::ColumnHeader::new(heading, name, align);
            column_view_box.append(&header);

            (column_header.next_sibling(), header)
        }

        pub fn update_column_headers(&self, readings: &crate::sys_info_v2::Readings) {
            let column_header_cpu = self.column_header_cpu.take();
            if let Some(column_header_cpu) = &column_header_cpu {
                column_header_cpu.set_heading(format!(
                    "{}%",
                    readings
                        .cpu_dynamic_info
                        .overall_utilization_percent
                        .round()
                ));
            }
            self.column_header_cpu.set(column_header_cpu);

            let column_header_memory = self.column_header_memory.take();
            if let Some(column_header_memory) = &column_header_memory {
                let used = readings.mem_info.mem_total - readings.mem_info.mem_available;
                column_header_memory.set_heading(format!(
                    "{}%",
                    ((used * 100) as f32 / readings.mem_info.mem_total as f32).round()
                ));
            }
            self.column_header_memory.set(column_header_memory);

            let column_header_disk = self.column_header_disk.take();
            if let Some(column_header_disk) = &column_header_disk {
                let total_busy_percent = readings
                    .disks_info
                    .iter()
                    .map(|disk| disk.busy_percent)
                    .sum::<f32>();

                if readings.disks_info.len() == 0 {
                    column_header_disk.set_heading("0%");
                } else {
                    column_header_disk.set_heading(format!(
                        "{}%",
                        (total_busy_percent / readings.disks_info.len() as f32).round()
                    ));
                }
            }
            self.column_header_disk.set(column_header_disk);

            let column_header_gpu = self.column_header_gpu_usage.take();
            if let Some(column_header_gpu) = &column_header_gpu {
                let avg = readings
                    .gpu_dynamic_info
                    .iter()
                    .map(|g| g.util_percent)
                    .sum::<u32>() as f32
                    / readings.gpu_dynamic_info.len() as f32;
                column_header_gpu.set_heading(format!("{:.0}%", avg.round()));
            }
            self.column_header_gpu_usage.set(column_header_gpu);

            let column_header_gpu_mem = self.column_header_gpu_memory_usage.take();
            if let Some(column_header_gpu_mem) = &column_header_gpu_mem {
                let avg = readings
                    .gpu_dynamic_info
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        let total_memory = readings.gpu_static_info[i].total_memory;
                        if total_memory == 0 {
                            return 0;
                        }
                        (g.used_memory * 100) / total_memory
                    })
                    .sum::<u64>() as f32
                    / readings.gpu_dynamic_info.len() as f32;
                column_header_gpu_mem.set_heading(format!("{:.0}%", avg.round()));
            }
            self.column_header_gpu_memory_usage
                .set(column_header_gpu_mem);
        }

        fn update_process_model(this: &AppsPage, model: ListStore, process: &Process) {
            use crate::apps_page::row_model::{ContentType, RowModel, RowModelBuilder};

            let mut to_remove = Vec::new();
            for i in 0..model.n_items() {
                let current = model.item(i).unwrap().downcast::<RowModel>();
                if current.is_err() {
                    continue;
                }
                let current = current.unwrap();

                if !process.children.contains_key(&(current.pid())) {
                    to_remove.push(i);
                }
            }

            for (i, to_remove_i) in to_remove.iter().enumerate() {
                let to_remove_i = (*to_remove_i as usize - i) as _;
                model.remove(to_remove_i);
            }

            for (pid, child) in &process.children {
                let pos = if model.n_items() > 0 {
                    model.find_with_equal_func(|current| {
                        let current = current.downcast_ref::<RowModel>();
                        if current.is_none() {
                            return false;
                        }
                        let current = current.unwrap();
                        current.pid() == *pid
                    })
                } else {
                    None
                };

                let entry_name = if !child.exe.as_ref().is_empty() {
                    let entry_name = std::path::Path::new(child.exe.as_ref())
                        .file_name()
                        .map(|name| name.to_str().unwrap_or(child.name.as_ref()))
                        .unwrap_or(child.name.as_ref());
                    if entry_name.starts_with("wine") {
                        if child.cmd.is_empty() {
                            child.name.as_ref()
                        } else {
                            child.cmd[0]
                                .as_ref()
                                .split("\\")
                                .last()
                                .unwrap_or(child.name.as_ref())
                                .split("/")
                                .last()
                                .unwrap_or(child.name.as_ref())
                        }
                    } else {
                        entry_name
                    }
                } else {
                    child.name.as_ref()
                };

                let child_model = if pos.is_none() {
                    let (cpu_usage, mem_usage, net_usage, disk_usage, gpu_usage, gpu_mem_usage) =
                        if this.use_merge_stats.get() {
                            (
                                child.merged_usage_stats.cpu_usage,
                                child.merged_usage_stats.memory_usage,
                                child.merged_usage_stats.network_usage,
                                child.merged_usage_stats.disk_usage,
                                child.merged_usage_stats.gpu_usage,
                                child.merged_usage_stats.gpu_memory_usage,
                            )
                        } else {
                            (
                                child.usage_stats.cpu_usage,
                                child.usage_stats.memory_usage,
                                child.usage_stats.network_usage,
                                child.usage_stats.disk_usage,
                                child.usage_stats.gpu_usage,
                                child.usage_stats.gpu_memory_usage,
                            )
                        };

                    let row_model = RowModelBuilder::new()
                        .name(entry_name)
                        .content_type(ContentType::Process)
                        .icon("application-x-executable-symbolic")
                        .pid(*pid)
                        .cpu_usage(cpu_usage)
                        .memory_usage(mem_usage)
                        .disk_usage(disk_usage)
                        .network_usage(net_usage)
                        .gpu_usage(gpu_usage)
                        .gpu_mem_usage(gpu_mem_usage)
                        .max_cpu_usage(this.max_cpu_usage.get())
                        .max_memory_usage(this.max_memory_usage.get())
                        .max_gpu_memory_usage(this.max_gpu_memory_usage.get())
                        .build();

                    row_model.set_merged_stats(&child.merged_usage_stats);

                    model.append(&row_model);
                    row_model.children().clone()
                } else {
                    let row_model = model
                        .item(pos.unwrap())
                        .unwrap()
                        .downcast::<RowModel>()
                        .unwrap();

                    let (cpu_usage, mem_usage, net_usage, disk_usage, gpu_usage, gpu_mem_usage) =
                        if this.use_merge_stats.get() {
                            (
                                child.merged_usage_stats.cpu_usage,
                                child.merged_usage_stats.memory_usage,
                                child.merged_usage_stats.network_usage,
                                child.merged_usage_stats.disk_usage,
                                child.merged_usage_stats.gpu_usage,
                                child.merged_usage_stats.gpu_memory_usage,
                            )
                        } else {
                            (
                                child.usage_stats.cpu_usage,
                                child.usage_stats.memory_usage,
                                child.usage_stats.network_usage,
                                child.usage_stats.disk_usage,
                                child.usage_stats.gpu_usage,
                                child.usage_stats.gpu_memory_usage,
                            )
                        };

                    row_model.set_icon("application-x-executable-symbolic");
                    row_model.set_cpu_usage(cpu_usage);
                    row_model.set_memory_usage(mem_usage);
                    row_model.set_disk_usage(disk_usage);
                    row_model.set_network_usage(net_usage);
                    row_model.set_gpu_usage(gpu_usage);
                    row_model.set_gpu_memory_usage(gpu_mem_usage);

                    row_model.set_merged_stats(&child.merged_usage_stats);

                    row_model.children().clone()
                };

                Self::update_process_model(this, child_model, child);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AppsPage {
        const NAME: &'static str = "AppsPage";
        type Type = super::AppsPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            list_item::ListItem::ensure_type();

            row_model::RowModel::ensure_type();

            column_header::ColumnHeader::ensure_type();
            pid_column::PidColumn::ensure_type();
            stat_column::StatColumn::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppsPage {
        fn constructed(&self) {
            self.parent_constructed();

            self.configure_actions();

            let settings = settings!();

            self.use_merge_stats
                .set(settings.boolean("apps-page-merged-process-stats"));

            settings.connect_changed(Some("apps-page-merged-process-stats"), {
                let this = self.obj().downgrade();
                move |settings, _| {
                    if let Some(this) = this.upgrade() {
                        this.imp()
                            .use_merge_stats
                            .set(settings.boolean("apps-page-merged-process-stats"));
                    }
                }
            });
        }
    }

    impl WidgetImpl for AppsPage {
        fn realize(&self) {
            self.parent_realize();

            let list_item_widget = self.column_view.first_child().unwrap();

            // Remove padding added in GTK 4.12
            list_item_widget.add_css_class("app-list-header");

            let column_view_title = list_item_widget.first_child().unwrap();
            let (column_view_title, column_header_name) = self.configure_column_header(
                &column_view_title,
                &i18n("Name"),
                "",
                gtk::Align::Start,
            );
            let (column_view_title, column_header_pid) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("PID"),
                "",
                gtk::Align::End,
            );
            let (column_view_title, column_header_cpu) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("CPU"),
                "0%",
                gtk::Align::End,
            );
            let (column_view_title, column_header_memory) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("Memory"),
                "0%",
                gtk::Align::End,
            );
            let (column_view_title, column_header_disk) = self.configure_column_header(
                &column_view_title.unwrap(),
                &i18n("Drive"),
                "0%",
                gtk::Align::End,
            );

            self.column_header_name.set(Some(column_header_name));
            self.column_header_pid.set(Some(column_header_pid));
            self.column_header_cpu.set(Some(column_header_cpu));
            self.column_header_memory.set(Some(column_header_memory));
            self.column_header_disk.set(Some(column_header_disk));

            if let Some(column_view_title) = column_view_title {
                let (column_view_title, column_header_gpu_usage) = self.configure_column_header(
                    &column_view_title,
                    &i18n("GPU Usage"),
                    "0%",
                    gtk::Align::End,
                );
                self.column_header_gpu_usage
                    .set(Some(column_header_gpu_usage));

                if let Some(column_view_title) = column_view_title {
                    let (_, column_header_gpu_memory) = self.configure_column_header(
                        &column_view_title,
                        &i18n("GPU Mem"),
                        "0%",
                        gtk::Align::End,
                    );

                    self.column_header_gpu_memory_usage
                        .set(Some(column_header_gpu_memory));
                }
            }
        }
    }

    impl BoxImpl for AppsPage {}
}

glib::wrapper! {
    pub struct AppsPage(ObjectSubclass<imp::AppsPage>)
        @extends gtk::Box, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl AppsPage {
    pub fn set_initial_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        use std::collections::HashMap;

        let this = self.imp();

        if readings.gpu_static_info.is_empty() {
            this.column_view.remove_column(&this.gpu_usage_column);
            this.column_view.remove_column(&this.gpu_memory_column);
        } else {
            // Intel GPUs don't have memory information
            if readings
                .gpu_static_info
                .iter()
                .all(|g| g.vendor_id == 0x8086)
            {
                this.column_view.remove_column(&this.gpu_memory_column);
            } else {
                this.max_gpu_memory_usage.set(
                    readings
                        .gpu_static_info
                        .iter()
                        .map(|g| g.total_memory as f32)
                        .sum(),
                );
            }
        }

        this.max_cpu_usage
            .set(readings.cpu_static_info.logical_cpu_count as f32 * 100.0);
        this.max_memory_usage
            .set(readings.mem_info.mem_total as f32);

        let mut apps = HashMap::new();
        std::mem::swap(&mut apps, &mut readings.running_apps);
        this.apps.set(apps);

        let mut process_tree = Process::default();
        std::mem::swap(&mut process_tree, &mut readings.process_tree);
        this.process_tree.set(process_tree);

        this.set_up_model();

        this.update_processes_models();
        this.update_app_model();
        this.update_column_headers(readings);

        true
    }

    pub fn update_readings(&self, readings: &mut crate::sys_info_v2::Readings) -> bool {
        let this = self.imp();

        let mut apps = this.apps.take();
        std::mem::swap(&mut apps, &mut readings.running_apps);
        this.apps.set(apps);

        let mut process_tree = this.process_tree.take();
        std::mem::swap(&mut process_tree, &mut readings.process_tree);
        this.process_tree.set(process_tree);

        this.update_processes_models();
        this.update_app_model();
        this.update_column_headers(readings);

        let sorter = this.tree_list_sorter.take();
        if let Some(sorter) = sorter.as_ref() {
            sorter.changed(gtk::SorterChange::Different)
        }
        this.tree_list_sorter.set(sorter);

        true
    }
}
