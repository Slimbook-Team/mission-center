/* sys_info_v2/dbus_interface/mod.rs
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

use std::{
    collections::HashMap,
    mem::{align_of, size_of},
    num::NonZeroU32,
    rc::Rc,
    sync::Arc,
};

use dbus::{
    arg::ArgType,
    blocking::{LocalConnection, Proxy},
};
use static_assertions::const_assert;

pub use apps::{App, AppMap};
pub use arc_str_vec::ArcStrVec;
pub use cpu_dynamic_info::CpuDynamicInfo;
pub use cpu_static_info::CpuStaticInfo;
pub use disk_info::{DiskInfo, DiskInfoVec, DiskType};
pub use fan_info::{FanInfo, FanInfoVec};
pub use gpu_dynamic_info::{GpuDynamicInfo, GpuDynamicInfoVec};
pub use gpu_static_info::{GpuStaticInfo, GpuStaticInfoVec, OpenGLApi};
pub use processes::{Process, ProcessMap, ProcessUsageStats};
pub use service::{Service, ServiceMap};

mod apps;
mod arc_str_vec;
mod cpu_dynamic_info;
mod cpu_static_info;
mod disk_info;
mod fan_info;
mod gpu_dynamic_info;
mod gpu_static_info;
mod processes;
mod service;

pub const MC_GATHERER_OBJECT_PATH: &str = "/io/missioncenter/MissionCenter/Gatherer";
pub const MC_GATHERER_INTERFACE_NAME: &str = "io.missioncenter.MissionCenter.Gatherer";

// I don't know how to create one of these, so I just copy the one from the `dbus` crate.
#[allow(unused)]
struct TypeMismatchError {
    pub expected: ArgType,
    pub found: ArgType,
    pub position: u32,
}

impl TypeMismatchError {
    pub fn new(expected: ArgType, found: ArgType, position: u32) -> dbus::arg::TypeMismatchError {
        unsafe {
            std::mem::transmute(Self {
                expected,
                found,
                position,
            })
        }
    }
}

const_assert!(size_of::<TypeMismatchError>() == size_of::<dbus::arg::TypeMismatchError>());
const_assert!(align_of::<TypeMismatchError>() == align_of::<dbus::arg::TypeMismatchError>());

pub trait Gatherer {
    fn get_cpu_static_info(&self) -> Result<CpuStaticInfo, dbus::Error>;
    fn get_cpu_dynamic_info(&self) -> Result<CpuDynamicInfo, dbus::Error>;
    fn get_disks_info(&self) -> Result<Vec<DiskInfo>, dbus::Error>;
    fn get_fans_info(&self) -> Result<Vec<FanInfo>, dbus::Error>;
    fn get_gpu_list(&self) -> Result<Vec<Arc<str>>, dbus::Error>;
    fn get_gpu_static_info(&self) -> Result<Vec<GpuStaticInfo>, dbus::Error>;
    fn get_gpu_dynamic_info(&self) -> Result<Vec<GpuDynamicInfo>, dbus::Error>;
    fn get_apps(&self) -> Result<HashMap<Arc<str>, App>, dbus::Error>;
    fn get_processes(&self) -> Result<HashMap<u32, Process>, dbus::Error>;
    fn get_services(&self) -> Result<HashMap<Arc<str>, Service>, dbus::Error>;
    fn terminate_process(&self, process_id: u32) -> Result<(), dbus::Error>;
    fn kill_process(&self, process_id: u32) -> Result<(), dbus::Error>;
    fn enable_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn disable_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn start_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn stop_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn restart_service(&self, service_name: &str) -> Result<(), dbus::Error>;
    fn get_service_logs(
        &self,
        service_name: &str,
        pid: Option<NonZeroU32>,
    ) -> Result<Arc<str>, dbus::Error>;
}

impl<'a> Gatherer for Proxy<'a, Rc<LocalConnection>> {
    fn get_cpu_static_info(&self) -> Result<CpuStaticInfo, dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "GetCPUStaticInfo", ())
    }

    fn get_cpu_dynamic_info(&self) -> Result<CpuDynamicInfo, dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "GetCPUDynamicInfo", ())
    }

    fn get_disks_info(&self) -> Result<Vec<DiskInfo>, dbus::Error> {
        let res: Result<DiskInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetDisksInfo", ());
        res.map(|v| v.into())
    }

    fn get_fans_info(&self) -> Result<Vec<FanInfo>, dbus::Error> {
        let res: Result<FanInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetFansInfo", ());
        res.map(|v| v.into())
    }

    fn get_gpu_list(&self) -> Result<Vec<Arc<str>>, dbus::Error> {
        let res: Result<ArcStrVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetGPUList", ());
        res.map(|v| v.into())
    }

    fn get_gpu_static_info(&self) -> Result<Vec<GpuStaticInfo>, dbus::Error> {
        let res: Result<GpuStaticInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetGPUStaticInfo", ());
        res.map(|v| v.into())
    }

    fn get_gpu_dynamic_info(&self) -> Result<Vec<GpuDynamicInfo>, dbus::Error> {
        let res: Result<GpuDynamicInfoVec, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetGPUDynamicInfo", ());
        res.map(|v| v.into())
    }

    fn get_apps(&self) -> Result<HashMap<Arc<str>, App>, dbus::Error> {
        let res: Result<AppMap, _> = self.method_call(MC_GATHERER_INTERFACE_NAME, "GetApps", ());
        res.map(|v| v.into())
    }

    fn get_processes(&self) -> Result<HashMap<u32, Process>, dbus::Error> {
        let res: Result<ProcessMap, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetProcesses", ());
        res.map(|v| v.into())
    }

    fn get_services(&self) -> Result<HashMap<Arc<str>, Service>, dbus::Error> {
        let res: Result<ServiceMap, _> =
            self.method_call(MC_GATHERER_INTERFACE_NAME, "GetServices", ());
        res.map(|v| v.into())
    }

    fn terminate_process(&self, process_id: u32) -> Result<(), dbus::Error> {
        self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "TerminateProcess",
            (process_id,),
        )
    }

    fn kill_process(&self, process_id: u32) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "KillProcess", (process_id,))
    }

    fn enable_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "EnableService", (service_name,))
    }

    fn disable_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "DisableService",
            (service_name,),
        )
    }

    fn start_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "StartService", (service_name,))
    }

    fn stop_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(MC_GATHERER_INTERFACE_NAME, "StopService", (service_name,))
    }

    fn restart_service(&self, service_name: &str) -> Result<(), dbus::Error> {
        self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "RestartService",
            (service_name,),
        )
    }

    fn get_service_logs(
        &self,
        service_name: &str,
        pid: Option<NonZeroU32>,
    ) -> Result<Arc<str>, dbus::Error> {
        let res: Result<(String,), _> = self.method_call(
            MC_GATHERER_INTERFACE_NAME,
            "GetServiceLogs",
            (service_name, pid.map(|v| v.get()).unwrap_or(0)),
        );
        res.map(|v| Arc::<str>::from(v.0))
    }
}
