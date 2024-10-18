/* sys_info_v2/dbus_interface/disk_static_info.rs
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

use dbus::{arg::*, strings::*};

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskType {
    Unknown = 0,
    HDD,
    SSD,
    NVMe,
    eMMC,
    SD,
    iSCSI,
    Optical,
}

impl Default for DiskType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
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

impl Default for DiskInfo {
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

impl Eq for DiskInfo {}

impl PartialEq<Self> for DiskInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id.as_ref() == other.id.as_ref()
    }
}

impl PartialOrd<Self> for DiskInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.as_ref().cmp(other.id.as_ref()))
    }
}

impl Ord for DiskInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.as_ref().cmp(other.id.as_ref())
    }
}

pub struct DiskInfoVec(pub Vec<DiskInfo>);

impl From<DiskInfoVec> for Vec<DiskInfo> {
    fn from(v: DiskInfoVec) -> Self {
        v.0
    }
}

impl Arg for DiskInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("a(ssyttbddtt)")
    }
}

impl ReadAll for DiskInfoVec {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for DiskInfoVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut result = vec![];

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Vec<DiskInfo>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<DiskInfo>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for i in arr {
                        let mut this = DiskInfo::default();

                        let mut i = match i.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '0: STRUCT', got None",
                                );
                                continue;
                            }
                            Some(i) => i,
                        };
                        let disk_info = i.as_mut();

                        this.id = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '0: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(n) => Arc::<str>::from(n),
                            },
                        };

                        this.model = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '1: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '1: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(m) => Arc::<str>::from(m),
                            },
                        };

                        this.r#type = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '2: y', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '2: y', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(t) => match t {
                                    1 => DiskType::HDD,
                                    2 => DiskType::SSD,
                                    3 => DiskType::NVMe,
                                    4 => DiskType::eMMC,
                                    5 => DiskType::SD,
                                    6 => DiskType::iSCSI,
                                    7 => DiskType::Optical,
                                    _ => DiskType::Unknown,
                                },
                            },
                        };

                        this.capacity = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '3: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '3: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(c) => c,
                            },
                        };

                        this.formatted = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '4: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '4: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(f) => f,
                            },
                        };

                        this.system_disk = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '5: b', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '5: b', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(ivm) => match ivm {
                                    1 => true,
                                    _ => false,
                                },
                            },
                        };

                        this.busy_percent = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '6: d', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '6: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(bp) => bp as _,
                            },
                        };

                        this.response_time_ms = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '7: d', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '7: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(rt) => rt as _,
                            },
                        };

                        this.read_speed = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '8: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '8: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(rs) => rs,
                            },
                        };

                        this.write_speed = match Iterator::next(disk_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get DiskInfo: Expected '9: t', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get DiskInfo: Expected '9: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(ws) => ws,
                            },
                        };

                        result.push(this);
                    }
                }
            },
        }

        Some(DiskInfoVec(result))
    }
}
