/* sys_info_v2/gatherer/src/platform/linux/disk_info.rs
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

use super::{INITIAL_REFRESH_TS, MIN_DELTA_REFRESH};
use crate::logging::{critical, warning};
use crate::platform::disk_info::{DiskInfoExt, DiskType, DisksInfoExt};
use glob::glob;
use serde::Deserialize;
use std::{sync::Arc, time::Instant};

#[derive(Debug, Default, Deserialize)]
struct LSBLKBlockDevice {
    name: String,
    mountpoints: Vec<Option<String>>,
    children: Option<Vec<Option<LSBLKBlockDevice>>>,
}

#[derive(Debug, Deserialize)]
struct LSBLKOutput {
    blockdevices: Vec<Option<LSBLKBlockDevice>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinuxDiskInfo {
    pub id: Arc<str>,
    pub model: Arc<str>,
    pub r#type: DiskType,
    pub capacity: u64,
    pub formatted: u64,
    pub system_disk: bool,

    pub busy_percent: f32,
    pub response_time_ms: f32,
    pub read_speed: u64,
    pub write_speed: u64,
}

impl Default for LinuxDiskInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            model: Arc::from(""),
            r#type: DiskType::default(),
            capacity: 0,
            formatted: 0,
            system_disk: false,

            busy_percent: 0.,
            response_time_ms: 0.,
            read_speed: 0,
            write_speed: 0,
        }
    }
}

pub struct LinuxDiskInfoIter<'a>(
    pub  std::iter::Map<
        std::slice::Iter<'a, (DiskStats, LinuxDiskInfo)>,
        fn(&'a (DiskStats, LinuxDiskInfo)) -> &'a LinuxDiskInfo,
    >,
);

impl<'a> LinuxDiskInfoIter<'a> {
    pub fn new(iter: std::slice::Iter<'a, (DiskStats, LinuxDiskInfo)>) -> Self {
        Self(iter.map(|(_, di)| di))
    }
}

impl<'a> Iterator for LinuxDiskInfoIter<'a> {
    type Item = &'a LinuxDiskInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl DiskInfoExt for LinuxDiskInfo {
    fn id(&self) -> &str {
        &self.id
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn r#type(&self) -> DiskType {
        self.r#type
    }

    fn capacity(&self) -> u64 {
        self.capacity
    }

    fn formatted(&self) -> u64 {
        self.formatted
    }

    fn is_system_disk(&self) -> bool {
        self.system_disk
    }

    fn busy_percent(&self) -> f32 {
        self.busy_percent
    }

    fn response_time_ms(&self) -> f32 {
        self.response_time_ms
    }

    fn read_speed(&self) -> u64 {
        self.read_speed
    }

    fn write_speed(&self) -> u64 {
        self.write_speed
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiskStats {
    sectors_read: u64,
    sectors_written: u64,

    read_ios: u64,
    write_ios: u64,
    discard_ios: u64,
    flush_ios: u64,
    io_total_time_ms: u64,

    read_ticks_weighted_ms: u64,
    write_ticks_weighted_ms: u64,
    discard_ticks_weighted_ms: u64,
    flush_ticks_weighted_ms: u64,

    read_time_ms: Instant,
}

pub struct LinuxDisksInfo {
    info: Vec<(DiskStats, LinuxDiskInfo)>,

    refresh_timestamp: Instant,
}

impl<'a> DisksInfoExt<'a> for LinuxDisksInfo {
    type S = LinuxDiskInfo;
    type Iter = LinuxDiskInfoIter<'a>;

    fn refresh_cache(&mut self) {
        use crate::{critical, warning};

        let now = Instant::now();
        if now.duration_since(self.refresh_timestamp) < MIN_DELTA_REFRESH {
            return;
        }
        self.refresh_timestamp = now;

        let mut prev_disks = std::mem::take(&mut self.info);

        let entries = match std::fs::read_dir("/sys/block") {
            Ok(e) => e,
            Err(e) => {
                critical!(
                    "Gatherer::DiskInfo",
                    "Failed to refresh disk information, failed to read disk entries: {}",
                    e
                );
                return;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warning!("Gatherer::DiskInfo", "Failed to read disk entry: {}", e);
                    continue;
                }
            };
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => {
                    warning!(
                        "Gatherer::DiskInfo",
                        "Failed to read disk entry file type: {}",
                        e
                    );
                    continue;
                }
            };

            let dir_name = if file_type.is_symlink() {
                let path = match entry.path().read_link() {
                    Err(e) => {
                        warning!(
                            "Gatherer::DiskInfo",
                            "Failed to read disk entry symlink: {}",
                            e
                        );
                        continue;
                    }
                    Ok(p) => {
                        let path = std::path::Path::new("/sys/block").join(p);
                        if !path.is_dir() {
                            continue;
                        }
                        path
                    }
                };

                match path.file_name() {
                    None => continue,
                    Some(dir_name) => dir_name.to_string_lossy().into_owned(),
                }
            } else if file_type.is_dir() {
                entry.file_name().to_string_lossy().into_owned()
            } else {
                continue;
            };

            let mut prev_disk_index = None;
            for i in 0..prev_disks.len() {
                if prev_disks[i].1.id.as_ref() == dir_name {
                    prev_disk_index = Some(i);
                    break;
                }
            }

            let stats = std::fs::read_to_string(format!("/sys/block/{}/stat", dir_name));

            let stats = match stats.as_ref() {
                Err(e) => {
                    warning!(
                        "MissionCenter::DiskInfo",
                        "Failed to read disk stat: {:?}",
                        e
                    );
                    ""
                }
                Ok(stats) => stats.trim(),
            };

            let mut read_ios = 0;
            let mut sectors_read = 0;
            let mut read_ticks_weighted_ms = 0;
            let mut write_ios = 0;
            let mut sectors_written = 0;
            let mut write_ticks_weighted_ms = 0;
            let mut io_total_time_ms: u64 = 0;
            let mut discard_ios = 0;
            let mut discard_ticks_weighted_ms = 0;
            let mut flush_ios = 0;
            let mut flush_ticks_weighted_ms = 0;

            const IDX_READ_IOS: usize = 0;
            const IDX_READ_SECTORS: usize = 2;
            const IDX_READ_TICKS: usize = 3;
            const IDX_WRITE_IOS: usize = 4;
            const IDX_WRITE_SECTORS: usize = 6;
            const IDX_WRITE_TICKS: usize = 7;
            const IDX_IO_TICKS: usize = 9;
            const IDX_DISCARD_IOS: usize = 11;
            const IDX_DISCARD_TICKS: usize = 14;
            const IDX_FLUSH_IOS: usize = 15;
            const IDX_FLUSH_TICKS: usize = 16;
            for (i, entry) in stats
                .split_whitespace()
                .enumerate()
                .map(|(i, v)| (i, v.trim()))
            {
                match i {
                    IDX_READ_IOS => read_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_READ_SECTORS => sectors_read = entry.parse::<u64>().unwrap_or(0),
                    IDX_READ_TICKS => read_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0),
                    IDX_WRITE_IOS => write_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_WRITE_SECTORS => sectors_written = entry.parse::<u64>().unwrap_or(0),
                    IDX_WRITE_TICKS => write_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0),
                    IDX_IO_TICKS => {
                        io_total_time_ms = entry.parse::<u64>().unwrap_or(0);
                    }
                    IDX_DISCARD_IOS => discard_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_DISCARD_TICKS => {
                        discard_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0)
                    }
                    IDX_FLUSH_IOS => flush_ios = entry.parse::<u64>().unwrap_or(0),
                    IDX_FLUSH_TICKS => {
                        flush_ticks_weighted_ms = entry.parse::<u64>().unwrap_or(0);
                        break;
                    }
                    _ => (),
                }
            }

            if let Some((mut disk_stat, mut info)) = prev_disk_index.map(|i| prev_disks.remove(i)) {
                let read_ticks_weighted_ms_prev =
                    if read_ticks_weighted_ms < disk_stat.read_ticks_weighted_ms {
                        read_ticks_weighted_ms
                    } else {
                        disk_stat.read_ticks_weighted_ms
                    };

                let write_ticks_weighted_ms_prev =
                    if write_ticks_weighted_ms < disk_stat.write_ticks_weighted_ms {
                        write_ticks_weighted_ms
                    } else {
                        disk_stat.write_ticks_weighted_ms
                    };

                let discard_ticks_weighted_ms_prev =
                    if discard_ticks_weighted_ms < disk_stat.discard_ticks_weighted_ms {
                        discard_ticks_weighted_ms
                    } else {
                        disk_stat.discard_ticks_weighted_ms
                    };

                let flush_ticks_weighted_ms_prev =
                    if flush_ticks_weighted_ms < disk_stat.flush_ticks_weighted_ms {
                        flush_ticks_weighted_ms
                    } else {
                        disk_stat.flush_ticks_weighted_ms
                    };

                let elapsed = disk_stat.read_time_ms.elapsed().as_secs_f32();

                let delta_read_ticks_weighted_ms =
                    read_ticks_weighted_ms - read_ticks_weighted_ms_prev;
                let delta_write_ticks_weighted_ms =
                    write_ticks_weighted_ms - write_ticks_weighted_ms_prev;
                let delta_discard_ticks_weighted_ms =
                    discard_ticks_weighted_ms - discard_ticks_weighted_ms_prev;
                let delta_flush_ticks_weighted_ms =
                    flush_ticks_weighted_ms - flush_ticks_weighted_ms_prev;
                let delta_ticks_weighted_ms = delta_read_ticks_weighted_ms
                    + delta_write_ticks_weighted_ms
                    + delta_discard_ticks_weighted_ms
                    + delta_flush_ticks_weighted_ms;

                // Arbitrary math is arbitrary
                let busy_percent = (delta_ticks_weighted_ms as f32 / (elapsed * 8.0)).min(100.);

                disk_stat.read_ticks_weighted_ms = read_ticks_weighted_ms;
                disk_stat.write_ticks_weighted_ms = write_ticks_weighted_ms;
                disk_stat.discard_ticks_weighted_ms = discard_ticks_weighted_ms;
                disk_stat.flush_ticks_weighted_ms = flush_ticks_weighted_ms;

                let io_time_ms_prev = if io_total_time_ms < disk_stat.io_total_time_ms {
                    io_total_time_ms
                } else {
                    disk_stat.io_total_time_ms
                };

                let read_ios_prev = if read_ios < disk_stat.read_ios {
                    read_ios
                } else {
                    disk_stat.read_ios
                };

                let write_ios_prev = if write_ios < disk_stat.write_ios {
                    write_ios
                } else {
                    disk_stat.write_ios
                };

                let discard_ios_prev = if discard_ios < disk_stat.discard_ios {
                    discard_ios
                } else {
                    disk_stat.discard_ios
                };

                let flush_ios_prev = if flush_ios < disk_stat.flush_ios {
                    flush_ios
                } else {
                    disk_stat.flush_ios
                };

                let delta_io_time_ms = io_total_time_ms - io_time_ms_prev;
                let delta_read_ios = read_ios - read_ios_prev;
                let delta_write_ios = write_ios - write_ios_prev;
                let delta_discard_ios = discard_ios - discard_ios_prev;
                let delta_flush_ios = flush_ios - flush_ios_prev;

                let delta_ios =
                    delta_read_ios + delta_write_ios + delta_discard_ios + delta_flush_ios;
                let response_time_ms = if delta_ios > 0 {
                    delta_io_time_ms as f32 / delta_ios as f32
                } else {
                    0.
                };

                disk_stat.read_ios = read_ios;
                disk_stat.write_ios = write_ios;
                disk_stat.discard_ios = discard_ios;
                disk_stat.flush_ios = flush_ios;
                disk_stat.io_total_time_ms = io_total_time_ms;

                let sectors_read_prev = if sectors_read < disk_stat.sectors_read {
                    sectors_read
                } else {
                    disk_stat.sectors_read
                };

                let sectors_written_prev = if sectors_written < disk_stat.sectors_written {
                    sectors_written
                } else {
                    disk_stat.sectors_written
                };

                let read_speed = ((sectors_read - sectors_read_prev) as f32 * 512.) / elapsed;
                let write_speed =
                    ((sectors_written - sectors_written_prev) as f32 * 512.) / elapsed;

                let read_speed = read_speed.round() as u64;
                let write_speed = write_speed.round() as u64;

                disk_stat.sectors_read = sectors_read;
                disk_stat.sectors_written = sectors_written;

                disk_stat.read_time_ms = Instant::now();

                info.busy_percent = busy_percent;
                info.response_time_ms = response_time_ms;
                info.read_speed = read_speed;
                info.write_speed = write_speed;

                self.info.push((disk_stat, info));
            } else {
                if dir_name.starts_with("loop")
                    || dir_name.starts_with("ram")
                    || dir_name.starts_with("zram")
                    || dir_name.starts_with("fd")
                    || dir_name.starts_with("md")
                    || dir_name.starts_with("dm")
                    || dir_name.starts_with("zd")
                {
                    continue;
                }

                let r#type = if let Ok(v) =
                    std::fs::read_to_string(format!("/sys/block/{}/queue/rotational", dir_name))
                {
                    let v = v.trim().parse::<u8>().ok().map_or(u8::MAX, |v| v);
                    if v == 0 {
                        if dir_name.starts_with("nvme") {
                            DiskType::NVMe
                        } else if dir_name.starts_with("mmc") {
                            Self::get_mmc_type(&dir_name)
                        } else {
                            DiskType::SSD
                        }
                    } else {
                        if dir_name.starts_with("sr") {
                            DiskType::Optical
                        } else {
                            match v {
                                1 => DiskType::HDD,
                                _ => DiskType::Unknown,
                            }
                        }
                    }
                } else {
                    DiskType::Unknown
                };

                let capacity = if let Ok(v) =
                    std::fs::read_to_string(format!("/sys/block/{}/size", dir_name))
                {
                    v.trim().parse::<u64>().ok().map_or(u64::MAX, |v| v * 512)
                } else {
                    u64::MAX
                };

                let fs_info = Self::filesystem_info(&dir_name);
                let (system_disk, formatted) = if let Some(v) = fs_info { v } else { (false, 0) };

                let vendor =
                    std::fs::read_to_string(format!("/sys/block/{}/device/vendor", dir_name))
                        .ok()
                        .unwrap_or("".to_string());

                let model =
                    std::fs::read_to_string(format!("/sys/block/{}/device/model", dir_name))
                        .ok()
                        .unwrap_or("".to_string());

                let model = Arc::<str>::from(format!("{} {}", vendor.trim(), model.trim()));

                self.info.push((
                    DiskStats {
                        sectors_read,
                        sectors_written,
                        read_ios,
                        write_ios,
                        discard_ios,
                        flush_ios,
                        io_total_time_ms,
                        read_ticks_weighted_ms,
                        write_ticks_weighted_ms,
                        discard_ticks_weighted_ms,
                        flush_ticks_weighted_ms,
                        read_time_ms: Instant::now(),
                    },
                    LinuxDiskInfo {
                        id: Arc::from(dir_name),
                        model,
                        r#type,
                        capacity,
                        formatted,
                        system_disk,

                        busy_percent: 0.,
                        response_time_ms: 0.,
                        read_speed: 0,
                        write_speed: 0,
                    },
                ));
            }
        }
    }

    fn info(&'a self) -> Self::Iter {
        LinuxDiskInfoIter::new(self.info.iter())
    }
}

impl LinuxDisksInfo {
    pub fn new() -> Self {
        Self {
            info: vec![],

            refresh_timestamp: *INITIAL_REFRESH_TS,
        }
    }

    fn filesystem_info(device_name: &str) -> Option<(bool, u64)> {
        use crate::{critical, warning};

        let entries = match std::fs::read_dir(format!("/sys/block/{}", device_name)) {
            Ok(e) => e,
            Err(e) => {
                critical!(
                    "Gatherer::DiskInfo",
                    "Failed to read filesystem information for '{}': {}",
                    device_name,
                    e
                );

                return None;
            }
        };

        let is_root_device = Self::mount_points(&device_name)
            .iter()
            .map(|v| v.as_str())
            .any(|v| v == "/");
        let mut formatted_size = 0_u64;
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warning!(
                        "Gatherer::DiskInfo",
                        "Failed to read some filesystem information for '{}': {}",
                        device_name,
                        e
                    );
                    continue;
                }
            };

            let part_name = entry.file_name();
            let part_name = part_name.to_string_lossy();
            if !part_name.starts_with(device_name) {
                continue;
            }
            std::fs::read_to_string(format!("/sys/block/{}/{}/size", &device_name, part_name))
                .ok()
                .map(|v| v.trim().parse::<u64>().ok().map_or(0, |v| v * 512))
                .map(|v| {
                    formatted_size += v;
                });
        }

        Some((is_root_device, formatted_size))
    }

    fn mount_points(device_name: &str) -> Vec<String> {
        use crate::critical;

        let mut cmd = std::process::Command::new("lsblk");
        cmd.arg("-o").arg("NAME,MOUNTPOINTS").arg("--json");

        let lsblk_out = if let Ok(output) = cmd.output() {
            if output.stderr.len() > 0 {
                critical!(
                    "Gatherer::DiskInfo",
                    "Failed to refresh block device information, host command execution failed: {}",
                    std::str::from_utf8(output.stderr.as_slice()).unwrap_or("Unknown error")
                );
                return vec![];
            }

            output.stdout
        } else {
            critical!(
                "Gatherer::DiskInfo",
                "Failed to refresh block device information, host command execution failed"
            );
            return vec![];
        };

        let mut lsblk_out = match serde_json::from_slice::<LSBLKOutput>(lsblk_out.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                critical!(
                    "MissionCenter::DiskInfo",
                    "Failed to refresh block device information, host command execution failed: {}",
                    e
                );
                return vec![];
            }
        };

        let mut mount_points = vec![];
        for block_device in lsblk_out
            .blockdevices
            .iter_mut()
            .filter_map(|bd| bd.as_mut())
        {
            let block_device = core::mem::take(block_device);
            if block_device.name != device_name {
                continue;
            }

            let children = match block_device.children {
                None => break,
                Some(c) => c,
            };

            fn find_mount_points(
                mut block_devices: Vec<Option<LSBLKBlockDevice>>,
                mount_points: &mut Vec<String>,
            ) {
                for block_device in block_devices.iter_mut().filter_map(|bd| bd.as_mut()) {
                    let mut block_device = core::mem::take(block_device);

                    for mountpoint in block_device
                        .mountpoints
                        .iter_mut()
                        .filter_map(|mp| mp.as_mut())
                    {
                        mount_points.push(core::mem::take(mountpoint));
                    }

                    if let Some(children) = block_device.children {
                        find_mount_points(children, mount_points);
                    }
                }
            }

            find_mount_points(children, &mut mount_points);
            break;
        }

        mount_points
    }

    fn get_mmc_type(dir_name: &String) -> DiskType {
        let Some(hwmon_idx) = dir_name[6..].parse::<u64>().ok() else {
            return DiskType::Unknown;
        };

        let globs = match glob(&format!(
            "/sys/class/mmc_host/mmc{}/mmc{}*/type",
            hwmon_idx, hwmon_idx
        )) {
            Ok(globs) => globs,
            Err(e) => {
                warning!("Gatherer::DiskInfo", "Failed to read mmc type entry: {}", e);
                return DiskType::Unknown;
            }
        };

        let mut res = DiskType::Unknown;
        for entry in globs {
            res = match entry {
                Ok(path) => match std::fs::read_to_string(&path).ok() {
                    Some(typ) => match typ.trim() {
                        "SD" => DiskType::SD,
                        "MMC" => DiskType::eMMC,
                        _ => {
                            critical!("Gatherer::DiskInfo", "Unknown mmc type: '{}'", typ);
                            continue;
                        }
                    },
                    _ => {
                        critical!(
                            "Gatherer::DiskInfo",
                            "Could not read mmc type: {}",
                            path.display()
                        );
                        continue;
                    }
                },
                _ => {
                    continue;
                }
            };
        }
        res
    }
}
