/* sys_info_v2/dbus_interface/cpu_dynamic_info.rs
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

use std::sync::Arc;

use dbus::{arg::*, strings::*};

#[derive(Debug, Default, Clone)]
pub struct CpuDynamicInfo {
    pub overall_utilization_percent: f32,
    pub overall_kernel_utilization_percent: f32,
    pub per_logical_cpu_utilization_percent: Vec<f32>,
    pub per_logical_cpu_kernel_utilization_percent: Vec<f32>,
    pub current_frequency_mhz: u64,
    pub temperature: Option<f32>,
    pub process_count: u64,
    pub thread_count: u64,
    pub handle_count: u64,
    pub uptime_seconds: u64,
    pub cpufreq_driver: Option<Arc<str>>,
    pub cpufreq_governor: Option<Arc<str>>,
    pub energy_performance_preference: Option<Arc<str>>,
}

impl Arg for CpuDynamicInfo {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ddadadtdtttt)")
    }
}

impl ReadAll for CpuDynamicInfo {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for CpuDynamicInfo {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut this = CpuDynamicInfo {
            overall_utilization_percent: 0.0,
            overall_kernel_utilization_percent: 0.0,
            per_logical_cpu_utilization_percent: vec![],
            per_logical_cpu_kernel_utilization_percent: vec![],
            current_frequency_mhz: 0,
            temperature: None,
            process_count: 0,
            thread_count: 0,
            handle_count: 0,
            uptime_seconds: 0,
            cpufreq_driver: None,
            cpufreq_governor: None,
            energy_performance_preference: None,
        };

        let dynamic_info = match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None",
                );
                return None;
            }
            Some(id) => id,
        };

        let mut dynamic_info = match dynamic_info.as_iter() {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                );
                return None;
            }
            Some(i) => i,
        };
        let dynamic_info = dynamic_info.as_mut();

        this.overall_utilization_percent = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '0: d', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_f64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '0: d', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(u) => u as _,
            },
        };

        this.overall_kernel_utilization_percent = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '1: d', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_f64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '1: d', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(u) => u as _,
            },
        };

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '2: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '2: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(u) => {
                    for v in u {
                        this.per_logical_cpu_utilization_percent
                            .push(v.as_f64().unwrap_or(0.) as f32);
                    }
                }
            },
        }

        match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '4: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '4: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(u) => {
                    for v in u {
                        this.per_logical_cpu_kernel_utilization_percent
                            .push(v.as_f64().unwrap_or(0.) as f32);
                    }
                }
            },
        }

        this.current_frequency_mhz = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '6: t', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '6: t', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(f) => f,
            },
        };

        this.temperature = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '7: d', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_f64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '7: d', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(u) => {
                    if u == 0. {
                        None
                    } else {
                        Some(u as f32)
                    }
                }
            },
        };

        this.process_count = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '8: t', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '8: t', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(pc) => pc,
            },
        };

        this.thread_count = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '9: t', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '9: t', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(tc) => tc,
            },
        };

        this.handle_count = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '10: t', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '10: t', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(hc) => hc,
            },
        };

        this.uptime_seconds = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuDynamicInfo: Expected '11: t', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_u64() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuDynamicInfo: Expected '11: t', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(us) => us,
            },
        };
        this.cpufreq_driver = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuStaticInfo: Expected '12: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuStaticInfo: Expected '12: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(ivs) => match ivs {
                    "" => None,
                    _ => Some(Arc::from(ivs)),
                },
            },
        };

        this.cpufreq_governor = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuStaticInfo: Expected '13: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuStaticInfo: Expected '13: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(ivs) => match ivs {
                    "" => None,
                    _ => Some(Arc::from(ivs)),
                },
            },
        };

        this.energy_performance_preference = match Iterator::next(dynamic_info) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get CpuStaticInfo: Expected '14: s', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_str() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get CpuStaticInfo: Expected '14: s', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(ivs) => match ivs {
                    "" => None,
                    _ => Some(Arc::from(ivs)),
                },
            },
        };

        Some(this)
    }
}
