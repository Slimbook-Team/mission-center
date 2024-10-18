/* sys_info_v2/gatherer.rs
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
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};

use dbus::blocking::{
    stdintf::{org_freedesktop_dbus::Peer, org_freedesktop_dbus::Properties},
    LocalConnection, Proxy,
};
use gtk::glib::g_critical;

pub use super::dbus_interface::*;
use super::{FLATPAK_APP_PATH, IS_FLATPAK};
use crate::show_error_dialog_and_exit;

macro_rules! dbus_call {
    ($self: ident, $method: tt, $dbus_method_name: literal $(,$args:ident)*) => {{
        use gtk::glib::g_critical;
        use super::dbus_interface::Gatherer;

        const RETRY_COUNT: i32 = 10;

        for i in 1..=RETRY_COUNT {
            match $self.proxy.$method($($args,)*) {
                Ok(reply) => {
                    return reply;
                }
                Err(e) => {
                    match $self.is_running() {
                        Ok(()) => {
                            if e.name() == Some("org.freedesktop.DBus.Error.NoReply") {
                                g_critical!(
                                    "MissionCenter::Gatherer",
                                    "DBus call '{}' timed out, on try {}",
                                    $dbus_method_name, i,
                                );

                                if i == RETRY_COUNT - 1 {
                                    g_critical!("MissionCenter::Gatherer", "Restarting Gatherer...");
                                    $self.stop();
                                    $self.start();
                                } else {
                                    std::thread::sleep(Duration::from_millis(100));
                                }
                            } else {
                                g_critical!(
                                    "MissionCenter::Gatherer",
                                    "DBus call '{}' failed on try {}: {}",
                                    $dbus_method_name, i, e,
                                );

                                std::thread::sleep(Duration::from_millis(100));
                            }
                        }
                        Err(exit_code) => {
                            g_critical!(
                                "MissionCenter::Gatherer",
                                "Child failed, on try {}, with exit code {}. Restarting Gatherer...",
                                i, exit_code,
                            );
                            $self.start();
                        }
                    }
                }
            }
        }

        show_error_dialog_and_exit(&format!("DBus call '{}' failed after {} retries 😟.\nThe app will now close.", $dbus_method_name, RETRY_COUNT));
    }};
}

pub(crate) struct Gatherer {
    #[allow(dead_code)]
    connection: Rc<LocalConnection>,
    proxy: Proxy<'static, Rc<LocalConnection>>,

    child: RefCell<Option<std::process::Child>>,
}

impl Drop for Gatherer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Gatherer {
    pub fn new() -> Self {
        let connection = Rc::new(LocalConnection::new_session().unwrap_or_else(|e| {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to connect to D-Bus: {}",
                e
            );
            show_error_dialog_and_exit("Failed to connect to D-Bus 😟.\nThe app will now close.");
        }));

        let proxy = Proxy::new(
            MC_GATHERER_INTERFACE_NAME,
            MC_GATHERER_OBJECT_PATH,
            Duration::from_millis(1000),
            connection.clone(),
        );

        Self {
            connection,
            proxy,
            child: RefCell::new(None),
        }
    }

    pub fn start(&self) {
        let mut command = if *IS_FLATPAK {
            const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

            let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
            cmd.env_remove("LD_PRELOAD");
            cmd.arg("-v")
                .arg("--watch-bus")
                .arg("--host")
                .arg(Self::executable());
            cmd
        } else {
            let mut cmd = std::process::Command::new(Self::executable());
            cmd.env_remove("LD_PRELOAD");

            if let Some(mut appdir) = std::env::var_os("APPDIR") {
                appdir.push("/runtime/default");
                cmd.current_dir(appdir);
            }

            cmd
        };
        command
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        self.child.borrow_mut().replace(match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Failed to spawn Gatherer process: {}",
                    &e
                );
                show_error_dialog_and_exit(&format!("Failed to spawn Gatherer process: {}", e));
            }
        });

        const START_WAIT_TIME_MS: u64 = 300;
        const RETRY_COUNT: i32 = 50;

        // Let the child process start up
        for i in 0..RETRY_COUNT {
            std::thread::sleep(Duration::from_millis(START_WAIT_TIME_MS / 2));
            match self.proxy.ping() {
                Ok(()) => return,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Call to Gatherer Ping method failed on try {}: {}",
                        i,
                        e,
                    );
                }
            }
            std::thread::sleep(Duration::from_millis(START_WAIT_TIME_MS / 2));
        }

        show_error_dialog_and_exit("Failed to spawn Gatherer process: Did not respond to Ping");
    }

    pub fn stop(&self) {
        let child = self.child.borrow_mut().take();
        if let Some(mut child) = child {
            // Try to get the child to wake up in case it's stuck
            #[cfg(target_family = "unix")]
            unsafe {
                libc::kill(child.id() as _, libc::SIGCONT);
            }

            let _ = child.kill();
            for _ in 0..2 {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => {
                        // Wait a bit and try again, the child process might just be slow to stop
                        std::thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::Gatherer",
                            "Failed to wait for Gatherer process to stop: {}",
                            &e
                        );

                        show_error_dialog_and_exit(&format!(
                            "Failed to wait for Gatherer process to stop: {}",
                            e
                        ));
                    }
                }
            }
        }
    }

    pub fn is_running(&self) -> Result<(), i32> {
        let mut lock = self.child.borrow_mut();

        let child = match lock.as_mut() {
            Some(child) => child,
            None => return Err(-1),
        };

        let status = match child.try_wait() {
            Ok(None) => return Ok(()),
            Ok(Some(status)) => status,
            Err(_) => {
                return Err(-1);
            }
        };

        match status.code() {
            Some(status_code) => Err(status_code),
            None => Err(-1),
        }
    }

    fn executable() -> String {
        use gtk::glib::g_debug;

        let exe_simple = "missioncenter-gatherer".to_owned();

        if *IS_FLATPAK {
            let flatpak_app_path = FLATPAK_APP_PATH.as_str();

            let cmd_glibc_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-gatherer-glibc just-testing",
                flatpak_app_path
            ))
            .status()
            .is_ok_and(|exit_status| exit_status.success());
            if cmd_glibc_status {
                let exe_glibc = format!("{}/bin/missioncenter-gatherer-glibc", flatpak_app_path);
                g_debug!(
                    "MissionCenter::Gatherer",
                    "Gatherer executable name: {}",
                    &exe_glibc
                );
                return exe_glibc;
            }

            let cmd_musl_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-gatherer-musl just-testing",
                flatpak_app_path
            ))
            .status()
            .is_ok_and(|exit_status| exit_status.success());
            if cmd_musl_status {
                let exe_musl = format!("{}/bin/missioncenter-gatherer-musl", flatpak_app_path);
                g_debug!(
                    "MissionCenter::Gatherer",
                    "Gatherer executable name: {}",
                    &exe_musl
                );
                return exe_musl;
            }
        }

        g_debug!(
            "MissionCenter::Gatherer",
            "Gatherer executable name: {}",
            &exe_simple
        );

        exe_simple
    }
}

impl Gatherer {
    pub fn set_refresh_interval(&self, interval: u64) {
        if let Err(e) = self
            .proxy
            .set(MC_GATHERER_INTERFACE_NAME, "RefreshInterval", interval)
        {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to set RefreshInterval property: {e}"
            );
        }
    }

    pub fn set_core_count_affects_percentages(&self, v: bool) {
        if let Err(e) = self
            .proxy
            .set(MC_GATHERER_INTERFACE_NAME, "CoreCountAffectsPercentages", v)
        {
            g_critical!(
                "MissionCenter::Gatherer",
                "Failed to set CoreCountAffectsPercentages property: {e}"
            );
        }
    }

    pub fn cpu_static_info(&self) -> CpuStaticInfo {
        dbus_call!(self, get_cpu_static_info, "GetCPUStaticInfo");
    }

    pub fn cpu_dynamic_info(&self) -> CpuDynamicInfo {
        dbus_call!(self, get_cpu_dynamic_info, "GetCPUDynamicInfo");
    }

    pub fn disks_info(&self) -> Vec<DiskInfo> {
        dbus_call!(self, get_disks_info, "GetDisksInfo");
    }

    pub fn fans_info(&self) -> Vec<FanInfo> {
        dbus_call!(self, get_fans_info, "GetFansInfo");
    }

    #[allow(unused)]
    pub fn gpu_list(&self) -> Vec<Arc<str>> {
        dbus_call!(self, get_gpu_list, "GetGPUList");
    }

    pub fn gpu_static_info(&self) -> Vec<GpuStaticInfo> {
        dbus_call!(self, get_gpu_static_info, "GetGPUStaticInfo");
    }

    pub fn gpu_dynamic_info(&self) -> Vec<GpuDynamicInfo> {
        dbus_call!(self, get_gpu_dynamic_info, "GetGPUDynamicInfo");
    }

    pub fn processes(&self) -> HashMap<u32, Process> {
        dbus_call!(self, get_processes, "GetProcesses");
    }

    pub fn apps(&self) -> HashMap<Arc<str>, App> {
        dbus_call!(self, get_apps, "GetApps");
    }

    pub fn services(&self) -> HashMap<Arc<str>, Service> {
        dbus_call!(self, get_services, "GetServices");
    }

    pub fn terminate_process(&self, pid: u32) {
        dbus_call!(self, terminate_process, "TerminateProcess", pid);
    }

    pub fn kill_process(&self, pid: u32) {
        dbus_call!(self, kill_process, "KillProcess", pid);
    }

    pub fn start_service(&self, service_name: &str) {
        dbus_call!(self, start_service, "StartService", service_name);
    }

    pub fn stop_service(&self, service_name: &str) {
        dbus_call!(self, stop_service, "StopService", service_name);
    }

    pub fn restart_service(&self, service_name: &str) {
        dbus_call!(self, restart_service, "RestartService", service_name);
    }

    pub fn enable_service(&self, service_name: &str) {
        dbus_call!(self, enable_service, "EnableService", service_name);
    }

    pub fn disable_service(&self, service_name: &str) {
        dbus_call!(self, disable_service, "DisableService", service_name);
    }

    pub fn get_service_logs(&self, service_name: &str, pid: Option<NonZeroU32>) -> Arc<str> {
        dbus_call!(self, get_service_logs, "GetServiceLogs", service_name, pid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gatherer_new() {
        let gatherer = Gatherer::new();
        assert!(gatherer.child.unwrap_or_else(|e| e.into_inner()).is_none());
    }

    #[test]
    fn test_gatherer_start() {
        let gatherer = Gatherer::new();
        let _ = gatherer.start();
        assert!(gatherer.child.unwrap_or_else(|e| e.into_inner()).is_some());
    }

    #[test]
    fn test_gatherer_stop() {
        let gatherer = Gatherer::new();
        let _ = gatherer.start();
        gatherer.stop();
    }
}
