/* sys_info_v2/dbus_interface/gpu_dynamic_info.rs
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
use gtk::glib::g_critical;

#[derive(Debug, Clone)]
pub struct GpuDynamicInfo {
    pub id: Arc<str>,
    pub temp_celsius: u32,
    pub fan_speed_percent: u32,
    pub util_percent: u32,
    pub power_draw_watts: f32,
    pub power_draw_max_watts: f32,
    pub clock_speed_mhz: u32,
    pub clock_speed_max_mhz: u32,
    pub mem_speed_mhz: u32,
    pub mem_speed_max_mhz: u32,
    pub free_memory: u64,
    pub used_memory: u64,
    pub used_gtt: u64,
    pub encoder_percent: u32,
    pub decoder_percent: u32,
}

impl Default for GpuDynamicInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            temp_celsius: 0,
            fan_speed_percent: 0,
            util_percent: 0,
            power_draw_watts: 0.0,
            power_draw_max_watts: 0.0,
            clock_speed_mhz: 0,
            clock_speed_max_mhz: 0,
            mem_speed_mhz: 0,
            mem_speed_max_mhz: 0,
            free_memory: 0,
            used_memory: 0,
            used_gtt: 0,
            encoder_percent: 0,
            decoder_percent: 0,
        }
    }
}

pub struct GpuDynamicInfoVec(pub Vec<GpuDynamicInfo>);

impl From<GpuDynamicInfoVec> for Vec<GpuDynamicInfo> {
    fn from(v: GpuDynamicInfoVec) -> Self {
        v.0
    }
}

impl From<Vec<GpuDynamicInfo>> for GpuDynamicInfoVec {
    fn from(v: Vec<GpuDynamicInfo>) -> Self {
        GpuDynamicInfoVec(v)
    }
}

impl Arg for GpuDynamicInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        dbus::Signature::from("a(suuudduuuuttuu)")
    }
}

impl ReadAll for GpuDynamicInfoVec {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for GpuDynamicInfoVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
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
                    for dynamic_info in arr {
                        let mut this = GpuDynamicInfo {
                            id: Arc::from(""),
                            temp_celsius: 0,
                            fan_speed_percent: 0,
                            util_percent: 0,
                            power_draw_watts: 0.0,
                            power_draw_max_watts: 0.0,
                            clock_speed_mhz: 0,
                            clock_speed_max_mhz: 0,
                            mem_speed_mhz: 0,
                            mem_speed_max_mhz: 0,
                            free_memory: 0,
                            used_memory: 0,
                            used_gtt: 0,
                            encoder_percent: 0,
                            decoder_percent: 0,
                        };

                        let mut dynamic_info = match dynamic_info.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                                );
                                return None;
                            }
                            Some(i) => i,
                        };
                        let dynamic_info = dynamic_info.as_mut();

                        this.id = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '0: s', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(id) => Arc::<str>::from(id),
                            },
                        };

                        this.temp_celsius = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '1: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '1: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(temp) => temp as _,
                            },
                        };

                        this.fan_speed_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '2: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '2: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(fs) => fs as _,
                            },
                        };

                        this.util_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '3: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '3: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(up) => up as _,
                            },
                        };

                        this.power_draw_watts = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '4: d', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '4: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(pd) => pd as _,
                            },
                        };

                        this.power_draw_max_watts = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '5: d', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_f64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '5: d', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(pdm) => pdm as _,
                            },
                        };

                        this.clock_speed_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '6: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '6: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(cs) => cs as _,
                            },
                        };

                        this.clock_speed_max_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '7: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '7: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(csm) => csm as _,
                            },
                        };

                        this.mem_speed_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '8: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '8: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(ms) => ms as _,
                            },
                        };

                        this.mem_speed_max_mhz = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '9: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '9: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(msm) => msm as _,
                            },
                        };

                        this.free_memory = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '10: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '10: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(fm) => fm as _,
                            },
                        };

                        this.used_memory = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '11: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '11: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(um) => um as _,
                            },
                        };

                        this.used_gtt = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '12: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '12: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(fm) => fm as _,
                            },
                        };

                        this.encoder_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '13: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '13: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(ep) => ep as _,
                            },
                        };

                        this.decoder_percent = match Iterator::next(dynamic_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '14: u', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '14: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(dp) => dp as _,
                            },
                        };

                        result.push(this);
                    }

                    Some(result.into())
                }
            },
        }
    }
}
