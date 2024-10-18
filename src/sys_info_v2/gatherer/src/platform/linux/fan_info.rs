/* sys_info_v2/gatherer/src/platform/linux/fan_info.rs
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
use std::sync::Arc;
use std::time::Instant;

use convert_case::{Case, Casing};
use glob::glob;

use super::{INITIAL_REFRESH_TS, MIN_DELTA_REFRESH};
use crate::platform::fan_info::{FanInfoExt, FansInfoExt};

#[derive(Debug, Clone, PartialEq)]
pub struct LinuxFanInfo {
    pub fan_label: Arc<str>,
    pub temp_name: Arc<str>,
    pub temp_amount: i64,
    pub rpm: u64,
    pub percent_vroomimg: f32,

    pub fan_index: u64,
    pub hwmon_index: u64,

    pub max_speed: u64,
}

impl Default for LinuxFanInfo {
    fn default() -> Self {
        Self {
            fan_label: Arc::from(""),
            temp_name: Arc::from(""),
            temp_amount: 0,
            rpm: 0,
            percent_vroomimg: 0.,

            fan_index: 0,
            hwmon_index: 0,

            max_speed: 0,
        }
    }
}

pub struct LinuxFanInfoIter<'a>(
    pub std::iter::Map<std::slice::Iter<'a, LinuxFanInfo>, fn(&'a LinuxFanInfo) -> &'a LinuxFanInfo>,
);

impl<'a> LinuxFanInfoIter<'a> {
    pub fn new(iter: std::slice::Iter<'a, LinuxFanInfo>) -> Self {
        Self(iter.map(|di| di))
    }
}

impl<'a> Iterator for LinuxFanInfoIter<'a> {
    type Item = &'a LinuxFanInfo;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl FanInfoExt for LinuxFanInfo {
    fn fan_label(&self) -> &str {
        &self.fan_label
    }

    fn temp_name(&self) -> &str {
        &self.temp_name
    }

    fn temp_amount(&self) -> i64 {
        self.temp_amount
    }

    fn rpm(&self) -> u64 {
        self.rpm
    }

    fn percent_vroomimg(&self) -> f32 {
        self.percent_vroomimg
    }

    fn fan_index(&self) -> u64 {
        self.fan_index
    }

    fn hwmon_index(&self) -> u64 {
        self.hwmon_index
    }

    fn max_speed(&self) -> u64 {
        self.max_speed
    }
}

pub struct LinuxFansInfo {
    info: Vec<LinuxFanInfo>,

    refresh_timestamp: Instant,
}

impl<'a> FansInfoExt<'a> for LinuxFansInfo {
    type S = LinuxFanInfo;
    type Iter = LinuxFanInfoIter<'a>;

    fn refresh_cache(&mut self) {
        use crate::warning;

        let now = Instant::now();
        if now.duration_since(self.refresh_timestamp) < MIN_DELTA_REFRESH {
            return;
        }
        self.refresh_timestamp = now;

        self.info.clear();

        match glob("/sys/class/hwmon/hwmon*/fan*_input") {
            Ok(globs) => {
                for entry in globs {
                    match entry {
                        Ok(path) => {
                            // read the first glob result for hwmon location
                            let parent_dir = path.parent().unwrap();
                            let parent_dir_str = path.parent().unwrap().to_str().unwrap();
                            let hwmon_idx =
                                if let Some(hwmon_dir) = parent_dir.file_name().unwrap().to_str() {
                                    hwmon_dir[5..].parse::<u64>().ok().unwrap_or(u64::MAX)
                                } else {
                                    continue;
                                };

                            // read the second glob result for fan index
                            let findex = if let Some(hwmon_instance_dir) =
                                path.file_name().unwrap().to_str()
                            {
                                hwmon_instance_dir[3..4]
                                    .parse::<u64>()
                                    .ok()
                                    .unwrap_or(u64::MAX)
                            } else {
                                continue;
                            };

                            let fan_label = if let Ok(label) = std::fs::read_to_string(format!(
                                "{}/fan{}_label",
                                parent_dir_str, findex
                            )) {
                                Arc::from(label.trim().to_case(Case::Title))
                            } else {
                                continue; // ignore missing label to prevent duplication
                            };

                            let temp_label = if let Ok(label) = std::fs::read_to_string(format!(
                                "{}/temp{}_label",
                                parent_dir_str, findex
                            )) {
                                Arc::from(label.trim().to_case(Case::Title))
                            } else {
                                // report no label as empty string
                                Arc::from("")
                            };

                            let percent_vrooming = if let Ok(v) =
                                std::fs::read_to_string(format!("{}/pwm{}", parent_dir_str, findex))
                            {
                                v.trim()
                                    .parse::<u64>()
                                    .ok()
                                    .map_or(-1.0, |v| v as f32 / 255.)
                            } else {
                                -1.0
                            };

                            let rpm = if let Ok(v) = std::fs::read_to_string(format!(
                                "{}/fan{}_input",
                                parent_dir_str, findex
                            )) {
                                v.trim().parse::<u64>().ok().unwrap_or(u64::MAX)
                            } else {
                                u64::MAX
                            };

                            let temp = if let Ok(v) = std::fs::read_to_string(format!(
                                "{}/temp{}_input",
                                parent_dir_str, findex
                            )) {
                                v.trim().parse::<i64>().ok().unwrap_or(i64::MIN)
                            } else {
                                i64::MIN
                            };

                            let max_speed = if let Ok(v) = std::fs::read_to_string(format!(
                                "{}/fan{}_max",
                                parent_dir_str, findex
                            )) {
                                v.trim().parse::<u64>().ok().unwrap_or(0)
                            } else {
                                0
                            };

                            self.info.push(LinuxFanInfo {
                                fan_label,
                                temp_name: temp_label,
                                temp_amount: temp,
                                rpm,
                                percent_vroomimg: percent_vrooming,

                                fan_index: findex,
                                hwmon_index: hwmon_idx,

                                max_speed,
                            })
                        }
                        Err(e) => {
                            warning!("Gatherer::FanInfo", "Failed to read hwmon entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warning!("Gatherer::FanInfo", "Failed to read hwmon entry: {}", e);
            }
        };
    }

    fn info(&'a self) -> Self::Iter {
        LinuxFanInfoIter::new(self.info.iter())
    }
}

impl LinuxFansInfo {
    pub fn new() -> Self {
        Self {
            info: vec![],

            refresh_timestamp: *INITIAL_REFRESH_TS,
        }
    }
}
