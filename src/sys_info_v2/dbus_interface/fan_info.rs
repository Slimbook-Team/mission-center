/* sys_info_v2/gatherer/src/platform/fan_info.rs
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
use dbus::arg::{Arg, ArgType, Get, Iter, ReadAll, RefArg, TypeMismatchError};
use dbus::Signature;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FanInfo {
    pub fan_label: Arc<str>,
    pub temp_name: Arc<str>,
    pub temp_amount: i64,
    pub rpm: u64,
    pub percent_vroomimg: f32,

    pub fan_index: u64,
    pub hwmon_index: u64,

    pub max_speed: u64,
}

impl Default for FanInfo {
    fn default() -> Self {
        Self {
            fan_label: Arc::from(""),
            temp_name: Arc::from(""),
            temp_amount: 0,
            rpm: 0,
            percent_vroomimg: 0.0,

            fan_index: 0,
            hwmon_index: 0,

            max_speed: 0,
        }
    }
}

impl Eq for FanInfo {}

impl PartialEq<Self> for FanInfo {
    fn eq(&self, other: &Self) -> bool {
        self.fan_index == other.fan_index && self.hwmon_index == other.hwmon_index
    }
}

impl PartialOrd<Self> for FanInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(if self.hwmon_index == other.hwmon_index {
            self.fan_index.cmp(&other.fan_index)
        } else {
            self.hwmon_index.cmp(&other.hwmon_index)
        })
    }
}

impl Ord for FanInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.hwmon_index == other.hwmon_index {
            self.fan_index.cmp(&other.fan_index)
        } else {
            self.hwmon_index.cmp(&other.hwmon_index)
        }
    }
}

pub struct FanInfoVec(pub Vec<FanInfo>);

impl From<FanInfoVec> for Vec<FanInfo> {
    fn from(v: FanInfoVec) -> Self {
        v.0
    }
}

impl Arg for FanInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("a(ssxtdttt)")
    }
}

impl ReadAll for FanInfoVec {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for FanInfoVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut result = vec![];

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Vec<FanInfo>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<FanInfo>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for i in arr {
                        let mut this = FanInfo::default();

                        let mut i = match i.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '0: STRUCT', got None",
                                );
                                continue;
                            }
                            Some(i) => i,
                        };
                        let fan_info = i.as_mut();

                        this.fan_label = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '0: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(n) => Arc::<str>::from(n),
                            },
                        };

                        this.temp_name = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '1: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '1: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(m) => Arc::<str>::from(m),
                            },
                        };

                        this.temp_amount = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '2: y', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_i64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '2: y', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(s) => s,
                            },
                        };

                        this.rpm = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '3: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '3: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(c) => c,
                            },
                        };

                        this.percent_vroomimg = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '4: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '4: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(f) => f as f32,
                            },
                        };

                        this.fan_index = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '5: b', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '5: b', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(i) => i,
                            },
                        };

                        this.hwmon_index = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '6: d', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '6: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(bp) => bp,
                            },
                        };

                        this.max_speed = match Iterator::next(fan_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get FanInfo: Expected '7: d', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get FanInfo: Expected '7: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(bp) => bp,
                            },
                        };

                        result.push(this);
                    }
                }
            },
        }

        Some(FanInfoVec(result))
    }
}
