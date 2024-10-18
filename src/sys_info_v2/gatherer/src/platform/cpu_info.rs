/* sys_info_v2/gatherer/src/platform/cpu_info.rs
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

use dbus::arg::{Append, Arg};

#[repr(u8)]
pub enum OptionalBool {
    False,
    True,
    None,
}

impl From<Option<bool>> for OptionalBool {
    fn from(value: Option<bool>) -> Self {
        value.map_or(OptionalBool::None, |b| {
            if b {
                OptionalBool::True
            } else {
                OptionalBool::False
            }
        })
    }
}

/// Describes the static (unchanging) information about the CPU/system
pub trait CpuStaticInfoExt: Default + Append + Arg {
    /// The CPU vendor and model
    fn name(&self) -> &str;

    /// The number of logical CPUs (i.e. including SMT)
    fn logical_cpu_count(&self) -> u32;

    /// The number of physical CPU sockets
    fn socket_count(&self) -> Option<u8>;

    /// The base CPU frequency in kHz
    fn base_frequency_khz(&self) -> Option<u64>;

    /// The name of the virtualization technology available on this host
    fn virtualization_technology(&self) -> Option<&str>;

    /// Check if the OS is running in a virtual machine
    fn is_virtual_machine(&self) -> Option<bool>;

    /// The total amount of L1 cache (instruction and data)
    fn l1_combined_cache(&self) -> Option<u64>;

    /// The amount of L2 cache
    fn l2_cache(&self) -> Option<u64>;

    /// The amount of L3 cache
    fn l3_cache(&self) -> Option<u64>;

    /// The amount of L4 cache
    fn l4_cache(&self) -> Option<u64>;
}

impl Arg for crate::platform::CpuStaticInfo {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(suytsyt)")
    }
}

impl Append for crate::platform::CpuStaticInfo {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append_struct(|ia| {
            ia.append(self.name());
            ia.append(self.logical_cpu_count());
            ia.append(self.socket_count().unwrap_or(0));
            ia.append(self.base_frequency_khz().unwrap_or(0));
            ia.append(self.virtualization_technology().unwrap_or(""));
            ia.append(OptionalBool::from(self.is_virtual_machine()) as u8);
            ia.append(self.l1_combined_cache().unwrap_or(0));
            ia.append(self.l2_cache().unwrap_or(0));
            ia.append(self.l3_cache().unwrap_or(0));
            ia.append(self.l4_cache().unwrap_or(0));
        });
    }
}

/// Describes CPU/system information that changes over time
pub trait CpuDynamicInfoExt<'a>: Default + Append + Arg {
    /// An iterator that yields number of logical core f32 percentage values
    ///
    /// It is expected that the iterator yields as many values as exactly the number
    /// of CPU logical cores
    type Iter: Iterator<Item = &'a f32>;

    /// The overall utilization of the CPU(s)
    fn overall_utilization_percent(&self) -> f32;

    /// The overall utilization of the CPU(s) by the OS kernel
    fn overall_kernel_utilization_percent(&self) -> f32;

    /// The overall utilization of each logical core
    fn per_logical_cpu_utilization_percent(&'a self) -> Self::Iter;

    /// The overall utilization of each logical core by the OS kernel
    fn per_logical_cpu_kernel_utilization_percent(&'a self) -> Self::Iter;

    /// The current average CPU frequency
    fn current_frequency_mhz(&self) -> u64;

    /// The temperature of the CPU
    ///
    /// While all modern chips report several temperatures from the CPU die, it is expected that
    /// implementations provide the most user relevant value here
    fn temperature(&self) -> Option<f32>;

    /// The number of running processes in the system
    fn process_count(&self) -> u64;

    /// The number of active threads in the system
    fn thread_count(&self) -> u64;

    /// The number of open file handles in the system
    fn handle_count(&self) -> u64;

    /// The number of seconds that have passed since the OS was booted
    fn uptime_seconds(&self) -> u64;

    /// The cpufreq driver
    fn cpufreq_driver(&self) -> Option<&str>;

    /// The cpufreq governor
    fn cpufreq_governor(&self) -> Option<&str>;

    /// The energy performance preference
    fn energy_performance_preference(&self) -> Option<&str>;
}

impl Arg for crate::platform::CpuDynamicInfo {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(ddadadtdtttt)")
    }
}

impl Append for crate::platform::CpuDynamicInfo {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append_struct(|ia| {
            ia.append(self.overall_utilization_percent() as f64);
            ia.append(self.overall_kernel_utilization_percent() as f64);
            ia.append(
                self.per_logical_cpu_utilization_percent()
                    .map(|v| *v as f64)
                    .collect::<Vec<_>>(),
            );
            ia.append(
                self.per_logical_cpu_kernel_utilization_percent()
                    .map(|v| *v as f64)
                    .collect::<Vec<_>>(),
            );
            ia.append(self.current_frequency_mhz());
            ia.append(self.temperature().map_or(0_f64, |v| v as f64));
            ia.append(self.process_count());
            ia.append(self.thread_count());
            ia.append(self.handle_count());
            ia.append(self.uptime_seconds());
            ia.append(self.cpufreq_driver().unwrap_or(""));
            ia.append(self.cpufreq_governor().unwrap_or(""));
            ia.append(self.energy_performance_preference().unwrap_or(""));
        });
    }
}

/// Provides an interface for gathering CPU/System information.
pub trait CpuInfoExt<'a> {
    type S: CpuStaticInfoExt;
    type D: CpuDynamicInfoExt<'a>;
    type P: crate::platform::ProcessesExt<'a>;

    /// Refresh the internal static information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    ///
    /// It is expected that this is only called one during the lifetime of this instance, but
    /// implementation should not rely on this.
    fn refresh_static_info_cache(&mut self);

    /// Refresh the internal dynamic/continuously changing information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_dynamic_info_cache(&mut self, processes: &Self::P);

    /// Returns the static information for the CPU.
    fn static_info(&self) -> &Self::S;

    /// Returns the dynamic information for the CPU.
    fn dynamic_info(&self) -> &Self::D;
}
