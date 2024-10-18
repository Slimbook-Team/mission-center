/* sys_info_v2/gatherer/src/platform/linux/cpu_info.rs
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

use std::{fs::OpenOptions, io::Read, os::unix::ffi::OsStrExt, sync::Arc, time::Instant};

use super::{CPU_COUNT, INITIAL_REFRESH_TS, MIN_DELTA_REFRESH};
use crate::{critical, debug, platform::cpu_info::*};

const PROC_STAT_IGNORE: [usize; 2] = [0, 5];
const PROC_STAT_IDLE: [usize; 1] = [4];
const PROC_STAT_KERNEL: [usize; 2] = [6, 7];

#[derive(Debug, Copy, Clone)]
struct CpuTicks {
    used: u64,
    idle: u64,
    kernel: u64,
}

impl Default for CpuTicks {
    fn default() -> Self {
        Self {
            used: 0,
            idle: 0,
            kernel: 0,
        }
    }
}

impl CpuTicks {
    pub fn update(&mut self, line: &str) -> (f32, f32) {
        let failure = |e| {
            critical!("Gatherer::CPU", "Failed to read /proc/stat: {}", e);
            0
        };
        let fields = line.split_whitespace();
        let mut new_ticks = CpuTicks::default();
        for (pos, field) in fields.enumerate() {
            match pos {
                // 0 = cpu(num), 4 = idle, 6 + 7 kernel stuff, rest = busy time
                x if PROC_STAT_IGNORE.contains(&x) => (),
                x if PROC_STAT_IDLE.contains(&x) => {
                    new_ticks.idle += field.trim().parse::<u64>().unwrap_or_else(failure)
                }
                x if PROC_STAT_KERNEL.contains(&x) => {
                    new_ticks.kernel += field.trim().parse::<u64>().unwrap_or_else(failure)
                }
                _ => new_ticks.used += field.trim().parse::<u64>().unwrap_or_else(failure),
            }
        }
        let used = new_ticks.used - self.used;
        let kernel = new_ticks.kernel - self.kernel;
        let idle = new_ticks.idle - self.idle;
        let total = (used + kernel + idle) as f32;

        let util = (used + kernel) as f32 / total;
        let kernel_util = kernel as f32 / total;

        *self = new_ticks;
        (util * 100.0, kernel_util * 100.0)
    }
}

#[derive(Clone, Debug)]
pub struct LinuxCpuStaticInfo {
    name: Arc<str>,
    logical_cpu_count: u32,
    socket_count: Option<u8>,
    base_frequency_khz: Option<u64>,
    virtualization_technology: Option<Arc<str>>,
    is_virtual_machine: Option<bool>,
    l1_combined_cache: Option<u64>,
    l2_cache: Option<u64>,
    l3_cache: Option<u64>,
    l4_cache: Option<u64>,
}

impl Default for LinuxCpuStaticInfo {
    fn default() -> Self {
        Self {
            name: Arc::from(""),
            logical_cpu_count: 0,
            socket_count: None,
            base_frequency_khz: None,
            virtualization_technology: None,
            is_virtual_machine: None,
            l1_combined_cache: None,
            l2_cache: None,
            l3_cache: None,
            l4_cache: None,
        }
    }
}

impl LinuxCpuStaticInfo {
    pub fn new() -> Self {
        Default::default()
    }
}

impl CpuStaticInfoExt for LinuxCpuStaticInfo {
    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn logical_cpu_count(&self) -> u32 {
        self.logical_cpu_count
    }

    fn socket_count(&self) -> Option<u8> {
        self.socket_count
    }

    fn base_frequency_khz(&self) -> Option<u64> {
        self.base_frequency_khz
    }

    fn virtualization_technology(&self) -> Option<&str> {
        self.virtualization_technology.as_ref().map(|s| s.as_ref())
    }

    fn is_virtual_machine(&self) -> Option<bool> {
        self.is_virtual_machine
    }

    fn l1_combined_cache(&self) -> Option<u64> {
        self.l1_combined_cache
    }

    fn l2_cache(&self) -> Option<u64> {
        self.l2_cache
    }

    fn l3_cache(&self) -> Option<u64> {
        self.l3_cache
    }

    fn l4_cache(&self) -> Option<u64> {
        self.l4_cache
    }
}

#[derive(Clone, Default, Debug)]
pub struct LinuxCpuDynamicInfo {
    overall_utilization_percent: f32,
    overall_kernel_utilization_percent: f32,
    cpu_store_old: CpuTicks,
    per_logical_cpu_utilization_percent: Vec<f32>,
    per_logical_cpu_kernel_utilization_percent: Vec<f32>,
    per_logical_cpu_store_old: Vec<CpuTicks>,
    current_frequency_mhz: u64,
    temperature: Option<f32>,
    process_count: u64,
    thread_count: u64,
    handle_count: u64,
    uptime_seconds: u64,
    cpufreq_driver: Option<Arc<str>>,
    cpufreq_governor: Option<Arc<str>>,
    energy_performance_preference: Option<Arc<str>>,
}

impl LinuxCpuDynamicInfo {
    pub fn new() -> Self {
        Self {
            overall_utilization_percent: 0.0,
            overall_kernel_utilization_percent: 0.0,
            cpu_store_old: CpuTicks::default(),
            per_logical_cpu_utilization_percent: vec![],
            per_logical_cpu_kernel_utilization_percent: vec![],
            per_logical_cpu_store_old: vec![],
            current_frequency_mhz: 0,
            temperature: None,
            process_count: 0,
            thread_count: 0,
            handle_count: 0,
            uptime_seconds: 0,
            cpufreq_driver: None,
            cpufreq_governor: None,
            energy_performance_preference: None,
        }
    }
}

impl<'a> CpuDynamicInfoExt<'a> for LinuxCpuDynamicInfo {
    type Iter = std::slice::Iter<'a, f32>;

    fn overall_utilization_percent(&self) -> f32 {
        self.overall_utilization_percent
    }

    fn overall_kernel_utilization_percent(&self) -> f32 {
        self.overall_kernel_utilization_percent
    }

    fn per_logical_cpu_utilization_percent(&'a self) -> Self::Iter {
        self.per_logical_cpu_utilization_percent.iter()
    }

    fn per_logical_cpu_kernel_utilization_percent(&'a self) -> Self::Iter {
        self.per_logical_cpu_kernel_utilization_percent.iter()
    }

    fn current_frequency_mhz(&self) -> u64 {
        self.current_frequency_mhz
    }

    fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    fn process_count(&self) -> u64 {
        self.process_count
    }

    fn thread_count(&self) -> u64 {
        self.thread_count
    }

    fn handle_count(&self) -> u64 {
        self.handle_count
    }

    fn uptime_seconds(&self) -> u64 {
        self.uptime_seconds
    }

    fn cpufreq_driver(&self) -> Option<&str> {
        self.cpufreq_driver.as_ref().map(|s| s.as_ref())
    }

    fn cpufreq_governor(&self) -> Option<&str> {
        self.cpufreq_governor.as_ref().map(|s| s.as_ref())
    }
    fn energy_performance_preference(&self) -> Option<&str> {
        self.energy_performance_preference
            .as_ref()
            .map(|s| s.as_ref())
    }
}

#[derive(Debug)]
pub struct LinuxCpuInfo {
    static_info: LinuxCpuStaticInfo,
    dynamic_info: LinuxCpuDynamicInfo,

    static_refresh_timestamp: Instant,
    dynamic_refresh_timestamp: Instant,
}

impl LinuxCpuInfo {
    pub fn new() -> Self {
        let mut cpu_store_old = Vec::with_capacity(*CPU_COUNT + 1);
        cpu_store_old.resize(*CPU_COUNT + 1, CpuTicks::default());

        Self {
            static_info: LinuxCpuStaticInfo::new(),
            dynamic_info: LinuxCpuDynamicInfo::new(),

            static_refresh_timestamp: *INITIAL_REFRESH_TS,
            dynamic_refresh_timestamp: *INITIAL_REFRESH_TS,
        }
    }

    // Code lifted and adapted from `sysinfo` crate, found in src/linux/cpu.rs
    fn name() -> Arc<str> {
        fn get_value(s: &str) -> String {
            s.split(':')
                .last()
                .map(|x| x.trim().into())
                .unwrap_or("".to_owned())
        }

        fn get_hex_value(s: &str) -> u32 {
            s.split(':')
                .last()
                .map(|x| x.trim())
                .filter(|x| x.starts_with("0x"))
                .map(|x| u32::from_str_radix(&x[2..], 16).unwrap())
                .unwrap_or_default()
        }

        fn get_arm_implementer(implementer: u32) -> Option<&'static str> {
            Some(match implementer {
                0x41 => "ARM",
                0x42 => "Broadcom",
                0x43 => "Cavium",
                0x44 => "DEC",
                0x46 => "FUJITSU",
                0x48 => "HiSilicon",
                0x49 => "Infineon",
                0x4d => "Motorola/Freescale",
                0x4e => "NVIDIA",
                0x50 => "APM",
                0x51 => "Qualcomm",
                0x53 => "Samsung",
                0x56 => "Marvell",
                0x61 => "Apple",
                0x66 => "Faraday",
                0x69 => "Intel",
                0x70 => "Phytium",
                0xc0 => "Ampere",
                _ => return None,
            })
        }

        fn get_arm_part(implementer: u32, part: u32) -> Option<&'static str> {
            Some(match (implementer, part) {
                // ARM
                (0x41, 0x810) => "ARM810",
                (0x41, 0x920) => "ARM920",
                (0x41, 0x922) => "ARM922",
                (0x41, 0x926) => "ARM926",
                (0x41, 0x940) => "ARM940",
                (0x41, 0x946) => "ARM946",
                (0x41, 0x966) => "ARM966",
                (0x41, 0xa20) => "ARM1020",
                (0x41, 0xa22) => "ARM1022",
                (0x41, 0xa26) => "ARM1026",
                (0x41, 0xb02) => "ARM11 MPCore",
                (0x41, 0xb36) => "ARM1136",
                (0x41, 0xb56) => "ARM1156",
                (0x41, 0xb76) => "ARM1176",
                (0x41, 0xc05) => "Cortex-A5",
                (0x41, 0xc07) => "Cortex-A7",
                (0x41, 0xc08) => "Cortex-A8",
                (0x41, 0xc09) => "Cortex-A9",
                (0x41, 0xc0d) => "Cortex-A17", // Originally A12
                (0x41, 0xc0f) => "Cortex-A15",
                (0x41, 0xc0e) => "Cortex-A17",
                (0x41, 0xc14) => "Cortex-R4",
                (0x41, 0xc15) => "Cortex-R5",
                (0x41, 0xc17) => "Cortex-R7",
                (0x41, 0xc18) => "Cortex-R8",
                (0x41, 0xc20) => "Cortex-M0",
                (0x41, 0xc21) => "Cortex-M1",
                (0x41, 0xc23) => "Cortex-M3",
                (0x41, 0xc24) => "Cortex-M4",
                (0x41, 0xc27) => "Cortex-M7",
                (0x41, 0xc60) => "Cortex-M0+",
                (0x41, 0xd01) => "Cortex-A32",
                (0x41, 0xd02) => "Cortex-A34",
                (0x41, 0xd03) => "Cortex-A53",
                (0x41, 0xd04) => "Cortex-A35",
                (0x41, 0xd05) => "Cortex-A55",
                (0x41, 0xd06) => "Cortex-A65",
                (0x41, 0xd07) => "Cortex-A57",
                (0x41, 0xd08) => "Cortex-A72",
                (0x41, 0xd09) => "Cortex-A73",
                (0x41, 0xd0a) => "Cortex-A75",
                (0x41, 0xd0b) => "Cortex-A76",
                (0x41, 0xd0c) => "Neoverse-N1",
                (0x41, 0xd0d) => "Cortex-A77",
                (0x41, 0xd0e) => "Cortex-A76AE",
                (0x41, 0xd13) => "Cortex-R52",
                (0x41, 0xd20) => "Cortex-M23",
                (0x41, 0xd21) => "Cortex-M33",
                (0x41, 0xd40) => "Neoverse-V1",
                (0x41, 0xd41) => "Cortex-A78",
                (0x41, 0xd42) => "Cortex-A78AE",
                (0x41, 0xd43) => "Cortex-A65AE",
                (0x41, 0xd44) => "Cortex-X1",
                (0x41, 0xd46) => "Cortex-A510",
                (0x41, 0xd47) => "Cortex-A710",
                (0x41, 0xd48) => "Cortex-X2",
                (0x41, 0xd49) => "Neoverse-N2",
                (0x41, 0xd4a) => "Neoverse-E1",
                (0x41, 0xd4b) => "Cortex-A78C",
                (0x41, 0xd4c) => "Cortex-X1C",
                (0x41, 0xd4d) => "Cortex-A715",
                (0x41, 0xd4e) => "Cortex-X3",

                // Broadcom
                (0x42, 0x00f) => "Brahma-B15",
                (0x42, 0x100) => "Brahma-B53",
                (0x42, 0x516) => "ThunderX2",

                // Cavium
                (0x43, 0x0a0) => "ThunderX",
                (0x43, 0x0a1) => "ThunderX-88XX",
                (0x43, 0x0a2) => "ThunderX-81XX",
                (0x43, 0x0a3) => "ThunderX-83XX",
                (0x43, 0x0af) => "ThunderX2-99xx",

                // DEC
                (0x44, 0xa10) => "SA110",
                (0x44, 0xa11) => "SA1100",

                // Fujitsu
                (0x46, 0x001) => "A64FX",

                // HiSilicon
                (0x48, 0xd01) => "Kunpeng-920", // aka tsv110

                // NVIDIA
                (0x4e, 0x000) => "Denver",
                (0x4e, 0x003) => "Denver 2",
                (0x4e, 0x004) => "Carmel",

                // APM
                (0x50, 0x000) => "X-Gene",

                // Qualcomm
                (0x51, 0x00f) => "Scorpion",
                (0x51, 0x02d) => "Scorpion",
                (0x51, 0x04d) => "Krait",
                (0x51, 0x06f) => "Krait",
                (0x51, 0x201) => "Kryo",
                (0x51, 0x205) => "Kryo",
                (0x51, 0x211) => "Kryo",
                (0x51, 0x800) => "Falkor-V1/Kryo",
                (0x51, 0x801) => "Kryo-V2",
                (0x51, 0x802) => "Kryo-3XX-Gold",
                (0x51, 0x803) => "Kryo-3XX-Silver",
                (0x51, 0x804) => "Kryo-4XX-Gold",
                (0x51, 0x805) => "Kryo-4XX-Silver",
                (0x51, 0xc00) => "Falkor",
                (0x51, 0xc01) => "Saphira",

                // Samsung
                (0x53, 0x001) => "exynos-m1",

                // Marvell
                (0x56, 0x131) => "Feroceon-88FR131",
                (0x56, 0x581) => "PJ4/PJ4b",
                (0x56, 0x584) => "PJ4B-MP",

                // Apple
                (0x61, 0x020) => "Icestorm-A14",
                (0x61, 0x021) => "Firestorm-A14",
                (0x61, 0x022) => "Icestorm-M1",
                (0x61, 0x023) => "Firestorm-M1",
                (0x61, 0x024) => "Icestorm-M1-Pro",
                (0x61, 0x025) => "Firestorm-M1-Pro",
                (0x61, 0x028) => "Icestorm-M1-Max",
                (0x61, 0x029) => "Firestorm-M1-Max",
                (0x61, 0x030) => "Blizzard-A15",
                (0x61, 0x031) => "Avalanche-A15",
                (0x61, 0x032) => "Blizzard-M2",
                (0x61, 0x033) => "Avalanche-M2",

                // Faraday
                (0x66, 0x526) => "FA526",
                (0x66, 0x626) => "FA626",

                // Intel
                (0x69, 0x200) => "i80200",
                (0x69, 0x210) => "PXA250A",
                (0x69, 0x212) => "PXA210A",
                (0x69, 0x242) => "i80321-400",
                (0x69, 0x243) => "i80321-600",
                (0x69, 0x290) => "PXA250B/PXA26x",
                (0x69, 0x292) => "PXA210B",
                (0x69, 0x2c2) => "i80321-400-B0",
                (0x69, 0x2c3) => "i80321-600-B0",
                (0x69, 0x2d0) => "PXA250C/PXA255/PXA26x",
                (0x69, 0x2d2) => "PXA210C",
                (0x69, 0x411) => "PXA27x",
                (0x69, 0x41c) => "IPX425-533",
                (0x69, 0x41d) => "IPX425-400",
                (0x69, 0x41f) => "IPX425-266",
                (0x69, 0x682) => "PXA32x",
                (0x69, 0x683) => "PXA930/PXA935",
                (0x69, 0x688) => "PXA30x",
                (0x69, 0x689) => "PXA31x",
                (0x69, 0xb11) => "SA1110",
                (0x69, 0xc12) => "IPX1200",

                // Phytium
                (0x70, 0x660) => "FTC660",
                (0x70, 0x661) => "FTC661",
                (0x70, 0x662) => "FTC662",
                (0x70, 0x663) => "FTC663",

                _ => return None,
            })
        }

        let mut vendor_id = "".to_owned();
        let mut brand = "".to_owned();
        let mut implementer = None;
        let mut part = None;

        let cpuinfo = match std::fs::read_to_string("/proc/cpuinfo") {
            Ok(s) => s,
            Err(e) => {
                println!("Gatherer: Failed to read /proc/cpuinfo: {}", e);
                return Arc::from("");
            }
        };

        for it in cpuinfo.split('\n') {
            if it.starts_with("vendor_id\t") {
                vendor_id = get_value(it);
            } else if it.starts_with("model name\t") {
                brand = get_value(it);
            } else if it.starts_with("CPU implementer\t") {
                implementer = Some(get_hex_value(it));
            } else if it.starts_with("CPU part\t") {
                part = Some(get_hex_value(it));
            } else {
                continue;
            }
            if (!brand.is_empty() && !vendor_id.is_empty())
                || (implementer.is_some() && part.is_some())
            {
                break;
            }
        }

        if let (Some(implementer), Some(part)) = (implementer, part) {
            match get_arm_implementer(implementer) {
                Some(s) => vendor_id = s.into(),
                None => return Arc::from(brand),
            }

            match get_arm_part(implementer, part) {
                Some(s) => {
                    vendor_id.push(' ');
                    vendor_id.push_str(s);
                    brand = vendor_id;
                }
                _ => {}
            }
        }

        Arc::from(brand.replace("(R)", "®").replace("(TM)", "™"))
    }

    fn logical_cpu_count() -> u32 {
        *CPU_COUNT as u32
    }

    fn socket_count() -> Option<u8> {
        use std::{fs::*, io::*};

        let mut sockets = std::collections::HashSet::new();
        sockets.reserve(4);

        let mut buf = String::new();

        let entries = match read_dir("/sys/devices/system/cpu/") {
            Ok(entries) => entries,
            Err(e) => {
                critical!(
                    "Gatherer::CPU",
                    "Could not read '/sys/devices/system/cpu': {}",
                    e
                );
                return None;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    critical!(
                        "Gatherer::CPU",
                        "Could not read entry in '/sys/devices/system/cpu': {}",
                        e
                    );
                    continue;
                }
            };

            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(e) => {
                    critical!(
                        "Gatherer::CPU",
                        "Could not read file type for '/sys/devices/system/cpu/{}': {}",
                        entry.file_name().to_string_lossy(),
                        e
                    );
                    continue;
                }
            };

            if !file_type.is_dir() {
                continue;
            }

            let mut file = match File::open(entry.path().join("topology/physical_package_id")) {
                Ok(file) => file,
                Err(_) => {
                    continue;
                }
            };

            buf.clear();
            match file.read_to_string(&mut buf) {
                Ok(_) => {}
                Err(e) => {
                    critical!(
                        "Gatherer::CPU",
                        "Could not read '/sys/devices/system/cpu/{}/topology/physical_package_id': {}",
                        file_name,
                        e
                    );
                    continue;
                }
            };

            let socket_id = match buf.trim().parse::<u8>() {
                Ok(socket_id) => socket_id,
                Err(e) => {
                    critical!(
                        "Gatherer::CPU",
                        "Could not read '/sys/devices/system/cpu/{}/topology/physical_package_id': {}",
                        file_name,
                        e
                    );
                    continue;
                }
            };
            sockets.insert(socket_id);
        }

        if sockets.is_empty() {
            critical!("Gatherer::CPU", "Could not determine socket count");
            None
        } else {
            Some(sockets.len() as u8)
        }
    }

    fn base_frequency_khz() -> Option<u64> {
        fn read_from_sys_base_frequency() -> Option<u64> {
            match std::fs::read("/sys/devices/system/cpu/cpu0/cpufreq/base_frequency") {
                Ok(content) => {
                    let content = match std::str::from_utf8(&content) {
                        Ok(content) => content,
                        Err(e) => {
                            critical!(
                                "Gatherer::CPU",
                                "Could not read base frequency from '/sys/devices/system/cpu/cpu0/cpufreq/base_frequency': {}",
                                e
                            );
                            return None;
                        }
                    };

                    match content.trim().parse() {
                        Ok(freq) => Some(freq),
                        Err(e) => {
                            critical!(
                                "Gatherer::CPU",
                                "Could not read base frequency from '/sys/devices/system/cpu/cpu0/cpufreq/base_frequency': {}",
                                e
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    debug!(
                        "Gatherer::CPU",
                        "Could not read base frequency from '/sys/devices/system/cpu/cpu0/cpufreq/base_frequency': {}",
                        e
                    );

                    None
                }
            }
        }

        fn read_from_sys_bios_limit() -> Option<u64> {
            match std::fs::read("/sys/devices/system/cpu/cpu0/cpufreq/bios_limit") {
                Ok(content) => {
                    let content = match std::str::from_utf8(&content) {
                        Ok(content) => content,
                        Err(e) => {
                            critical!(
                                "Gatherer::CPU",
                                "Could not read base frequency from '/sys/devices/system/cpu/cpu0/cpufreq/bios_limit': {}",
                                e
                            );
                            return None;
                        }
                    };

                    match content.trim().parse() {
                        Ok(freq) => Some(freq),
                        Err(e) => {
                            critical!(
                                "Gatherer::CPU",
                                "Could not read base frequency from '/sys/devices/system/cpu/cpu0/cpufreq/bios_limit': {}",
                                e
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    debug!(
                        "Gatherer::CPU",
                        "Could not read base frequency from '/sys/devices/system/cpu/cpu0/cpufreq/bios_limit': {}",
                        e
                    );

                    None
                }
            }
        }

        const FNS: &[fn() -> Option<u64>] =
            &[read_from_sys_base_frequency, read_from_sys_bios_limit];

        for f in FNS {
            if let Some(freq) = f() {
                return Some(freq);
            }
        }

        None
    }

    fn virtualization() -> Option<Arc<str>> {
        use crate::warning;
        use std::io::Read;

        let mut virtualization: Option<Arc<str>> = None;

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        match std::fs::read_to_string("/proc/cpuinfo") {
            Ok(cpuinfo) => {
                for line in cpuinfo.split('\n').map(|l| l.trim()) {
                    if line.starts_with("flags") {
                        for flag in line.split(':').nth(1).unwrap_or("").trim().split(' ') {
                            if flag == "vmx" {
                                virtualization = Some("Intel VT-x".into());
                                break;
                            }

                            if flag == "svm" {
                                virtualization = Some("AMD-V".into());
                            }
                        }

                        break;
                    }
                }
            }
            Err(e) => {
                warning!(
                    "Gatherer::CPU",
                    "Failed to read virtualization capabilities from `/proc/cpuinfo`: {}",
                    e
                );
            }
        }

        if std::path::Path::new("/dev/kvm").exists() {
            virtualization = if let Some(virt) = virtualization.as_ref() {
                Some(Arc::from(format!("KVM / {}", virt).as_str()))
            } else {
                Some("KVM".into())
            };
        } else {
            debug!("Gatherer::CPU", "Virtualization: `/dev/kvm` does not exist");
        }

        let mut buffer = [0u8; 9];
        match std::fs::File::open("/proc/xen/capabilities") {
            Ok(mut file) => {
                file.read(&mut buffer).unwrap();
                if &buffer == b"control_d" {
                    virtualization = if let Some(virt) = virtualization.as_ref() {
                        if virt.as_ref().starts_with("KVM") {
                            Some(Arc::from(format!("KVM & Xen / {}", virt).as_str()))
                        } else {
                            Some(Arc::from(format!("Xen / {}", virt).as_str()))
                        }
                    } else {
                        Some("Xen".into())
                    };
                }
            }
            Err(e) => {
                debug!(
                    "Gatherer::CPU",
                    "Virtualization: Failed to open /proc/xen/capabilities: {}", e
                );
            }
        }

        virtualization
    }

    fn virtual_machine() -> Option<bool> {
        use dbus::blocking::{stdintf::org_freedesktop_dbus::Properties, *};

        let conn = match Connection::new_system() {
            Ok(c) => c,
            Err(e) => {
                critical!(
                    "Gatherer::CPU",
                    "Failed to determine VM: Failed to connect to D-Bus: {}",
                    e
                );
                return None;
            }
        };

        let proxy = conn.with_proxy(
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            std::time::Duration::from_millis(1000),
        );

        let response: String = match proxy.get("org.freedesktop.systemd1.Manager", "Virtualization")
        {
            Ok(m) => m,
            Err(e) => {
                critical!(
                    "Gatherer::CPU",
                    "Failed to determine VM: Failed to get Virtualization property: {}",
                    e
                );
                return None;
            }
        };

        Some(response.len() > 0)
    }

    fn cache_info() -> [Option<u64>; 5] {
        use crate::warning;
        use std::{collections::HashSet, fs::*, os::unix::prelude::*, str::FromStr};

        fn read_index_entry_content(
            file_name: &str,
            index_path: &std::path::Path,
        ) -> Option<String> {
            let path = index_path.join(file_name);
            match read_to_string(path) {
                Ok(content) => Some(content),
                Err(e) => {
                    warning!(
                        "Gatherer::CPU",
                        "Could not read '{}/{}': {}",
                        index_path.display(),
                        file_name,
                        e,
                    );
                    None
                }
            }
        }

        fn read_index_entry_number<R: FromStr<Err = core::num::ParseIntError>>(
            file_name: &str,
            index_path: &std::path::Path,
            suffix: Option<&str>,
        ) -> Option<R> {
            let content = match read_index_entry_content(file_name, index_path) {
                Some(content) => content,
                None => return None,
            };
            let content = content.trim();
            let value = match suffix {
                None => content.parse::<R>(),
                Some(suffix) => content.trim_end_matches(suffix).parse::<R>(),
            };
            match value {
                Err(e) => {
                    warning!(
                        "Gatherer::CPU",
                        "Failed to parse '{}/{}': {}",
                        index_path.display(),
                        file_name,
                        e,
                    );
                    None
                }
                Ok(v) => Some(v),
            }
        }

        fn read_cache_values(path: &std::path::Path) -> [Option<u64>; 5] {
            let mut result = [None; 5];

            let mut l1_visited_data = HashSet::new();
            let mut l1_visited_instr = HashSet::new();
            let mut l2_visited = HashSet::new();
            let mut l3_visited = HashSet::new();
            let mut l4_visited = HashSet::new();

            let cpu_entries = match path.read_dir() {
                Ok(entries) => entries,
                Err(e) => {
                    warning!(
                        "Gatherer::CPU",
                        "Could not read '{}': {}",
                        path.display(),
                        e
                    );
                    return result;
                }
            };
            for cpu_entry in cpu_entries {
                let cpu_entry = match cpu_entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        warning!(
                            "Gatherer::CPU",
                            "Could not read cpu entry in '{}': {}",
                            path.display(),
                            e
                        );
                        continue;
                    }
                };
                let mut path = cpu_entry.path();

                let cpu_name = match path.file_name() {
                    Some(name) => name,
                    None => continue,
                };

                let is_cpu = &cpu_name.as_bytes()[0..3] == b"cpu";
                if is_cpu {
                    let cpu_number =
                        match unsafe { std::str::from_utf8_unchecked(&cpu_name.as_bytes()[3..]) }
                            .parse::<u16>()
                        {
                            Ok(n) => n,
                            Err(_) => continue,
                        };

                    path.push("cache");
                    let cache_entries = match path.read_dir() {
                        Ok(entries) => entries,
                        Err(e) => {
                            warning!(
                                "Gatherer::CPU",
                                "Could not read '{}': {}",
                                path.display(),
                                e
                            );
                            return result;
                        }
                    };
                    for cache_entry in cache_entries {
                        let cache_entry = match cache_entry {
                            Ok(entry) => entry,
                            Err(e) => {
                                warning!(
                                    "Gatherer::CPU",
                                    "Could not read cpu entry in '{}': {}",
                                    path.display(),
                                    e
                                );
                                continue;
                            }
                        };
                        let path = cache_entry.path();
                        let is_cache_entry = path
                            .file_name()
                            .map(|file| &file.as_bytes()[0..5] == b"index")
                            .unwrap_or(false);
                        if is_cache_entry {
                            let level = match read_index_entry_number::<u8>("level", &path, None) {
                                None => continue,
                                Some(l) => l,
                            };

                            let cache_type = match read_index_entry_content("type", &path) {
                                None => continue,
                                Some(ct) => ct,
                            };

                            let visited_cpus = match cache_type.trim() {
                                "Data" => &mut l1_visited_data,
                                "Instruction" => &mut l1_visited_instr,
                                "Unified" => match level {
                                    2 => &mut l2_visited,
                                    3 => &mut l3_visited,
                                    4 => &mut l4_visited,
                                    _ => continue,
                                },
                                _ => continue,
                            };

                            if visited_cpus.contains(&cpu_number) {
                                continue;
                            }

                            let size =
                                match read_index_entry_number::<usize>("size", &path, Some("K")) {
                                    None => continue,
                                    Some(s) => s,
                                };

                            let result_index = level as usize;
                            result[result_index] = match result[result_index] {
                                None => Some(size as u64),
                                Some(s) => Some(s + size as u64),
                            };

                            match read_index_entry_content("shared_cpu_list", &path) {
                                Some(scl) => {
                                    let shared_cpu_list = scl.trim().split(',');
                                    for cpu in shared_cpu_list {
                                        let mut shared_cpu_sequence = cpu.split('-');

                                        let start = match shared_cpu_sequence
                                            .next()
                                            .map(|s| s.parse::<u16>())
                                        {
                                            Some(Ok(s)) => s,
                                            Some(Err(_)) | None => continue,
                                        };

                                        let end = match shared_cpu_sequence
                                            .next()
                                            .map(|e| e.parse::<u16>())
                                        {
                                            Some(Ok(e)) => e,
                                            Some(Err(_)) | None => {
                                                visited_cpus.insert(start);
                                                continue;
                                            }
                                        };

                                        for i in start..=end {
                                            visited_cpus.insert(i);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }

            result
        }

        let mut result = [None; 5];

        match read_dir("/sys/devices/system/node/") {
            Ok(entries) => {
                for nn_entry in entries {
                    let nn_entry = match nn_entry {
                        Ok(entry) => entry,
                        Err(e) => {
                            warning!(
                                "Gatherer::CPU",
                                "Could not read entry in '/sys/devices/system/node': {}",
                                e
                            );
                            continue;
                        }
                    };
                    let path = nn_entry.path();
                    if !path.is_dir() {
                        continue;
                    }

                    let is_node = path
                        .file_name()
                        .map(|file| &file.as_bytes()[0..4] == b"node")
                        .unwrap_or(false);
                    if !is_node {
                        continue;
                    }

                    let node_vals = read_cache_values(&path);
                    for i in 0..result.len() {
                        if let Some(size) = node_vals[i] {
                            result[i] = match result[i] {
                                None => Some(size),
                                Some(s) => Some(s + size),
                            };
                        }
                    }
                }
            }
            Err(e) => {
                warning!(
                    "Gatherer::CPU",
                    "Could not read '/sys/devices/system/node': {}. Falling back to '/sys/devices/system/cpu'",
                    e
                );

                result = read_cache_values(std::path::Path::new("/sys/devices/system/cpu"));
            }
        }

        for i in 1..result.len() {
            result[i] = result[i].map(|size| size * 1024);
        }
        result
    }

    fn get_cpufreq_driver_governor() -> (Option<Arc<str>>, Option<Arc<str>>, Option<Arc<str>>) {
        fn get_cpufreq_driver() -> Option<Arc<str>> {
            match std::fs::read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_driver") {
                Ok(content) => match std::str::from_utf8(&content) {
                    Ok(content) => Some(Arc::from(format!("{}", content.trim()).as_str())),
                    Err(e) => {
                        debug!(
                            "Gatherer::CPU",
                            "Could not read cpufreq driver from '/sys/devices/system/cpu/cpu0/cpufreq/scaling_driver': {}",
                            e
                        );

                        None
                    }
                },
                Err(e) => {
                    debug!(
                        "Gatherer::CPU",
                        "Could not read cpufreq driver from '/sys/devices/system/cpu/cpu0/cpufreq/scaling_driver': {}",
                        e
                    );

                    None
                }
            }
        }

        fn get_cpufreq_governor() -> Option<Arc<str>> {
            match std::fs::read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") {
                Ok(content) => match std::str::from_utf8(&content) {
                    Ok(content) => Some(Arc::from(format!("{}", content.trim()).as_str())),
                    Err(e) => {
                        debug!(
                            "Gatherer::CPU",
                            "Could not read cpufreq governor from '/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor': {}",
                            e
                        );

                        None
                    }
                },
                Err(e) => {
                    debug!(
                        "Gatherer::CPU",
                        "Could not read cpufreq governor from '/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor': {}",
                        e
                    );

                    None
                }
            }
        }
        // shouldn't show error as few people have this / the error would be normal
        fn energy_performance_preference() -> Option<Arc<str>> {
            match std::fs::read(
                "/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference",
            ) {
                Ok(content) => match std::str::from_utf8(&content) {
                    Ok(content) => Some(Arc::from(format!("{}", content.trim()).as_str())),
                    Err(e) => {
                        critical!(
                                "Gatherer::CPU",
                                "Could not read power preference from '/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference': {}",
                                e
                            );

                        None
                    }
                },
                Err(_) => None,
            }
        }
        (
            get_cpufreq_driver(),
            get_cpufreq_governor(),
            energy_performance_preference(),
        )
    }

    // Adapted from `sysinfo` crate, linux/cpu.rs:415
    fn cpu_frequency_mhz() -> u64 {
        #[inline(always)]
        fn read_sys_cpufreq() -> Option<u64> {
            let mut result = 0_u64;

            let sys_dev_cpu = match std::fs::read_dir("/sys/devices/system/cpu") {
                Ok(d) => d,
                Err(e) => {
                    debug!(
                        "Gatherer::CPU",
                        "Failed to read frequency: Failed to open /sys/devices/system/cpu: {}", e
                    );
                    return None;
                }
            };

            let mut buffer = String::new();
            for cpu in sys_dev_cpu.filter_map(|d| d.ok()).filter(|d| {
                d.file_name().as_bytes().starts_with(b"cpu")
                    && d.file_type().is_ok_and(|ty| ty.is_dir())
            }) {
                buffer.clear();

                let mut path = cpu.path();
                path.push("cpufreq/scaling_cur_freq");

                let mut file = match OpenOptions::new().read(true).open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        debug!(
                            "Gatherer::CPU",
                            "Failed to read frequency: Failed to open /sys/devices/system/cpu/{}/cpufreq/scaling_cur_freq: {}",
                            cpu.file_name().to_string_lossy(),
                            e
                        );
                        continue;
                    }
                };

                match file.read_to_string(&mut buffer) {
                    Ok(_) => {}
                    Err(e) => {
                        debug!(
                            "Gatherer::CPU",
                            "Failed to read frequency: Failed to read /sys/devices/system/cpu/{}/cpufreq/scaling_cur_freq: {}",
                            cpu.file_name().to_string_lossy(),
                            e
                        );
                        continue;
                    }
                }

                let freq = match buffer.trim().parse::<u64>() {
                    Ok(f) => f,
                    Err(e) => {
                        debug!(
                            "Gatherer::CPU",
                            "Failed to read frequency: Failed to parse /sys/devices/system/cpu/{}/cpufreq/scaling_cur_freq: {}",
                            cpu.file_name().to_string_lossy(),
                            e
                        );
                        continue;
                    }
                };

                result = result.max(freq);
            }

            if result > 0 {
                Some(result / 1000)
            } else {
                None
            }
        }

        #[inline(always)]
        fn read_proc_cpuinfo() -> Option<u64> {
            let cpuinfo = match std::fs::read_to_string("/proc/cpuinfo") {
                Ok(s) => s,
                Err(e) => {
                    debug!(
                        "Gatherer::CPU",
                        "Failed to read frequency: Failed to open /proc/cpuinfo: {}", e
                    );
                    return None;
                }
            };

            let mut result = 0;
            for line in cpuinfo
                .split('\n')
                .filter(|line| line.starts_with("cpu MHz\t") || line.starts_with("clock\t"))
            {
                result = line
                    .split(':')
                    .last()
                    .and_then(|val| val.replace("MHz", "").trim().parse::<f64>().ok())
                    .map(|speed| speed as u64)
                    .unwrap_or_default()
                    .max(result);
            }

            Some(result)
        }

        if let Some(freq) = read_sys_cpufreq() {
            return freq;
        }

        read_proc_cpuinfo().unwrap_or_default()
    }

    fn temperature() -> Option<f32> {
        let dir = match std::fs::read_dir("/sys/class/hwmon") {
            Ok(d) => d,
            Err(e) => {
                critical!("Gatherer::CPU", "Failed to open `/sys/class/hwmon`: {}", e);
                return None;
            }
        };

        for mut entry in dir
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|path| path.is_dir())
        {
            let mut name = entry.clone();
            name.push("name");

            let name = match std::fs::read_to_string(name) {
                Ok(name) => name.trim().to_lowercase(),
                Err(_) => continue,
            };
            if name != "k10temp" && name != "coretemp" && name != "zenpower" {
                continue;
            }

            entry.push("temp1_input");
            let temp = match std::fs::read_to_string(&entry) {
                Ok(temp) => temp,
                Err(e) => {
                    critical!(
                        "Gatherer::CPU",
                        "Failed to read temperature from `{}`: {}",
                        entry.display(),
                        e
                    );
                    continue;
                }
            };

            return Some(match temp.trim().parse::<u32>() {
                Ok(temp) => (temp as f32) / 1000.,
                Err(e) => {
                    critical!(
                        "Gatherer::CPU",
                        "Failed to parse temperature from `{}`: {}",
                        entry.display(),
                        e
                    );
                    continue;
                }
            });
        }

        None
    }

    fn process_count(processes: &crate::platform::Processes) -> u64 {
        use crate::platform::ProcessesExt;

        processes.process_list().len() as _
    }

    fn thread_count(processes: &crate::platform::Processes) -> u64 {
        use crate::platform::{ProcessExt, ProcessesExt};

        processes
            .process_list()
            .iter()
            .map(|(_, p)| p.task_count())
            .sum::<usize>() as _
    }

    fn handle_count() -> u64 {
        let file_nr = match std::fs::read_to_string("/proc/sys/fs/file-nr") {
            Ok(s) => s,
            Err(e) => {
                critical!(
                    "Gatherer::CPU",
                    "Failed to get handle count, could not read /proc/sys/fs/file-nr: {}",
                    e
                );
                return 0;
            }
        };
        let file_nr = match file_nr.split_whitespace().next() {
            Some(s) => s,
            None => {
                critical!(
                    "Gatherer::CPU",
                    "Failed to get handle count, failed to parse /proc/sys/fs/file-nr",
                );
                return 0;
            }
        };

        match file_nr.trim().parse() {
            Ok(count) => count,
            Err(e) => {
                critical!("Gatherer::CPU", "Failed to get handle count, failed to parse /proc/sys/fs/file-nr content ({}): {}", file_nr, e);
                0
            }
        }
    }

    fn uptime() -> std::time::Duration {
        let proc_uptime = match std::fs::read_to_string("/proc/uptime") {
            Ok(s) => s,
            Err(e) => {
                critical!(
                    "Gatherer::CPU",
                    "Failed to get handle count, could not read /proc/sys/fs/file-nr: {}",
                    e
                );
                return std::time::Duration::from_millis(0);
            }
        };

        match proc_uptime
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim()
            .parse::<f64>()
        {
            Ok(count) => std::time::Duration::from_secs_f64(count),
            Err(e) => {
                critical!(
                    "Gatherer::CPU",
                    "Failed to parse uptime, failed to parse /proc/uptime content ({}): {}",
                    proc_uptime,
                    e
                );
                std::time::Duration::from_millis(0)
            }
        }
    }
}

impl<'a> CpuInfoExt<'a> for LinuxCpuInfo {
    type S = LinuxCpuStaticInfo;
    type D = LinuxCpuDynamicInfo;
    type P = crate::platform::Processes;

    fn refresh_static_info_cache(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.static_refresh_timestamp) < MIN_DELTA_REFRESH {
            return;
        }
        self.static_refresh_timestamp = now;

        if self.static_info.logical_cpu_count == 0 {
            let cache_info = Self::cache_info();

            self.static_info = LinuxCpuStaticInfo {
                name: Self::name(),
                logical_cpu_count: Self::logical_cpu_count(),
                socket_count: Self::socket_count(),
                base_frequency_khz: Self::base_frequency_khz(),
                virtualization_technology: Self::virtualization(),
                is_virtual_machine: Self::virtual_machine(),
                l1_combined_cache: cache_info[1],
                l2_cache: cache_info[2],
                l3_cache: cache_info[3],
                l4_cache: cache_info[4],
            }
        }
    }

    fn refresh_dynamic_info_cache(&mut self, processes: &crate::platform::Processes) {
        let now = Instant::now();
        if now.duration_since(self.dynamic_refresh_timestamp) < MIN_DELTA_REFRESH {
            return;
        }
        self.dynamic_refresh_timestamp = now;

        self.dynamic_info
            .per_logical_cpu_utilization_percent
            .resize(*CPU_COUNT, 0.0);
        self.dynamic_info
            .per_logical_cpu_kernel_utilization_percent
            .resize(*CPU_COUNT, 0.0);
        self.dynamic_info
            .per_logical_cpu_store_old
            .resize(*CPU_COUNT, CpuTicks::default());

        let per_core_usage =
            &mut self.dynamic_info.per_logical_cpu_utilization_percent[..*CPU_COUNT];
        let per_core_kernel_usage =
            &mut self.dynamic_info.per_logical_cpu_kernel_utilization_percent[..*CPU_COUNT];
        let per_core_save = &mut self.dynamic_info.per_logical_cpu_store_old[..*CPU_COUNT];

        let proc_stat = std::fs::read_to_string("/proc/stat").unwrap_or_else(|e| {
            critical!("Gatherer::CPU", "Failed to read /proc/stat: {}", e);
            "".to_owned()
        });

        let mut line_iter = proc_stat
            .lines()
            .map(|l| l.trim())
            .skip_while(|l| !l.starts_with("cpu"));
        if let Some(cpu_overall_line) = line_iter.next() {
            (
                self.dynamic_info.overall_utilization_percent,
                self.dynamic_info.overall_kernel_utilization_percent,
            ) = self.dynamic_info.cpu_store_old.update(cpu_overall_line);

            for (i, line) in line_iter.enumerate() {
                if i >= *CPU_COUNT || !line.starts_with("cpu") {
                    break;
                }

                (per_core_usage[i], per_core_kernel_usage[i]) = per_core_save[i].update(line);
            }
        } else {
            self.dynamic_info.overall_utilization_percent = 0.;
            self.dynamic_info.overall_kernel_utilization_percent = 0.;
            per_core_usage.fill(0.);
            per_core_kernel_usage.fill(0.);
        }
        (
            self.dynamic_info.cpufreq_driver,
            self.dynamic_info.cpufreq_governor,
            self.dynamic_info.energy_performance_preference,
        ) = Self::get_cpufreq_driver_governor();

        self.dynamic_info.current_frequency_mhz = Self::cpu_frequency_mhz();
        self.dynamic_info.temperature = Self::temperature();
        self.dynamic_info.process_count = Self::process_count(processes);
        self.dynamic_info.thread_count = Self::thread_count(processes);
        self.dynamic_info.handle_count = Self::handle_count();
        self.dynamic_info.uptime_seconds = Self::uptime().as_secs();

        self.static_refresh_timestamp = Instant::now();
    }

    fn static_info(&self) -> &Self::S {
        &self.static_info
    }

    fn dynamic_info(&self) -> &Self::D {
        &self.dynamic_info
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_static_info() {
        let mut cpu = LinuxCpuInfo::new();
        cpu.refresh_static_info_cache();
        assert!(!cpu.static_info().name().is_empty());

        dbg!(cpu.static_info());
    }

    #[test]
    fn test_dynamic_info() {
        use crate::platform::{Processes, ProcessesExt};

        let mut p = Processes::new();
        p.refresh_cache();

        let mut cpu = LinuxCpuInfo::new();
        cpu.refresh_dynamic_info_cache(&p);
        assert!(!cpu.dynamic_info().process_count() > 0);

        dbg!(cpu.dynamic_info());
    }
}
