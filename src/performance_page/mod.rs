/* performance_page/view_models
 *
 * Copyright 2024 Romeo Calota
 * Copyright 2024 jojo2357
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

use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
};

use adw::{prelude::*, subclass::prelude::*};
use glib::{ParamSpec, Properties, Value};
use gtk::{
    gdk, gio,
    glib::{self, g_critical},
};

use widgets::{GraphWidget, SidebarDropHint};

use crate::{
    i18n::*,
    settings,
    sys_info_v2::{DiskType, NetworkDevice},
};

mod cpu;
mod disk;
mod fan;
mod gpu;
mod memory;
mod network;
mod summary_graph;
mod widgets;

type SummaryGraph = summary_graph::SummaryGraph;
type CpuPage = cpu::PerformancePageCpu;
type DiskPage = disk::PerformancePageDisk;
type MemoryPage = memory::PerformancePageMemory;
type NetworkPage = network::PerformancePageNetwork;
type GpuPage = gpu::PerformancePageGpu;
type FanPage = fan::PerformancePageFan;

trait PageExt {
    fn infobar_collapsed(&self);
    fn infobar_uncollapsed(&self);
}

mod imp {
    use super::*;

    // GNOME color palette: Blue 4
    const CPU_BASE_COLOR: [u8; 3] = [0x1c, 0x71, 0xd8];
    // GNOME color palette: Blue 2
    const MEMORY_BASE_COLOR: [u8; 3] = [0x62, 0xa0, 0xea];
    // GNOME color palette: Green 5
    const DISK_BASE_COLOR: [u8; 3] = [0x26, 0xa2, 0x69];
    // GNOME color palette: Purple 1
    const NETWORK_BASE_COLOR: [u8; 3] = [0xdc, 0x8a, 0xdd];
    // GNOME color palette: Purple 4
    const FAN_BASE_COLOR: [u8; 3] = [0x81, 0x3d, 0x9c];
    // GNOME color palette: Red 1
    const GPU_BASE_COLOR: [u8; 3] = [0xf6, 0x61, 0x51];

    enum Pages {
        Cpu((SummaryGraph, CpuPage)),
        Memory((SummaryGraph, MemoryPage)),
        Disk(HashMap<String, (SummaryGraph, DiskPage)>),
        Network(HashMap<String, (SummaryGraph, NetworkPage)>),
        Gpu(HashMap<String, (SummaryGraph, GpuPage)>),
        Fan(HashMap<String, (SummaryGraph, FanPage)>),
    }

    #[derive(Properties)]
    #[properties(wrapper_type = super::PerformancePage)]
    #[derive(gtk::CompositeTemplate)]
    #[template(resource = "/io/missioncenter/MissionCenter/ui/performance_page/page.ui")]
    pub struct PerformancePage {
        #[template_child]
        pub breakpoint: TemplateChild<adw::Breakpoint>,
        #[template_child]
        pub page_content: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub page_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub info_bar: TemplateChild<adw::Bin>,

        #[property(get = Self::sidebar, set = Self::set_sidebar)]
        pub sidebar: Cell<gtk::ListBox>,
        #[property(get, set = Self::set_sidebar_edit_mode)]
        pub sidebar_edit_mode: Cell<bool>,
        #[property(get, set)]
        summary_mode: Cell<bool>,
        #[property(name = "infobar-visible", get = Self::infobar_visible, set = Self::set_infobar_visible, type = bool
        )]
        _infobar_visible: [u8; 0],
        #[property(name = "info-button-visible", get = Self::info_button_visible, type = bool)]
        _info_button_visible: [u8; 0],

        breakpoint_applied: Cell<bool>,

        pages: Cell<Vec<Pages>>,
        pub summary_graphs: Cell<HashMap<SummaryGraph, gtk::DragSource>>,

        action_group: Cell<gio::SimpleActionGroup>,
        context_menu_view_actions: Cell<HashMap<String, gio::SimpleAction>>,
        current_view_action: Cell<gio::SimpleAction>,
    }

    impl Default for PerformancePage {
        fn default() -> Self {
            Self {
                breakpoint: Default::default(),
                page_content: Default::default(),
                page_stack: Default::default(),
                info_bar: Default::default(),

                sidebar: Cell::new(gtk::ListBox::new()),
                sidebar_edit_mode: Cell::new(false),
                summary_mode: Cell::new(false),
                _infobar_visible: [0; 0],
                _info_button_visible: [0; 0],

                breakpoint_applied: Cell::new(false),

                pages: Cell::new(Vec::new()),
                summary_graphs: Cell::new(HashMap::new()),

                action_group: Cell::new(gio::SimpleActionGroup::new()),
                context_menu_view_actions: Cell::new(HashMap::new()),
                current_view_action: Cell::new(gio::SimpleAction::new("", None)),
            }
        }
    }

    impl PerformancePage {
        pub fn sidebar(&self) -> gtk::ListBox {
            unsafe { &*self.sidebar.as_ptr() }.clone()
        }

        fn set_sidebar(&self, lb: &gtk::ListBox) {
            let this = self.obj().as_ref().clone();

            Self::configure_actions(&this);
            lb.connect_row_selected(move |_, selected_row| {
                if let Some(row) = selected_row {
                    let child = match row.child() {
                        Some(child) => child,
                        None => {
                            g_critical!(
                                "MissionCenter::PerformancePage",
                                "Failed to get child of selected row"
                            );

                            return;
                        }
                    };

                    let child_name = child.widget_name();
                    let page_name = child_name.as_str();

                    let imp = this.imp();

                    let actions = imp.context_menu_view_actions.take();
                    if let Some(new_action) = actions.get(page_name) {
                        let prev_action = imp.current_view_action.replace(new_action.clone());
                        prev_action.set_state(&glib::Variant::from(false));
                        new_action.set_state(&glib::Variant::from(true));
                    }

                    imp.context_menu_view_actions.set(actions);
                    imp.page_stack.set_visible_child_name(page_name);

                    settings!()
                        .set_string("performance-selected-page", page_name)
                        .unwrap_or_else(|_| {
                            glib::g_warning!(
                                "MissionCenter::PerformancePage",
                                "Failed to set performance-selected-page setting"
                            );
                        });
                }
            });

            let drop_target = gtk::DropTarget::new(glib::Type::INVALID, gdk::DragAction::all());
            drop_target.set_preload(true);
            drop_target.set_types(&[glib::Type::I32]);
            drop_target.connect_motion({
                let this = self.obj().downgrade();
                move |_, _, y| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return gdk::DragAction::empty(),
                    };

                    let sidebar = this.imp().sidebar();

                    let summary_graphs = this.imp().summary_graphs.take();

                    for graph in summary_graphs.keys() {
                        graph.hide_drop_hint();
                    }

                    let mut drop_hint_bottom = false;
                    let row_count = summary_graphs.len() as i32;
                    let graph = match sidebar
                        .row_at_y(y as i32)
                        .and_then(|row| row.child())
                        .and_then(|child| child.downcast_ref::<SummaryGraph>().cloned())
                    {
                        Some(graph) => graph,
                        None => {
                            if y < 10. {
                                this.imp().summary_graphs.set(summary_graphs);
                                return gdk::DragAction::empty();
                            }

                            drop_hint_bottom = true;

                            let mut target_graph = None;

                            for i in (0..row_count).rev() {
                                let row = match sidebar.row_at_index(i) {
                                    Some(row) => row,
                                    None => continue,
                                };

                                if !row.is_visible() {
                                    continue;
                                }

                                match row
                                    .child()
                                    .and_then(|child| child.downcast_ref::<SummaryGraph>().cloned())
                                {
                                    Some(graph) => {
                                        target_graph = Some(graph);
                                        break;
                                    }
                                    None => {
                                        this.imp().summary_graphs.set(summary_graphs);
                                        return gdk::DragAction::empty();
                                    }
                                }
                            }

                            match target_graph {
                                Some(graph) => graph,
                                None => {
                                    this.imp().summary_graphs.set(summary_graphs);
                                    return gdk::DragAction::empty();
                                }
                            }
                        }
                    };

                    if drop_hint_bottom {
                        graph.show_drop_hint_bottom();
                    } else {
                        graph.show_drop_hint_top();
                    }

                    this.imp().summary_graphs.set(summary_graphs);

                    gdk::DragAction::MOVE
                }
            });
            drop_target.connect_leave({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    let summary_graphs = this.imp().summary_graphs.take();
                    for graph in summary_graphs.keys() {
                        graph.hide_drop_hint();
                    }
                    this.imp().summary_graphs.set(summary_graphs);
                }
            });
            drop_target.connect_drop({
                let this = self.obj().downgrade();
                move |_, value, _, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return false,
                    };

                    let row_index: i32 = match value.get() {
                        Ok(value) => value,
                        Err(_) => return false,
                    };

                    let sidebar = this.sidebar();

                    let dragged_row = match sidebar.row_at_index(row_index) {
                        Some(row) => row,
                        None => return false,
                    };

                    let dragged_graph = match dragged_row
                        .child()
                        .and_then(|child| child.downcast_ref::<SummaryGraph>().cloned())
                    {
                        Some(graph) => graph,
                        None => return false,
                    };

                    let summary_graphs = this.imp().summary_graphs.take();

                    for graph in summary_graphs.keys() {
                        if graph.is_drop_hint_visible() {
                            if let Some(target_row) = graph
                                .parent()
                                .and_then(|p| p.downcast_ref::<gtk::ListBoxRow>().cloned())
                            {
                                dragged_graph.set_visible(true);
                                let drag_controller = match summary_graphs.get(&dragged_graph) {
                                    Some(drag_controller) => drag_controller.clone(),
                                    None => {
                                        this.imp().summary_graphs.set(summary_graphs);
                                        g_critical!(
                                            "MissionCenter::PerformancePage",
                                            "Drag controller is missing from summary graphs"
                                        );
                                        return false;
                                    }
                                };

                                sidebar.remove(&dragged_row);
                                drop(dragged_row);

                                let new_index = if graph.is_drop_hint_bottom() {
                                    target_row.index() + 1
                                } else {
                                    target_row.index()
                                };

                                sidebar.insert(&dragged_graph, new_index);
                                sidebar
                                    .row_at_index(new_index)
                                    .and_then(|row| Some(row.add_controller(drag_controller)));
                            }

                            break;
                        }
                    }

                    this.imp().summary_graphs.set(summary_graphs);

                    true
                }
            });
            lb.add_controller(drop_target);

            self.sidebar.set(lb.clone())
        }

        fn set_sidebar_edit_mode(&self, edit_mode: bool) {
            let active_page_name = self.page_stack.visible_child_name().unwrap_or_default();

            let summary_graphs = self.summary_graphs.take();
            let graph_count = summary_graphs.len() as i32;
            for (graph, drag_source) in &summary_graphs {
                graph.set_edit_mode(edit_mode);

                if edit_mode {
                    drag_source.set_actions(gdk::DragAction::MOVE);
                } else {
                    drag_source.set_actions(gdk::DragAction::empty());
                }

                if !graph.is_visible() && active_page_name == graph.widget_name() {
                    if let Some(index) = graph
                        .parent()
                        .and_then(|parent| parent.downcast_ref::<gtk::ListBoxRow>().cloned())
                        .and_then(|row| Some(row.index()))
                    {
                        let mut forward_index = index + 1;
                        let mut backward_index = index - 1;
                        let mut new_row = None;

                        fn visible_row(
                            sidebar: &gtk::ListBox,
                            index: i32,
                        ) -> Option<gtk::ListBoxRow> {
                            sidebar.row_at_index(index).and_then(|row| {
                                if !row.is_visible() {
                                    None
                                } else {
                                    Some(row)
                                }
                            })
                        }

                        // Try to find the nearest visible entry
                        let sidebar = self.sidebar();
                        loop {
                            if forward_index >= graph_count && backward_index < 0 {
                                break;
                            }

                            // Go to the next visible entry
                            loop {
                                if forward_index >= graph_count {
                                    break;
                                }

                                match visible_row(&sidebar, forward_index) {
                                    Some(row) => {
                                        new_row = Some(row);
                                        break;
                                    }
                                    None => {
                                        forward_index += 1;
                                        continue;
                                    }
                                }
                            }

                            if let Some(row) = new_row {
                                self.sidebar().select_row(Some(&row));
                                break;
                            }

                            // Go to the previous visible entry
                            loop {
                                if backward_index < 0 {
                                    break;
                                }

                                match visible_row(&sidebar, backward_index) {
                                    Some(row) => {
                                        new_row = Some(row);
                                        break;
                                    }
                                    None => {
                                        backward_index -= 1;
                                        continue;
                                    }
                                }
                            }

                            if let Some(row) = new_row {
                                self.sidebar().select_row(Some(&row));
                                break;
                            }
                        }
                    }
                }
            }
            self.summary_graphs.set(summary_graphs);

            self.sidebar_edit_mode.set(edit_mode);
        }

        fn infobar_visible(&self) -> bool {
            self.page_content.shows_sidebar()
        }

        fn set_infobar_visible(&self, v: bool) {
            self.page_content
                .set_show_sidebar(!self.page_content.is_collapsed() || v);
        }

        fn info_button_visible(&self) -> bool {
            self.page_content.is_collapsed()
        }
    }

    impl PerformancePage {
        fn configure_actions(this: &super::PerformancePage) {
            let actions = unsafe { &*this.imp().action_group.as_ptr() }.clone();

            let mut view_actions = HashMap::new();

            let action = gio::SimpleAction::new_stateful(
                "summary",
                None,
                &glib::Variant::from(this.imp().summary_mode.get()),
            );
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    let new_state = !this.summary_mode();
                    action.set_state(&glib::Variant::from(new_state));
                    this.set_summary_mode(new_state);
                    if !this.imp().breakpoint_applied.get() {
                        this.imp().page_content.set_show_sidebar(!new_state);
                    }
                }
            });
            actions.add_action(&action);

            let action = gio::SimpleAction::new_stateful("cpu", None, &glib::Variant::from(true));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let pages = this.pages.take();
                    for page in &pages {
                        let (graph, _) = match page {
                            Pages::Cpu(cpu_page) => cpu_page,
                            _ => continue,
                        };

                        let row = match graph.parent() {
                            Some(row) => row,
                            None => break,
                        };

                        if !row.is_visible() {
                            break;
                        }

                        this.sidebar()
                            .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                        let prev_action = this.current_view_action.replace(action.clone());
                        prev_action.set_state(&glib::Variant::from(false));
                        action.set_state(&glib::Variant::from(true));

                        break;
                    }
                    this.pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("cpu".to_string(), action.clone());
            this.imp().current_view_action.set(action);

            let action =
                gio::SimpleAction::new_stateful("memory", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let pages = this.pages.take();
                    for page in &pages {
                        let (graph, _) = match page {
                            Pages::Memory(memory_page) => memory_page,
                            _ => continue,
                        };

                        let row = match graph.parent() {
                            Some(row) => row,
                            None => break,
                        };

                        if !row.is_visible() {
                            break;
                        }

                        this.sidebar()
                            .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                        let prev_action = this.current_view_action.replace(action.clone());
                        prev_action.set_state(&glib::Variant::from(false));
                        action.set_state(&glib::Variant::from(true));

                        break;
                    }
                    this.pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("memory".to_string(), action);

            let action = gio::SimpleAction::new_stateful("disk", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let pages = this.pages.take();
                    'page_loop: for page in &pages {
                        let disk_pages = match page {
                            Pages::Disk(disk_pages) => disk_pages,
                            _ => continue,
                        };

                        for (graph, _) in disk_pages.values() {
                            let row = match graph.parent() {
                                Some(row) => row,
                                None => continue,
                            };

                            if !row.is_visible() {
                                continue;
                            }

                            this.sidebar()
                                .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                            let prev_action = this.current_view_action.replace(action.clone());
                            prev_action.set_state(&glib::Variant::from(false));
                            action.set_state(&glib::Variant::from(true));

                            break 'page_loop;
                        }

                        break;
                    }
                    this.pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("disk".to_string(), action);

            let action =
                gio::SimpleAction::new_stateful("network", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let pages = this.pages.take();
                    'page_loop: for page in &pages {
                        let net_pages = match page {
                            Pages::Network(net_pages) => net_pages,
                            _ => continue,
                        };

                        for (graph, _) in net_pages.values() {
                            let row = match graph.parent() {
                                Some(row) => row,
                                None => continue,
                            };

                            if !row.is_visible() {
                                continue;
                            }

                            this.sidebar()
                                .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                            let prev_action = this.current_view_action.replace(action.clone());
                            prev_action.set_state(&glib::Variant::from(false));
                            action.set_state(&glib::Variant::from(true));

                            break 'page_loop;
                        }

                        break;
                    }
                    this.pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("network".to_string(), action);

            let action = gio::SimpleAction::new_stateful("gpu", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let pages = this.pages.take();
                    'page_loop: for page in &pages {
                        let gpu_pages = match page {
                            Pages::Gpu(gpu_pages) => gpu_pages,
                            _ => continue,
                        };

                        for (graph, _) in gpu_pages.values() {
                            let row = match graph.parent() {
                                Some(row) => row,
                                None => continue,
                            };

                            if !row.is_visible() {
                                continue;
                            }

                            this.sidebar()
                                .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                            let prev_action = this.current_view_action.replace(action.clone());
                            prev_action.set_state(&glib::Variant::from(false));
                            action.set_state(&glib::Variant::from(true));

                            break 'page_loop;
                        }

                        break;
                    }
                    this.pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("gpu".to_string(), action);
            let action = gio::SimpleAction::new_stateful("fan", None, &glib::Variant::from(false));
            action.connect_activate({
                let this = this.downgrade();
                move |action, _| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    let pages = this.pages.take();
                    for page in &pages {
                        let fan_pages = match page {
                            Pages::Fan(fan_pages) => fan_pages,
                            _ => continue,
                        };

                        let fan_page = fan_pages.values().next();
                        if fan_page.is_none() {
                            continue;
                        }
                        let fan_page = fan_page.unwrap();

                        let row = fan_page.0.parent();
                        if row.is_none() {
                            continue;
                        }
                        let row = row.unwrap();

                        this.sidebar()
                            .select_row(row.downcast_ref::<gtk::ListBoxRow>());

                        let prev_action = this.current_view_action.replace(action.clone());
                        prev_action.set_state(&glib::Variant::from(false));
                        action.set_state(&glib::Variant::from(true));

                        break;
                    }
                    this.pages.set(pages);
                }
            });
            actions.add_action(&action);
            view_actions.insert("fan".to_string(), action);

            this.imp().context_menu_view_actions.set(view_actions);
        }

        fn configure_page<P: PageExt + IsA<gtk::Widget>>(&self, page: &P) {
            self.page_content.connect_collapsed_notify({
                let page = page.downgrade();
                move |pc| {
                    if let Some(page) = page.upgrade() {
                        if pc.is_collapsed() {
                            page.infobar_collapsed();
                        } else {
                            page.infobar_uncollapsed();
                        }
                    }
                }
            });

            self.obj()
                .as_ref()
                .bind_property("summary-mode", page, "summary-mode")
                .flags(glib::BindingFlags::SYNC_CREATE)
                .build();
        }

        fn add_to_sidebar(&self, graph: &SummaryGraph, hint: Option<i32>) {
            let sidebar = self.sidebar();

            let drag_source = gtk::DragSource::builder()
                .actions(gdk::DragAction::empty())
                .build();

            if self.sidebar_edit_mode.get() {
                graph.set_edit_mode(true);
                drag_source.set_actions(gdk::DragAction::MOVE);
            }

            let mut summary_graphs = self.summary_graphs.take();

            summary_graphs.insert(graph.clone(), drag_source.clone());

            let index = match hint {
                Some(index) => {
                    let index = index.max(0);
                    sidebar.insert(graph, index);
                    index
                }
                None => {
                    sidebar.append(graph);
                    (summary_graphs.len() - 1) as i32
                }
            };

            self.summary_graphs.set(summary_graphs);

            if let Some(row) = sidebar.row_at_index(index) {
                drag_source.connect_prepare({
                    let this = self.obj().downgrade();
                    let graph = graph.downgrade();
                    move |src, x, y| {
                        if !src.actions().contains(gdk::DragAction::MOVE) {
                            return None;
                        }

                        let this = match this.upgrade() {
                            Some(this) => this,
                            None => return None,
                        };

                        let graph = match graph.upgrade() {
                            Some(graph) => graph,
                            None => return None,
                        };

                        let row = match graph
                            .parent()
                            .and_then(|row| row.downcast_ref::<gtk::ListBoxRow>().cloned())
                        {
                            Some(row) => row,
                            None => return None,
                        };

                        this.sidebar().unselect_all();

                        let summary_graphs = this.imp().summary_graphs.take();

                        let drag_source = match summary_graphs.get(&graph) {
                            Some(drag_source) => drag_source,
                            None => {
                                this.imp().summary_graphs.set(summary_graphs);
                                g_critical!(
                                    "MissionCenter::PerformancePage",
                                    "Drag source is missing from summary graphs"
                                );
                                return None;
                            }
                        };

                        drag_source.set_icon(
                            Some(&gtk::WidgetPaintable::new(Some(&row)).current_image()),
                            x.round() as i32,
                            y.round() as i32,
                        );

                        let content_provider =
                            gdk::ContentProvider::for_value(&glib::Value::from(row.index()));

                        row.set_visible(false);
                        for sg in summary_graphs.keys() {
                            if sg.as_ptr() != graph.as_ptr() {
                                sg.parent().and_then(|p| Some(p.set_sensitive(false)));
                            }
                        }

                        this.imp().summary_graphs.set(summary_graphs);

                        Some(content_provider)
                    }
                });

                drag_source.connect_drag_end({
                    let this = self.obj().downgrade();
                    move |src, _, _| {
                        let this = match this.upgrade() {
                            Some(this) => this,
                            None => return,
                        };

                        let summary_graphs = this.imp().summary_graphs.take();
                        for graph in summary_graphs.keys() {
                            graph.parent().and_then(|p| Some(p.set_sensitive(true)));
                            graph.parent().and_then(|p| Some(p.set_visible(true)));
                            graph.hide_drop_hint();
                        }
                        this.imp().summary_graphs.set(summary_graphs);

                        src.set_icon(None::<&gtk::WidgetPaintable>, 0, 0);
                        src.set_content(None::<&gdk::ContentProvider>);

                        let this = this.imp();

                        let settings = settings!();

                        let sidebar = this.sidebar();
                        let mut row_index = -1;
                        let mut sidebar_order = String::new();
                        loop {
                            row_index += 1;
                            let row = match sidebar.row_at_index(row_index) {
                                Some(row) => row,
                                None => break,
                            };

                            let graph = match row
                                .child()
                                .and_then(|child| child.downcast_ref::<SummaryGraph>().cloned())
                            {
                                Some(graph) => graph,
                                None => continue,
                            };

                            sidebar_order.push_str(graph.widget_name().as_str());
                            sidebar_order.push(';');
                        }

                        let sidebar_order = if !sidebar_order.is_empty() {
                            &sidebar_order[..sidebar_order.len() - 1]
                        } else {
                            ""
                        };

                        settings
                            .set_string("performance-sidebar-order", sidebar_order)
                            .unwrap_or_else(|_| {
                                glib::g_warning!(
                                    "MissionCenter::PerformancePage",
                                    "Failed to set performance-sidebar-order setting"
                                );
                            });
                    }
                });

                row.add_controller(drag_source);
            }
        }

        fn set_up_cpu_page(&self, pages: &mut Vec<Pages>, readings: &crate::sys_info_v2::Readings) {
            let summary = SummaryGraph::new();
            summary.set_widget_name("cpu");

            summary.set_heading(i18n("CPU"));
            summary.set_info1("0% 0.00 GHz");
            match readings.cpu_dynamic_info.temperature.as_ref() {
                Some(v) => summary.set_info2(format!("{:.0} °C", *v)),
                _ => {}
            }

            summary.set_base_color(gdk::RGBA::new(
                CPU_BASE_COLOR[0] as f32 / 255.,
                CPU_BASE_COLOR[1] as f32 / 255.,
                CPU_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = settings!();

            summary
                .graph_widget()
                .set_data_points(settings.int("performance-page-data-points") as u32);
            summary
                .graph_widget()
                .set_smooth_graphs(settings.boolean("performance-smooth-graphs"));

            let page = CpuPage::new(&settings);
            page.set_base_color(gdk::RGBA::new(
                CPU_BASE_COLOR[0] as f32 / 255.,
                CPU_BASE_COLOR[1] as f32 / 255.,
                CPU_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            page.set_static_information(readings);

            self.configure_page(&page);

            self.page_stack.add_named(&page, Some("cpu"));
            self.add_to_sidebar(&summary, None);

            pages.push(Pages::Cpu((summary, page)));
        }

        fn set_up_memory_page(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let summary = SummaryGraph::new();
            summary.set_widget_name("memory");

            {
                let graph_widget = summary.graph_widget();

                graph_widget.set_value_range_max(readings.mem_info.mem_total as f32);
                graph_widget.set_data_set_count(2);
                graph_widget.set_filled(0, false);
                graph_widget.set_dashed(0, true);
            }

            summary.set_heading(i18n("Memory"));
            summary.set_info1("0/0 GiB (100%)");

            summary.set_base_color(gdk::RGBA::new(
                MEMORY_BASE_COLOR[0] as f32 / 255.,
                MEMORY_BASE_COLOR[1] as f32 / 255.,
                MEMORY_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = settings!();

            summary
                .graph_widget()
                .set_data_points(settings.int("performance-page-data-points") as u32);

            summary
                .graph_widget()
                .set_smooth_graphs(settings.boolean("performance-smooth-graphs"));

            let page = MemoryPage::new(&settings);
            page.set_base_color(gdk::RGBA::new(
                MEMORY_BASE_COLOR[0] as f32 / 255.,
                MEMORY_BASE_COLOR[1] as f32 / 255.,
                MEMORY_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            page.set_memory_color(gdk::RGBA::new(
                DISK_BASE_COLOR[0] as f32 / 255.,
                DISK_BASE_COLOR[1] as f32 / 255.,
                DISK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            page.set_static_information(readings);

            self.configure_page(&page);

            self.page_stack.add_named(&page, Some("memory"));
            self.add_to_sidebar(&summary, None);

            pages.push(Pages::Memory((summary, page)));
        }

        fn set_up_disk_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let mut disks = HashMap::new();
            let len = readings.disks_info.len();
            let hide_index = len == 1;
            for i in 0..len {
                let mut ret = self.create_disk_page(
                    readings,
                    if hide_index { None } else { Some(i as i32) },
                    None,
                );
                disks.insert(std::mem::take(&mut ret.0), ret.1);
            }

            pages.push(Pages::Disk(disks));
        }

        pub fn update_disk_heading(
            &self,
            disk_graph: &SummaryGraph,
            disk_id: &str,
            index: Option<i32>,
        ) {
            if index.is_some() {
                disk_graph.set_heading(i18n_f(
                    "Drive {} ({})",
                    &[&format!("{}", index.unwrap()), &format!("{}", disk_id)],
                ));
            } else {
                disk_graph.set_heading(i18n("Drive"));
            }
        }

        fn disk_page_name(disk_id: &str) -> String {
            format!("disk-{}", disk_id)
        }

        pub fn create_disk_page(
            &self,
            readings: &crate::sys_info_v2::Readings,
            disk_id: Option<i32>,
            pos_hint: Option<i32>,
        ) -> (String, (SummaryGraph, DiskPage)) {
            let disk_static_info = &readings.disks_info[disk_id.unwrap_or(0) as usize];

            let page_name = Self::disk_page_name(disk_static_info.id.as_ref());

            let summary = SummaryGraph::new();
            summary.set_widget_name(&page_name);

            self.update_disk_heading(&summary, disk_static_info.id.as_ref(), disk_id);
            summary.set_info1(match disk_static_info.r#type {
                DiskType::HDD => i18n("HDD"),
                DiskType::SSD => i18n("SSD"),
                DiskType::NVMe => i18n("NVMe"),
                DiskType::eMMC => i18n("eMMC"),
                DiskType::SD => i18n("SD"),
                DiskType::iSCSI => i18n("iSCSI"),
                DiskType::Optical => i18n("Optical"),
                DiskType::Unknown => i18n("Unknown"),
            });
            summary.set_info2(format!("{:.0}%", disk_static_info.busy_percent));
            summary.set_base_color(gdk::RGBA::new(
                DISK_BASE_COLOR[0] as f32 / 255.,
                DISK_BASE_COLOR[1] as f32 / 255.,
                DISK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = settings!();

            summary
                .graph_widget()
                .set_data_points(settings.int("performance-page-data-points") as u32);

            summary
                .graph_widget()
                .set_smooth_graphs(settings.boolean("performance-smooth-graphs"));

            let page = DiskPage::new(&page_name, &settings);
            page.set_base_color(gdk::RGBA::new(
                DISK_BASE_COLOR[0] as f32 / 255.,
                DISK_BASE_COLOR[1] as f32 / 255.,
                DISK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            page.set_static_information(disk_id, disk_static_info);

            self.configure_page(&page);

            self.page_stack.add_named(&page, Some(&page_name));
            self.add_to_sidebar(&summary, pos_hint);

            let mut actions = self.context_menu_view_actions.take();
            match actions.get("disk") {
                None => {
                    g_critical!(
                        "MissionCenter::PerformancePage",
                        "Failed to wire up disk action for {}, logic bug?",
                        &disk_static_info.id
                    );
                }
                Some(action) => {
                    actions.insert(page_name.clone(), action.clone());
                }
            }
            self.context_menu_view_actions.set(actions);

            (page_name, (summary, page))
        }

        fn set_up_network_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let mut networks = HashMap::new();
            for network_device in &readings.network_devices {
                let mut ret = self.create_network_page(network_device, None);
                networks.insert(std::mem::take(&mut ret.0), ret.1);
            }

            pages.push(Pages::Network(networks));
        }

        fn network_page_name(if_name: &str) -> String {
            format!("net-{}", if_name)
        }

        fn create_network_page(
            &self,
            network_device: &NetworkDevice,
            pos_hint: Option<i32>,
        ) -> (String, (SummaryGraph, NetworkPage)) {
            let if_name = network_device.descriptor.if_name.as_str();
            let page_name = Self::network_page_name(if_name);

            let conn_type = network_device.descriptor.kind.to_string();
            let summary = SummaryGraph::new();
            summary.set_widget_name(&page_name);
            summary.set_heading(format!("{} ({})", conn_type.clone(), if_name.to_string()));
            {
                let graph_widget = summary.graph_widget();
                graph_widget.set_data_set_count(2);
                graph_widget.set_scaling(GraphWidget::auto_pow2_scaling());
                graph_widget.set_filled(0, false);
                graph_widget.set_dashed(0, true);
                graph_widget.set_base_color(gdk::RGBA::new(
                    NETWORK_BASE_COLOR[0] as f32 / 255.,
                    NETWORK_BASE_COLOR[1] as f32 / 255.,
                    NETWORK_BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
            }

            let settings = settings!();

            summary
                .graph_widget()
                .set_data_points(settings.int("performance-page-data-points") as u32);
            summary
                .graph_widget()
                .set_smooth_graphs(settings.boolean("performance-smooth-graphs"));

            if network_device.max_speed > 0 {
                if !settings.boolean("performance-page-network-dynamic-scaling") {
                    summary
                        .graph_widget()
                        .set_scaling(GraphWidget::no_scaling());
                    summary
                        .graph_widget()
                        .set_value_range_max((network_device.max_speed * 8) as f32);
                }
                let max_speed = network_device.max_speed * 8;
                settings.connect_changed(Some("performance-page-network-dynamic-scaling"), {
                    let graph = summary.graph_widget().downgrade();
                    move |settings, _| {
                        let graph = match graph.upgrade() {
                            Some(graph) => graph,
                            None => return,
                        };

                        let dynamic_scaling =
                            settings.boolean("performance-page-network-dynamic-scaling");

                        if dynamic_scaling {
                            graph.set_scaling(GraphWidget::auto_pow2_scaling());
                        } else {
                            graph.set_scaling(GraphWidget::no_scaling());
                        }
                        graph.set_value_range_max(max_speed as f32);
                    }
                });
            }

            let page = NetworkPage::new(if_name, network_device.descriptor.kind, &settings);
            page.set_base_color(gdk::RGBA::new(
                NETWORK_BASE_COLOR[0] as f32 / 255.,
                NETWORK_BASE_COLOR[1] as f32 / 255.,
                NETWORK_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            page.set_static_information(network_device);
            self.configure_page(&page);

            self.page_stack.add_named(&page, Some(&page_name));
            self.add_to_sidebar(&summary, pos_hint);

            let mut actions = self.context_menu_view_actions.take();
            match actions.get("network") {
                None => {
                    g_critical!(
                        "MissionCenter::PerformancePage",
                        "Failed to wire up network action for {}, logic bug?",
                        if_name
                    );
                }

                Some(action) => {
                    actions.insert(page_name.clone(), action.clone());
                }
            }
            self.context_menu_view_actions.set(actions);

            (page_name, (summary, page))
        }

        fn gpu_page_name(device_id: &str) -> String {
            format!("gpu-{}", device_id)
        }

        fn set_up_gpu_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let mut gpus = HashMap::new();

            let hide_index = readings.gpu_static_info.len() == 1;
            for (index, static_info) in readings.gpu_static_info.iter().enumerate() {
                let dynamic_info = &readings.gpu_dynamic_info[index];

                let page_name = Self::gpu_page_name(static_info.id.as_ref());

                let summary = SummaryGraph::new();
                summary.set_widget_name(&page_name);

                let settings = settings!();

                summary
                    .graph_widget()
                    .set_data_points(settings.int("performance-page-data-points") as u32);

                summary
                    .graph_widget()
                    .set_smooth_graphs(settings.boolean("performance-smooth-graphs"));

                let page = GpuPage::new(&static_info.device_name, &settings);

                if !hide_index {
                    summary.set_heading(i18n_f("GPU {}", &[&format!("{}", index)]));
                } else {
                    summary.set_heading(i18n_f("GPU", &[]));
                }
                summary.set_info1(static_info.device_name.as_ref());
                summary.set_info2(format!(
                    "{}% ({} °C)",
                    dynamic_info.util_percent, dynamic_info.temp_celsius
                ));
                summary.set_base_color(gdk::RGBA::new(
                    GPU_BASE_COLOR[0] as f32 / 255.,
                    GPU_BASE_COLOR[1] as f32 / 255.,
                    GPU_BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));

                page.set_base_color(gdk::RGBA::new(
                    GPU_BASE_COLOR[0] as f32 / 255.,
                    GPU_BASE_COLOR[1] as f32 / 255.,
                    GPU_BASE_COLOR[2] as f32 / 255.,
                    1.,
                ));
                page.set_static_information(
                    if !hide_index { Some(index) } else { None },
                    static_info,
                );

                self.configure_page(&page);

                self.page_stack.add_named(&page, Some(&page_name));
                self.add_to_sidebar(&summary, None);

                let mut actions = self.context_menu_view_actions.take();
                match actions.get("gpu") {
                    None => {
                        g_critical!(
                            "MissionCenter::PerformancePage",
                            "Failed to wire up GPU action for {}, logic bug?",
                            &static_info.device_name
                        );
                    }
                    Some(action) => {
                        actions.insert(page_name.clone(), action.clone());
                    }
                }
                self.context_menu_view_actions.set(actions);

                gpus.insert(page_name, (summary, page));
            }

            pages.push(Pages::Gpu(gpus));
        }

        fn set_up_fan_pages(
            &self,
            pages: &mut Vec<Pages>,
            readings: &crate::sys_info_v2::Readings,
        ) {
            let mut fans = HashMap::new();
            let len = readings.fans_info.len();
            let hide_index = len == 1;
            for i in 0..len {
                let mut ret = self.create_fan_page(
                    readings,
                    if hide_index { None } else { Some(i as i32) },
                    None,
                );
                fans.insert(std::mem::take(&mut ret.0), ret.1);
            }

            pages.push(Pages::Fan(fans));
        }

        fn fan_page_name(fan_label: &str) -> String {
            format!("fan-{}", fan_label)
        }

        pub fn create_fan_page(
            &self,
            readings: &crate::sys_info_v2::Readings,
            fan_id: Option<i32>,
            pos_hint: Option<i32>,
        ) -> (String, (SummaryGraph, FanPage)) {
            let fan_static_info =
                &readings.fans_info[fan_id.map(|i| i as usize).clone().unwrap_or(0)];

            let page_name = Self::fan_page_name(fan_static_info.fan_label.as_ref());

            let summary = SummaryGraph::new();
            summary.set_widget_name(&page_name);

            if fan_id.is_some() {
                summary.set_heading(i18n_f("Fan {}", &[&format!("{}", fan_id.unwrap())]));
            } else {
                summary.set_heading(i18n("Fan"));
            }
            summary.set_base_color(gdk::RGBA::new(
                FAN_BASE_COLOR[0] as f32 / 255.,
                FAN_BASE_COLOR[1] as f32 / 255.,
                FAN_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));

            let settings = settings!();

            summary
                .graph_widget()
                .set_scaling(GraphWidget::normalized_scaling());
            summary.graph_widget().set_only_scale_up(true);
            summary
                .graph_widget()
                .set_data_points(settings.int("performance-page-data-points") as u32);
            summary
                .graph_widget()
                .set_smooth_graphs(settings.boolean("performance-smooth-graphs"));

            let page = FanPage::new(&page_name, &settings);
            page.set_base_color(gdk::RGBA::new(
                FAN_BASE_COLOR[0] as f32 / 255.,
                FAN_BASE_COLOR[1] as f32 / 255.,
                FAN_BASE_COLOR[2] as f32 / 255.,
                1.,
            ));
            page.set_static_information(fan_static_info);

            self.configure_page(&page);

            self.page_stack.add_named(&page, Some(&page_name));
            self.add_to_sidebar(&summary, pos_hint);

            let mut actions = self.context_menu_view_actions.take();
            match actions.get("fan") {
                None => {
                    g_critical!(
                        "MissionCenter::PerformancePage",
                        "Failed to wire up fan action for {}, logic bug?",
                        &fan_static_info.fan_label
                    );
                }
                Some(action) => {
                    actions.insert(page_name.clone(), action.clone());
                }
            }
            self.context_menu_view_actions.set(actions);

            (page_name, (summary, page))
        }

        pub fn default_sort_sidebar_entries(&self) {
            fn add_graph_to_sidebar(
                graph: Option<(SummaryGraph, gtk::DragSource)>,
                sidebar: &gtk::ListBox,
                index: &mut i32,
            ) {
                if let Some((graph, drag_controller)) = graph {
                    sidebar.insert(&graph, *index);
                    sidebar
                        .row_at_index(*index)
                        .and_then(|row| Some(row.add_controller(drag_controller)));
                    *index += 1;
                }
            }

            fn add_graphs_to_sidebar(
                mut graphs: Vec<(SummaryGraph, gtk::DragSource)>,
                sidebar: &gtk::ListBox,
                index: &mut i32,
            ) {
                for (graph, drag_controller) in graphs.drain(..) {
                    sidebar.insert(&graph, *index);
                    sidebar
                        .row_at_index(*index)
                        .and_then(|row| Some(row.add_controller(drag_controller)));
                    *index += 1;
                }
            }

            let summary_graphs = self.summary_graphs.take();

            let mut cpu_graph = None;
            let mut memory_graph = None;
            let mut disk_graphs = Vec::with_capacity(summary_graphs.len());
            let mut net_graphs = Vec::with_capacity(summary_graphs.len());
            let mut gpu_graphs = Vec::with_capacity(summary_graphs.len());
            let mut fan_graphs = Vec::with_capacity(summary_graphs.len());

            for (graph, drag_source) in &summary_graphs {
                graph.set_is_enabled(true);

                if graph.widget_name().starts_with("cpu") {
                    cpu_graph = Some((graph.clone(), drag_source.clone()));
                } else if graph.widget_name().starts_with("memory") {
                    memory_graph = Some((graph.clone(), drag_source.clone()));
                } else if graph.widget_name().starts_with("disk") {
                    disk_graphs.push((graph.clone(), drag_source.clone()));
                } else if graph.widget_name().starts_with("net") {
                    net_graphs.push((graph.clone(), drag_source.clone()));
                } else if graph.widget_name().starts_with("gpu") {
                    gpu_graphs.push((graph.clone(), drag_source.clone()));
                } else if graph.widget_name().starts_with("fan") {
                    fan_graphs.push((graph.clone(), drag_source.clone()));
                }
            }

            self.summary_graphs.set(summary_graphs);

            disk_graphs
                .sort_unstable_by(|(g1, _), (g2, _)| g1.widget_name().cmp(&g2.widget_name()));
            net_graphs.sort_unstable_by(|(g1, _), (g2, _)| g1.widget_name().cmp(&g2.widget_name()));
            gpu_graphs.sort_unstable_by(|(g1, _), (g2, _)| g1.widget_name().cmp(&g2.widget_name()));
            fan_graphs.sort_unstable_by(|(g1, _), (g2, _)| g1.widget_name().cmp(&g2.widget_name()));

            let sidebar = self.sidebar();
            sidebar.remove_all();

            let mut index = 0;
            add_graph_to_sidebar(cpu_graph, &sidebar, &mut index);
            add_graph_to_sidebar(memory_graph, &sidebar, &mut index);
            add_graphs_to_sidebar(disk_graphs, &sidebar, &mut index);
            add_graphs_to_sidebar(net_graphs, &sidebar, &mut index);
            add_graphs_to_sidebar(gpu_graphs, &sidebar, &mut index);
            add_graphs_to_sidebar(fan_graphs, &sidebar, &mut index);
        }
    }

    impl PerformancePage {
        pub fn set_up_pages(
            this: &super::PerformancePage,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            let this = this.imp();

            let mut pages = vec![];
            this.set_up_cpu_page(&mut pages, &readings);
            this.set_up_memory_page(&mut pages, &readings);
            this.set_up_disk_pages(&mut pages, &readings);
            this.set_up_network_pages(&mut pages, &readings);
            this.set_up_gpu_pages(&mut pages, &readings);
            this.set_up_fan_pages(&mut pages, &readings);
            this.pages.set(pages);

            this.default_sort_sidebar_entries();

            let settings = settings!();

            let view_actions = this.context_menu_view_actions.take();
            let action = if let Some(action) =
                view_actions.get(settings.string("performance-selected-page").as_str())
            {
                action
            } else {
                view_actions.get("cpu").expect("All computers have a CPU")
            };
            action.activate(None);

            this.context_menu_view_actions.set(view_actions);

            let sidebar = this.sidebar();

            let hidden_graphs = settings.string("performance-sidebar-hidden-graphs");
            let hidden_graphs = hidden_graphs
                .split(";")
                .filter(|g| !g.is_empty())
                .collect::<HashSet<_>>();

            let sidebar_order = settings.string("performance-sidebar-order");

            let mut row_map = HashMap::new();
            let mut row_index = -1;
            loop {
                row_index += 1;
                let row = match sidebar.row_at_index(row_index) {
                    Some(row) => row,
                    None => break,
                };

                let graph = match row
                    .child()
                    .and_then(|child| child.downcast_ref::<SummaryGraph>().cloned())
                {
                    Some(graph) => graph,
                    None => continue,
                };

                let name = graph.widget_name();

                if hidden_graphs.contains(name.as_str()) {
                    graph.set_is_enabled(false);
                    row.set_visible(false);
                }

                row_map.insert(graph.widget_name(), (row, graph));
            }

            let summary_graphs = this.summary_graphs.take();

            for (i, row_name) in sidebar_order
                .split(';')
                .filter(|g| !g.is_empty())
                .enumerate()
                .map(|(i, r)| (i as i32, r))
            {
                if let Some((row, graph)) = row_map.remove(row_name) {
                    let drag_controller = match summary_graphs.get(&graph) {
                        Some(drag_controller) => drag_controller.clone(),
                        None => {
                            g_critical!(
                                "MissionCenter::PerformancePage",
                                "Drag controller is missing from summary graphs for {}",
                                row_name
                            );
                            continue;
                        }
                    };

                    sidebar.remove(&row);
                    drop(row);

                    sidebar.insert(&graph, i);
                    sidebar.row_at_index(i).and_then(|row| {
                        if !graph.is_enabled() {
                            row.set_visible(false);
                        }
                        Some(row.add_controller(drag_controller))
                    });
                }
            }

            this.summary_graphs.set(summary_graphs);

            true
        }

        pub fn update_readings(
            this: &super::PerformancePage,
            readings: &crate::sys_info_v2::Readings,
        ) -> bool {
            use glib::g_warning;

            let mut pages = this.imp().pages.take();

            let mut pages_to_destroy = Vec::new();

            fn remove_pages<P: IsA<gtk::Widget>>(
                pages_to_destroy: &Vec<String>,
                pages: &mut HashMap<String, (SummaryGraph, P)>,
                summary_graphs: &mut HashMap<SummaryGraph, gtk::DragSource>,
                sidebar: &gtk::ListBox,
                page_stack: &gtk::Stack,
            ) {
                for disk_page_name in pages_to_destroy {
                    if let Some((graph, page)) =
                        pages.get(disk_page_name).and_then(|v| Some(v.clone()))
                    {
                        let parent = match graph.parent() {
                            Some(parent) => parent,
                            None => {
                                g_warning!(
                                    "MissionCenter::PerformancePage",
                                    "Failed to get parent of graph widget"
                                );
                                continue;
                            }
                        };
                        summary_graphs.remove(&graph);
                        sidebar.remove(&parent);
                        page_stack.remove(&page);
                        pages.remove(disk_page_name);
                    }
                }
            }

            for page in &mut pages {
                match page {
                    Pages::Cpu(_) => {}    // not dynamic
                    Pages::Memory(_) => {} // not dynamic
                    Pages::Disk(ref mut disks_pages) => {
                        for disk_page_name in disks_pages.keys() {
                            if !readings.disks_info.iter().any(|device| {
                                &Self::disk_page_name(device.id.as_ref()) == disk_page_name
                            }) {
                                pages_to_destroy.push(disk_page_name.clone());
                            }
                        }

                        let mut summary_graphs = this.imp().summary_graphs.take();

                        remove_pages(
                            &pages_to_destroy,
                            disks_pages,
                            &mut summary_graphs,
                            &this.sidebar(),
                            &this.imp().page_stack,
                        );
                        pages_to_destroy.clear();

                        this.imp().summary_graphs.set(summary_graphs);
                    }
                    Pages::Network(net_pages) => {
                        for net_page_name in net_pages.keys() {
                            if !readings.network_devices.iter().any(|device| {
                                &Self::network_page_name(&device.descriptor.if_name)
                                    == net_page_name
                            }) {
                                pages_to_destroy.push(net_page_name.clone());
                            }
                        }

                        let mut summary_graphs = this.imp().summary_graphs.take();

                        remove_pages(
                            &pages_to_destroy,
                            net_pages,
                            &mut summary_graphs,
                            &this.sidebar(),
                            &this.imp().page_stack,
                        );
                        pages_to_destroy.clear();

                        this.imp().summary_graphs.set(summary_graphs);
                    }
                    Pages::Gpu(_) => {}
                    Pages::Fan(_) => {}
                }
            }

            let mut result = true;

            let settings = settings!();

            let data_points = settings.int("performance-page-data-points") as u32;
            let smooth = settings.boolean("performance-smooth-graphs");

            for page in &mut pages {
                match page {
                    Pages::Cpu((summary, page)) => {
                        let graph_widget = summary.graph_widget();
                        graph_widget.set_data_points(data_points);
                        graph_widget.set_smooth_graphs(smooth);

                        graph_widget.add_data_point(
                            0,
                            readings.cpu_dynamic_info.overall_utilization_percent,
                        );
                        summary.set_info1(format!(
                            "{}% {:.2} GHz",
                            readings
                                .cpu_dynamic_info
                                .overall_utilization_percent
                                .round(),
                            readings.cpu_dynamic_info.current_frequency_mhz as f32 / 1024.
                        ));
                        match readings.cpu_dynamic_info.temperature.as_ref() {
                            Some(v) => summary.set_info2(format!("{:.0} °C", *v)),
                            _ => {}
                        }

                        result &= page.update_readings(readings);
                    }
                    Pages::Memory((summary, page)) => {
                        let total_raw = readings.mem_info.mem_total;
                        let total = crate::to_human_readable(total_raw as _, 1024.);
                        let used_raw = readings.mem_info.mem_total
                            - (readings.mem_info.mem_available + readings.mem_info.dirty);
                        let graph_widget = summary.graph_widget();
                        graph_widget.set_data_points(data_points);
                        graph_widget.set_smooth_graphs(smooth);
                        graph_widget.add_data_point(0, readings.mem_info.committed as _);
                        graph_widget.add_data_point(1, used_raw as _);
                        let used = crate::to_human_readable(used_raw as _, 1024.);

                        summary.set_info1(format!(
                            "{}%",
                            ((used_raw as f32 / total_raw as f32) * 100.).round()
                        ));

                        summary.set_info2(format!(
                            "{0:.2$} {1}iB/{3:.5$} {4}iB",
                            used.0, used.1, used.2, total.0, total.1, total.2
                        ));

                        result &= page.update_readings(readings);
                    }
                    Pages::Disk(pages) => {
                        let mut last_sidebar_pos = -1;
                        let mut consecutive_dev_count = 0;

                        let mut new_devices = Vec::new();
                        let hide_index = readings.disks_info.len() == 1;
                        for (index, disk) in readings.disks_info.iter().enumerate() {
                            if let Some((summary, page)) =
                                pages.get(&Self::disk_page_name(disk.id.as_ref()))
                            {
                                this.imp().update_disk_heading(
                                    summary,
                                    disk.id.as_ref(),
                                    if hide_index { None } else { Some(index as i32) },
                                );

                                // Search for a group of existing disks and try to add new entries at that position
                                summary
                                    .parent()
                                    .and_then(|p| p.downcast_ref::<gtk::ListBoxRow>().cloned())
                                    .and_then(|row| {
                                        let sidebar_pos = row.index();
                                        if sidebar_pos == last_sidebar_pos + 1 {
                                            consecutive_dev_count += 1;
                                        } else {
                                            consecutive_dev_count = 1;
                                        };
                                        last_sidebar_pos = sidebar_pos;

                                        Some(())
                                    });

                                let graph_widget = summary.graph_widget();
                                graph_widget.set_data_points(data_points);
                                graph_widget.set_smooth_graphs(smooth);
                                graph_widget.add_data_point(0, disk.busy_percent);
                                summary.set_info2(format!("{:.0}%", disk.busy_percent));

                                result &= page.update_readings(
                                    if hide_index { None } else { Some(index) },
                                    disk,
                                );
                            } else {
                                new_devices.push(index);
                            }
                        }

                        for new_device_index in new_devices {
                            let (disk_id, page) = this.imp().create_disk_page(
                                readings,
                                if hide_index {
                                    None
                                } else {
                                    Some(new_device_index as i32)
                                },
                                if last_sidebar_pos > -1 && consecutive_dev_count > 1 {
                                    last_sidebar_pos += 1;
                                    Some(last_sidebar_pos)
                                } else {
                                    None
                                },
                            );
                            pages.insert(disk_id, page);
                        }
                    }
                    Pages::Network(pages) => {
                        let mut last_sidebar_pos = -1;
                        let mut consecutive_dev_count = 0;

                        let mut new_devices = Vec::new();
                        for (index, network_device) in readings.network_devices.iter().enumerate() {
                            if let Some((summary, page)) = pages
                                .get(&Self::network_page_name(&network_device.descriptor.if_name))
                            {
                                let data_per_time = page.unit_per_second_label();
                                let byte_coeff = page.byte_conversion_factor();

                                let send_speed = network_device.send_bps * byte_coeff;
                                let rec_speed = network_device.recv_bps * byte_coeff;

                                // Search for a group of existing network devices and try to add new entries at that position
                                summary
                                    .parent()
                                    .and_then(|p| p.downcast_ref::<gtk::ListBoxRow>().cloned())
                                    .and_then(|row| {
                                        let sidebar_pos = row.index();
                                        if sidebar_pos == last_sidebar_pos + 1 {
                                            consecutive_dev_count += 1;
                                        } else {
                                            consecutive_dev_count = 1;
                                        };
                                        last_sidebar_pos = sidebar_pos;

                                        Some(())
                                    });

                                let graph_widget = summary.graph_widget();
                                graph_widget.set_data_points(data_points);
                                graph_widget.set_smooth_graphs(smooth);
                                let sent_speed = crate::to_human_readable(send_speed, 1024.);
                                let rect_speeed = crate::to_human_readable(rec_speed, 1024.);

                                summary.set_info1(i18n_f(
                                    "{}: {} {}{}",
                                    &[
                                        "S",
                                        &format!("{0:.1$}", sent_speed.0, sent_speed.2),
                                        &format!("{}", sent_speed.1),
                                        &*data_per_time,
                                    ],
                                ));
                                summary.set_info2(i18n_f(
                                    "{}: {} {}{}",
                                    &[
                                        "R",
                                        &format!("{0:.1$}", rect_speeed.0, rect_speeed.2),
                                        &format!("{}", rect_speeed.1),
                                        &*data_per_time,
                                    ],
                                ));

                                result &= page.update_readings(network_device);
                            } else {
                                new_devices.push(index);
                            }
                        }

                        for new_device_index in new_devices {
                            let (net_if_id, page) = this.imp().create_network_page(
                                &readings.network_devices[new_device_index],
                                if last_sidebar_pos > -1 && consecutive_dev_count > 1 {
                                    last_sidebar_pos += 1;
                                    Some(last_sidebar_pos)
                                } else {
                                    None
                                },
                            );
                            pages.insert(net_if_id, page);
                        }
                    }
                    Pages::Gpu(pages) => {
                        for gpu in &readings.gpu_dynamic_info {
                            if let Some((summary, page)) =
                                pages.get(&Self::gpu_page_name(gpu.id.as_ref()))
                            {
                                let graph_widget = summary.graph_widget();
                                graph_widget.set_data_points(data_points);
                                graph_widget.set_smooth_graphs(smooth);
                                graph_widget.add_data_point(0, gpu.util_percent as f32);
                                if gpu.temp_celsius > 20 {
                                    summary.set_info2(format!(
                                        "{}% ({} °C)",
                                        gpu.util_percent, gpu.temp_celsius
                                    ));
                                } else {
                                    summary.set_info2(format!("{}%", gpu.util_percent));
                                }
                                let id = gpu.id.clone();
                                let mut gpu_static = None;
                                for gpu_stat in &readings.gpu_static_info {
                                    if id == gpu_stat.id {
                                        gpu_static = Some(gpu_stat);
                                        break;
                                    }
                                }

                                result &= page.update_readings(gpu, gpu_static.unwrap());
                            }
                        }
                    }
                    Pages::Fan(pages) => {
                        for fan_info in &readings.fans_info {
                            if let Some((summary, page)) =
                                pages.get(&Self::fan_page_name(fan_info.fan_label.as_ref()))
                            {
                                let graph_widget = summary.graph_widget();
                                graph_widget.set_data_points(data_points);
                                graph_widget.set_smooth_graphs(smooth);
                                graph_widget.add_data_point(0, fan_info.rpm as f32);
                                summary.set_info1(format!("{} RPM", fan_info.rpm));
                                if fan_info.temp_amount != i64::MIN {
                                    summary.set_info2(format!(
                                        "{:.0} °C",
                                        fan_info.temp_amount as f32 / 1000.0
                                    ));
                                } else {
                                    summary.set_info2("");
                                }
                                result &= page.update_readings(fan_info);
                            }
                        }
                    }
                }
            }

            this.imp().pages.set(pages);

            result
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PerformancePage {
        const NAME: &'static str = "PerformancePage";
        type Type = super::PerformancePage;
        type ParentType = adw::BreakpointBin;

        fn class_init(klass: &mut Self::Class) {
            SummaryGraph::ensure_type();
            GraphWidget::ensure_type();
            CpuPage::ensure_type();
            NetworkPage::ensure_type();
            SidebarDropHint::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PerformancePage {
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

            let this = self.obj().clone();

            this.insert_action_group("graph", Some(unsafe { &*self.action_group.as_ptr() }));

            self.breakpoint.set_condition(Some(
                &adw::BreakpointCondition::parse("max-width: 570sp").unwrap(),
            ));
            self.breakpoint.connect_apply({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.breakpoint_applied.set(true);
                    this.page_content.set_collapsed(true);
                    this.page_content.set_show_sidebar(false);
                }
            });
            self.breakpoint.connect_unapply({
                let this = self.obj().downgrade();
                move |_| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    this.breakpoint_applied.set(false);
                    this.page_content.set_collapsed(false);
                    if !this.summary_mode.get() {
                        this.page_content.set_show_sidebar(true);
                    } else {
                        this.page_content.set_show_sidebar(false);
                    }
                }
            });

            self.page_content
                .sidebar()
                .expect("Infobar is not set")
                .parent()
                .and_then(|p| Some(p.remove_css_class("sidebar-pane")));
            self.page_content.connect_collapsed_notify({
                let this = self.obj().downgrade();
                move |pc| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };
                    let this = this.imp();

                    if !pc.is_collapsed() {
                        this.page_content
                            .sidebar()
                            .expect("Infobar is not set")
                            .parent()
                            .and_then(|p| Some(p.remove_css_class("sidebar-pane")));

                        this.info_bar.set_halign(gtk::Align::Fill);
                    } else {
                        this.info_bar.set_halign(gtk::Align::Center);
                    }
                    this.obj().notify_info_button_visible();
                }
            });

            self.page_content.connect_show_sidebar_notify({
                let this = self.obj().downgrade();
                move |_| {
                    if let Some(this) = this.upgrade() {
                        this.notify_infobar_visible();
                    }
                }
            });

            if let Some(child) = self.page_stack.visible_child() {
                let infobar_content = child.property::<Option<gtk::Widget>>("infobar-content");
                self.info_bar.set_child(infobar_content.as_ref());
            }
            self.page_stack.connect_visible_child_notify({
                let this = self.obj().downgrade();
                move |page_stack| {
                    let this = match this.upgrade() {
                        Some(this) => this,
                        None => return,
                    };

                    if let Some(child) = page_stack.visible_child() {
                        let infobar_content =
                            child.property::<Option<gtk::Widget>>("infobar-content");
                        this.imp().info_bar.set_child(infobar_content.as_ref());
                    }
                }
            });
        }
    }

    impl WidgetImpl for PerformancePage {}

    impl BreakpointBinImpl for PerformancePage {}
}

glib::wrapper! {
    pub struct PerformancePage(ObjectSubclass<imp::PerformancePage>)
        @extends adw::BreakpointBin, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl PerformancePage {
    pub fn set_initial_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        let ok = imp::PerformancePage::set_up_pages(self, readings);
        imp::PerformancePage::update_readings(self, readings) && ok
    }

    pub fn update_readings(&self, readings: &crate::sys_info_v2::Readings) -> bool {
        imp::PerformancePage::update_readings(self, readings)
    }

    pub fn sidebar_enable_all(&self) {
        let this = self.imp();

        if !this.sidebar_edit_mode.get() {
            return;
        }

        let summary_graphs = this.summary_graphs.take();
        for (graph, _) in &summary_graphs {
            graph.set_is_enabled(true);
        }
        this.summary_graphs.set(summary_graphs);
    }

    pub fn sidebar_disable_all(&self) {
        let this = self.imp();

        if !this.sidebar_edit_mode.get() {
            return;
        }

        let summary_graphs = this.summary_graphs.take();
        for (graph, _) in &summary_graphs {
            graph.set_is_enabled(false);
        }
        this.summary_graphs.set(summary_graphs);
    }

    pub fn sidebar_reset_to_default(&self) {
        let this = self.imp();

        if !this.sidebar_edit_mode.get() {
            return;
        }

        let settings = settings!();

        settings
            .set_string("performance-sidebar-order", "")
            .unwrap_or_else(|_| {
                glib::g_warning!(
                    "MissionCenter::PerformancePage",
                    "Failed to set performance-selected-page setting"
                );
            });
        settings
            .set_string("performance-sidebar-hidden-graphs", "")
            .unwrap_or_else(|_| {
                glib::g_warning!(
                    "MissionCenter::PerformancePage",
                    "Failed to set performance-sidebar-hidden-graphs setting"
                );
            });

        this.default_sort_sidebar_entries();
    }
}
