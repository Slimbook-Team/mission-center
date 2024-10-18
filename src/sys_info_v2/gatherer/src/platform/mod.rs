/* sys_info_v2/gatherer/src/platform/mod.rs
 *
 * Copyright 2023 Romeo Calota
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

pub use apps::*;
pub use cpu_info::*;
pub use disk_info::*;
pub use fan_info::*;
pub use gpu_info::*;
#[cfg(target_os = "linux")]
pub use linux::*;
pub use processes::*;
pub use services::*;
pub use utilities::*;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform_impl;

#[cfg(target_os = "linux")]
#[allow(unused)]
mod linux {
    use super::*;

    pub type Process = platform_impl::LinuxProcess;
    pub type Processes = platform_impl::LinuxProcesses;
    pub type App = platform_impl::LinuxApp;
    pub type Apps = platform_impl::LinuxApps;
    pub type CpuStaticInfo = platform_impl::LinuxCpuStaticInfo;
    pub type CpuDynamicInfo = platform_impl::LinuxCpuDynamicInfo;
    pub type DiskInfo = platform_impl::LinuxDiskInfo;
    pub type DiskInfoIter<'a> = platform_impl::LinuxDiskInfoIter<'a>;
    pub type DisksInfo = platform_impl::LinuxDisksInfo;
    pub type FanInfo = platform_impl::LinuxFanInfo;
    pub type FanInfoIter<'a> = platform_impl::LinuxFanInfoIter<'a>;
    pub type FansInfo = platform_impl::LinuxFansInfo;
    pub type CpuInfo = platform_impl::LinuxCpuInfo;
    pub type GpuStaticInfo = platform_impl::LinuxGpuStaticInfo;
    pub type GpuDynamicInfo = platform_impl::LinuxGpuDynamicInfo;
    pub type GpuInfo = platform_impl::LinuxGpuInfo;
    pub type Service = platform_impl::LinuxService;
    pub type Services<'a> = platform_impl::LinuxServices<'a>;
    pub type ServiceController<'a> = platform_impl::LinuxServiceController<'a>;
    pub type ServicesError = platform_impl::LinuxServicesError;
    pub type PlatformUtilities = platform_impl::LinuxPlatformUtilities;
}

mod apps;
mod cpu_info;
mod disk_info;
mod fan_info;
mod gpu_info;
mod processes;
mod services;
mod utilities;
