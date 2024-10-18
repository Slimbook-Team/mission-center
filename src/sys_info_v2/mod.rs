/* sys_info_v2/mod.rs
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

use std::num::NonZeroU32;
use std::sync::atomic::AtomicU64;
use std::{
    collections::HashMap,
    sync::{
        atomic::{self, AtomicBool},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Duration,
};

use gtk::glib::{g_critical, g_debug, g_warning, idle_add_once};
use lazy_static::lazy_static;

use crate::{
    app,
    application::{BASE_INTERVAL, INTERVAL_STEP},
    sys_info_v2::proc_info::process_hierarchy,
};
use gatherer::Gatherer;
pub use gatherer::{
    App, CpuDynamicInfo, CpuStaticInfo, DiskInfo, DiskType, FanInfo, GpuDynamicInfo, GpuStaticInfo,
    OpenGLApi, Process, ProcessUsageStats, Service,
};

macro_rules! cmd {
    ($cmd: expr) => {{
        use std::process::Command;

        if *crate::sys_info_v2::IS_FLATPAK {
            const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

            let mut cmd = Command::new(FLATPAK_SPAWN_CMD);
            cmd.arg("--host").arg("sh").arg("-c");
            cmd.arg($cmd);

            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.arg("-c");
            cmd.arg($cmd);

            cmd
        }
    }};
}

macro_rules! cmd_flatpak_host {
    ($cmd: expr) => {{
        use std::process::Command;

        const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

        let mut cmd = Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("--host").arg("sh").arg("-c");
        cmd.arg($cmd);

        cmd
    }};
}

mod dbus_interface;
mod gatherer;
mod mem_info;
mod net_info;
mod proc_info;

pub type MemInfo = mem_info::MemInfo;
#[allow(dead_code)]
pub type MemoryDevice = mem_info::MemoryDevice;

pub type NetInfo = net_info::NetInfo;
pub type NetworkDevice = net_info::NetworkDevice;
pub type NetDeviceType = net_info::NetDeviceType;

pub type Pid = u32;

lazy_static! {
    static ref IS_FLATPAK: bool = std::path::Path::new("/.flatpak-info").exists();
    static ref FLATPAK_APP_PATH: String = {
        use ini::*;

        let ini = match Ini::load_from_file("/.flatpak-info") {
            Err(_) => return "".to_owned(),
            Ok(ini) => ini,
        };

        let section = match ini.section(Some("Instance")) {
            None => panic!("Unable to find Instance section in /.flatpak-info"),
            Some(section) => section,
        };

        match section.get("app-path") {
            None => panic!("Unable to find 'app-path' key in Instance section in /.flatpak-info"),
            Some(app_path) => app_path.to_owned(),
        }
    };
}

enum Message {
    ContinueReading,
    UpdateRefreshInterval(u64),
    UpdateCoreCountAffectsPercentages(bool),
    TerminateProcess(Pid),
    KillProcess(Pid),
    StartService(Arc<str>),
    StopService(Arc<str>),
    RestartService(Arc<str>),
    EnableService(Arc<str>),
    DisableService(Arc<str>),
    GetServiceLogs(Arc<str>, Option<NonZeroU32>),
}

enum Response {
    String(Arc<str>),
}

#[derive(Debug)]
pub struct Readings {
    pub cpu_static_info: CpuStaticInfo,
    pub cpu_dynamic_info: CpuDynamicInfo,
    pub mem_info: MemInfo,
    pub disks_info: Vec<DiskInfo>,
    pub network_devices: Vec<NetworkDevice>,
    pub gpu_static_info: Vec<GpuStaticInfo>,
    pub gpu_dynamic_info: Vec<GpuDynamicInfo>,
    pub fans_info: Vec<FanInfo>,

    pub running_apps: HashMap<Arc<str>, App>,
    pub process_tree: Process,

    pub services: HashMap<Arc<str>, Service>,
}

impl Readings {
    pub fn new() -> Self {
        Self {
            cpu_static_info: Default::default(),
            cpu_dynamic_info: Default::default(),
            mem_info: MemInfo::default(),
            disks_info: vec![],
            network_devices: vec![],
            gpu_static_info: vec![],
            gpu_dynamic_info: vec![],
            fans_info: vec![],

            running_apps: HashMap::new(),
            process_tree: Process::default(),

            services: HashMap::new(),
        }
    }
}

pub struct SysInfoV2 {
    speed: Arc<AtomicU64>,

    refresh_thread: Option<std::thread::JoinHandle<()>>,
    refresh_thread_running: Arc<AtomicBool>,

    sender: Sender<Message>,
    receiver: Receiver<Response>,
}

impl Drop for SysInfoV2 {
    fn drop(&mut self) {
        self.refresh_thread_running
            .store(false, atomic::Ordering::Release);

        if let Some(refresh_thread) = std::mem::take(&mut self.refresh_thread) {
            refresh_thread
                .join()
                .expect("Unable to stop the refresh thread");
        }
    }
}

impl Default for SysInfoV2 {
    fn default() -> Self {
        let (tx, _) = mpsc::channel::<Message>();
        let (_, resp_rx) = mpsc::channel::<Response>();

        Self {
            speed: Arc::new(0.into()),

            refresh_thread: None,
            refresh_thread_running: Arc::new(true.into()),

            sender: tx,
            receiver: resp_rx,
        }
    }
}

impl SysInfoV2 {
    pub fn new() -> Self {
        let speed = Arc::new(AtomicU64::new(
            (BASE_INTERVAL / INTERVAL_STEP).round() as u64
        ));
        let refresh_thread_running = Arc::new(AtomicBool::new(true));

        let s = speed.clone();
        let run = refresh_thread_running.clone();

        let (tx, rx) = mpsc::channel::<Message>();
        let (resp_tx, resp_rx) = mpsc::channel::<Response>();
        Self {
            speed,
            refresh_thread: Some(std::thread::spawn(move || {
                Self::gather_and_proxy(rx, resp_tx, run, s);
            })),
            refresh_thread_running,
            sender: tx,
            receiver: resp_rx,
        }
    }

    pub fn set_update_speed(&self, speed: u64) {
        self.speed.store(speed, atomic::Ordering::Release);

        let refresh_interval = ((speed as f64 * INTERVAL_STEP) * 1000.) as u64;
        match self
            .sender
            .send(Message::UpdateRefreshInterval(refresh_interval))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending UpdateRefreshInterval to Gatherer: {e}"
                );
            }
            _ => {}
        }
    }

    pub fn set_core_count_affects_percentages(&self, show: bool) {
        match self
            .sender
            .send(Message::UpdateCoreCountAffectsPercentages(show))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending UpdateCoreCountAffectsPercentages to Gatherer: {e}"
                );
            }
            _ => {}
        }
    }

    pub fn continue_reading(&self) {
        match self.sender.send(Message::ContinueReading) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending ContinueReading to gatherer: {}",
                    e
                );
            }
            _ => {}
        }
    }

    pub fn terminate_process(&self, pid: u32) {
        match self.sender.send(Message::TerminateProcess(pid)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending TerminateProcess({}) to gatherer: {}",
                    pid,
                    e
                );
            }
            _ => {}
        }
    }

    pub fn kill_process(&self, pid: u32) {
        match self.sender.send(Message::KillProcess(pid)) {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending KillProcess({}) to gatherer: {}",
                    pid,
                    e
                );
            }
            _ => {}
        }
    }

    pub fn start_service(&self, name: &str) {
        match self
            .sender
            .send(Message::StartService(Arc::<str>::from(name)))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending StartService({}) to gatherer: {}",
                    name,
                    e
                );
            }
            _ => {}
        }
    }

    pub fn stop_service(&self, name: &str) {
        match self
            .sender
            .send(Message::StopService(Arc::<str>::from(name)))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending StopService({}) to gatherer: {}",
                    name,
                    e
                );
            }
            _ => {}
        }
    }

    pub fn restart_service(&self, name: &str) {
        match self
            .sender
            .send(Message::RestartService(Arc::<str>::from(name)))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending RestartService({}) to gatherer: {}",
                    name,
                    e
                );
            }
            _ => {}
        }
    }

    pub fn enable_service(&self, name: &str) {
        match self
            .sender
            .send(Message::EnableService(Arc::<str>::from(name)))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending EnableService({}) to gatherer: {}",
                    name,
                    e
                );
            }
            _ => {}
        }
    }

    pub fn disable_service(&self, name: &str) {
        match self
            .sender
            .send(Message::DisableService(Arc::<str>::from(name)))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending DisableService({}) to gatherer: {}",
                    name,
                    e
                );
            }
            _ => {}
        }
    }

    pub fn service_logs(&self, name: &str, pid: Option<NonZeroU32>) -> Arc<str> {
        match self
            .sender
            .send(Message::GetServiceLogs(Arc::<str>::from(name), pid))
        {
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error sending GetServiceLogs({}) to gatherer: {}",
                    name,
                    e
                );

                return Arc::from("");
            }
            _ => {}
        }

        match self.receiver.recv() {
            Ok(Response::String(logs)) => logs,
            Err(e) => {
                g_critical!(
                    "MissionCenter::SysInfo",
                    "Error receiving GetServiceLogs response: {}",
                    e
                );
                Arc::from("")
            }
        }
    }
}

impl SysInfoV2 {
    fn handle_incoming_message(
        gatherer: &Gatherer,
        rx: &mut Receiver<Message>,
        tx: &mut Sender<Response>,
        timeout: Duration,
    ) -> bool {
        match rx.recv_timeout(timeout) {
            Ok(message) => match message {
                Message::ContinueReading => {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "Received ContinueReading message while not reading"
                    );
                }
                Message::UpdateRefreshInterval(interval) => {
                    gatherer.set_refresh_interval(interval);
                }
                Message::UpdateCoreCountAffectsPercentages(show) => {
                    gatherer.set_core_count_affects_percentages(show);
                }
                Message::TerminateProcess(pid) => {
                    gatherer.terminate_process(pid);
                }
                Message::KillProcess(pid) => {
                    gatherer.kill_process(pid);
                }
                Message::StartService(name) => {
                    gatherer.start_service(&name);
                }
                Message::StopService(name) => {
                    gatherer.stop_service(&name);
                }
                Message::RestartService(name) => {
                    gatherer.restart_service(&name);
                }
                Message::EnableService(name) => {
                    gatherer.enable_service(&name);
                }
                Message::DisableService(name) => {
                    gatherer.disable_service(&name);
                }
                Message::GetServiceLogs(name, pid) => {
                    let resp = gatherer.get_service_logs(&name, pid);
                    if let Err(e) = tx.send(Response::String(resp)) {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Error sending GetServiceLogs response: {}",
                            e
                        );
                    }
                }
            },
            Err(_) => {}
        }

        true
    }

    fn gather_and_proxy(
        mut rx: Receiver<Message>,
        mut tx: Sender<Response>,
        running: Arc<AtomicBool>,
        speed: Arc<AtomicU64>,
    ) {
        let gatherer = Gatherer::new();
        gatherer.start();

        let mut net_info = NetInfo::new();

        let mut readings = Readings {
            cpu_static_info: gatherer.cpu_static_info(),
            cpu_dynamic_info: gatherer.cpu_dynamic_info(),
            mem_info: MemInfo::load().unwrap_or_default(),
            disks_info: gatherer.disks_info(),
            fans_info: gatherer.fans_info(),
            network_devices: if let Some(net_info) = net_info.as_mut() {
                net_info.load_devices()
            } else {
                vec![]
            },
            gpu_static_info: gatherer.gpu_static_info(),
            gpu_dynamic_info: gatherer.gpu_dynamic_info(),
            process_tree: process_hierarchy(&gatherer.processes()).unwrap_or_default(),
            running_apps: gatherer.apps(),
            services: gatherer.services(),
        };

        let refresh_services = !readings.services.is_empty();
        if readings.services.is_empty() {
            g_warning!(
                "MissionCenter::SysInfo",
                "No services were found, not asking for them again to avoid spamming the logs"
            );
        }

        readings.disks_info.sort_unstable();
        readings
            .network_devices
            .sort_unstable_by(|n1, n2| n1.descriptor.if_name.cmp(&n2.descriptor.if_name));

        idle_add_once({
            let initial_readings = Readings {
                cpu_static_info: readings.cpu_static_info.clone(),
                cpu_dynamic_info: std::mem::take(&mut readings.cpu_dynamic_info),
                mem_info: readings.mem_info.clone(),
                disks_info: std::mem::take(&mut readings.disks_info),
                fans_info: std::mem::take(&mut readings.fans_info),
                network_devices: std::mem::take(&mut readings.network_devices),
                gpu_static_info: readings.gpu_static_info.clone(),
                gpu_dynamic_info: std::mem::take(&mut readings.gpu_dynamic_info),
                running_apps: std::mem::take(&mut readings.running_apps),
                process_tree: std::mem::take(&mut readings.process_tree),
                services: std::mem::take(&mut readings.services),
            };

            move || {
                app!().set_initial_readings(initial_readings);
            }
        });

        loop {
            match rx.recv() {
                Ok(message) => match message {
                    Message::ContinueReading => {
                        break;
                    }
                    Message::UpdateRefreshInterval(interval) => {
                        gatherer.set_refresh_interval(interval);
                    }
                    Message::UpdateCoreCountAffectsPercentages(show) => {
                        gatherer.set_core_count_affects_percentages(show);
                    }
                    _ => {}
                },
                Err(_) => {
                    g_warning!(
                        "MissionCenter::SysInfo",
                        "No more messages in the buffer and channel closed",
                    );
                    return;
                }
            }
        }

        'read_loop: while running.load(atomic::Ordering::Acquire) {
            let loop_start = std::time::Instant::now();

            let timer = std::time::Instant::now();
            readings.cpu_dynamic_info = gatherer.cpu_dynamic_info();
            g_debug!(
                "MissionCenter::Perf",
                "CPU dynamic info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.mem_info = MemInfo::load().unwrap_or_default();
            g_debug!(
                "MissionCenter::Perf",
                "Memory info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.disks_info = gatherer.disks_info();
            g_debug!(
                "MissionCenter::Perf",
                "Disks info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.network_devices = if let Some(net_info) = net_info.as_mut() {
                net_info.load_devices()
            } else {
                vec![]
            };
            g_debug!(
                "MissionCenter::Perf",
                "Network devices info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.gpu_dynamic_info = gatherer.gpu_dynamic_info();
            g_debug!(
                "MissionCenter::Perf",
                "GPU dynamic info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.fans_info = gatherer.fans_info();
            g_debug!(
                "MissionCenter::Perf",
                "Fans info load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.process_tree = process_hierarchy(&gatherer.processes()).unwrap_or_default();
            g_debug!(
                "MissionCenter::Perf",
                "Process hierarchy load took: {:?}",
                timer.elapsed()
            );

            let timer = std::time::Instant::now();
            readings.running_apps = gatherer.apps();
            g_debug!(
                "MissionCenter::Perf",
                "Running apps load took: {:?}",
                timer.elapsed(),
            );

            if refresh_services {
                let timer = std::time::Instant::now();
                readings.services = gatherer.services();
                g_debug!(
                    "MissionCenter::Perf",
                    "Services load took: {:?}",
                    timer.elapsed()
                );
            }

            readings.disks_info.sort_unstable();
            readings
                .network_devices
                .sort_unstable_by(|n1, n2| n1.descriptor.if_name.cmp(&n2.descriptor.if_name));

            if !running.load(atomic::Ordering::Acquire) {
                break 'read_loop;
            }

            idle_add_once({
                let mut new_readings = Readings {
                    cpu_static_info: readings.cpu_static_info.clone(),
                    cpu_dynamic_info: std::mem::take(&mut readings.cpu_dynamic_info),
                    mem_info: readings.mem_info.clone(),
                    disks_info: std::mem::take(&mut readings.disks_info),
                    fans_info: std::mem::take(&mut readings.fans_info),
                    network_devices: std::mem::take(&mut readings.network_devices),
                    gpu_static_info: readings.gpu_static_info.clone(),
                    gpu_dynamic_info: std::mem::take(&mut readings.gpu_dynamic_info),
                    running_apps: std::mem::take(&mut readings.running_apps),
                    process_tree: std::mem::take(&mut readings.process_tree),
                    services: std::mem::take(&mut readings.services),
                };

                move || {
                    let app = app!();
                    let now = std::time::Instant::now();
                    let timer = std::time::Instant::now();
                    if !app.refresh_readings(&mut new_readings) {
                        g_critical!(
                            "MissionCenter::SysInfo",
                            "Readings were not completely refreshed, stale readings will be displayed"
                        );
                    }
                    g_debug!(
                        "MissionCenter::Perf",
                        "UI refresh took: {:?}",
                        timer.elapsed()
                    );
                    g_debug!(
                        "MissionCenter::SysInfo",
                        "Refreshed readings in {:?}",
                        now.elapsed()
                    );
                }
            });

            let mut wait_time = Duration::from_millis(
                ((speed.load(atomic::Ordering::Relaxed) as f64 * INTERVAL_STEP) * 1000.) as u64,
            )
            .saturating_sub(loop_start.elapsed());

            const ITERATIONS_COUNT: u32 = 10;

            let wait_time_fraction = wait_time / ITERATIONS_COUNT;
            for _ in 0..ITERATIONS_COUNT {
                let wait_timer = std::time::Instant::now();

                if !Self::handle_incoming_message(&gatherer, &mut rx, &mut tx, wait_time_fraction) {
                    break 'read_loop;
                }

                if !running.load(atomic::Ordering::Acquire) {
                    break 'read_loop;
                }

                wait_time = wait_time.saturating_sub(wait_timer.elapsed());
                if wait_time.is_zero() {
                    break;
                }
            }

            if !Self::handle_incoming_message(&gatherer, &mut rx, &mut tx, wait_time) {
                break 'read_loop;
            }

            let elapsed_since_start = loop_start.elapsed();
            g_debug!(
                "MissionCenter::Perf",
                "Full read-publish cycle took {elapsed_since_start:?}",
            );
        }
    }
}
