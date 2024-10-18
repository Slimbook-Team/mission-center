/* sys_info_v2/dbus_interface/gpu_static_info.rs
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

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OpenGLApi {
    OpenGL,
    OpenGLES,
    Invalid = 255,
}

#[derive(Debug, Copy, Clone)]
pub struct OpenGLApiVersion {
    pub major: u8,
    pub minor: u8,
    pub api: OpenGLApi,
}

#[derive(Default, Debug, Copy, Clone)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

#[derive(Debug, Clone)]
pub struct GpuStaticInfo {
    pub id: Arc<str>,
    pub device_name: Arc<str>,
    pub vendor_id: u16,
    pub device_id: u16,
    pub total_memory: u64,
    pub total_gtt: u64,
    pub opengl_version: Option<OpenGLApiVersion>,
    pub vulkan_version: Option<ApiVersion>,
    pub metal_version: Option<ApiVersion>,
    pub direct3d_version: Option<ApiVersion>,
    pub pcie_gen: u8,
    pub pcie_lanes: u8,
}

impl Default for GpuStaticInfo {
    fn default() -> Self {
        let empty = Arc::<str>::from("");
        GpuStaticInfo {
            id: empty.clone(),
            device_name: empty,
            vendor_id: 0,
            device_id: 0,
            total_memory: 0,
            total_gtt: 0,
            opengl_version: None,
            vulkan_version: None,
            metal_version: None,
            direct3d_version: None,
            pcie_gen: 0,
            pcie_lanes: 0,
        }
    }
}

pub struct GpuStaticInfoVec(pub Vec<GpuStaticInfo>);

impl From<GpuStaticInfoVec> for Vec<GpuStaticInfo> {
    fn from(v: GpuStaticInfoVec) -> Self {
        v.0
    }
}

impl From<Vec<GpuStaticInfo>> for GpuStaticInfoVec {
    fn from(v: Vec<GpuStaticInfo>) -> Self {
        GpuStaticInfoVec(v)
    }
}

impl Arg for GpuStaticInfoVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        dbus::Signature::from("a(ssqqt(yyy)(qqq)(qqq)(qqq)yy)")
    }
}

impl ReadAll for GpuStaticInfoVec {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for GpuStaticInfoVec {
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
                    for static_info in arr {
                        let empty_string = Arc::<str>::from("");

                        let mut info = GpuStaticInfo {
                            id: empty_string.clone(),
                            device_name: empty_string,
                            vendor_id: 0,
                            device_id: 0,
                            total_memory: 0,
                            total_gtt: 0,
                            opengl_version: None,
                            vulkan_version: None,
                            metal_version: None,
                            direct3d_version: None,
                            pcie_gen: 0,
                            pcie_lanes: 0,
                        };

                        let mut static_info = match static_info.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '0: STRUCT', got None, failed to iterate over fields",
                                );
                                return None;
                            }
                            Some(i) => i,
                        };
                        let static_info = static_info.as_mut();

                        info.id = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '0: s', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(id) => Arc::<str>::from(id),
                            },
                        };

                        info.device_name = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '1: s', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '1: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(id) => Arc::<str>::from(id),
                            },
                        };

                        info.vendor_id = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '2: q', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '2: q', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(vendor_id) => vendor_id as _,
                            },
                        };

                        info.device_id = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '3: q', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '3: q', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(device_id) => device_id as _,
                            },
                        };

                        info.total_memory = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '4: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '4: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(total_memory) => total_memory,
                            },
                        };

                        info.total_gtt = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuDynamicInfo: Expected '5: t', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuDynamicInfo: Expected '5: t', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(um) => um as _,
                            },
                        };
                        info.opengl_version = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '6: STRUCT', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_iter() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '6: STRUCT', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(mut it) => {
                                    let major = if let Some(major) = Iterator::next(it.as_mut()) {
                                        major.as_u64().unwrap_or(0)
                                    } else {
                                        g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-0: y', got None",
                                        );

                                        0
                                    };

                                    let minor = if let Some(minor) = Iterator::next(it.as_mut()) {
                                        minor.as_u64().unwrap_or(0)
                                    } else {
                                        g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-1: y', got None",
                                        );

                                        0
                                    };

                                    let gl_api = if let Some(minor) = Iterator::next(it.as_mut()) {
                                        match minor.as_u64().unwrap_or(OpenGLApi::Invalid as u64) {
                                            0 => OpenGLApi::OpenGL,
                                            1 => OpenGLApi::OpenGLES,
                                            _ => OpenGLApi::Invalid,
                                        }
                                    } else {
                                        g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo(OpenGLVersion): Expected '6-2: y', got None",
                                        );

                                        OpenGLApi::Invalid
                                    };

                                    if major == 0 || minor == 0 || gl_api == OpenGLApi::Invalid {
                                        None
                                    } else {
                                        Some(OpenGLApiVersion {
                                            major: major as u8,
                                            minor: minor as u8,
                                            api: gl_api,
                                        })
                                    }
                                }
                            },
                        };

                        let mut api_versions = [None; 3];
                        for i in 0..3 {
                            api_versions[i] = match Iterator::next(static_info) {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '{}: STRUCT', got None",
                                        i + 7,
                                    );
                                    return None;
                                }
                                Some(id) => match id.as_iter() {
                                    None => {
                                        g_critical!(
                                            "MissionCenter::GathererDBusProxy",
                                            "Failed to get GpuStaticInfo: Expected '{}: STRUCT', got {:?}",
                                            i + 7,
                                            id.arg_type(),
                                        );
                                        return None;
                                    }
                                    Some(mut it) => {
                                        let major = if let Some(major) = Iterator::next(it.as_mut())
                                        {
                                            major.as_u64().unwrap_or(0)
                                        } else {
                                            g_critical!(
                                                "MissionCenter::GathererDBusProxy",
                                                "Failed to get GpuStaticInfo(ApiVersion): Expected '{}-0: y', got None",
                                                i + 7
                                            );

                                            0
                                        };

                                        let minor = if let Some(minor) = Iterator::next(it.as_mut())
                                        {
                                            minor.as_u64().unwrap_or(0)
                                        } else {
                                            g_critical!(
                                                "MissionCenter::GathererDBusProxy",
                                                "Failed to get GpuStaticInfo(ApiVersion): Expected '{}-1: y', got None",
                                                i + 7
                                            );

                                            0
                                        };

                                        let patch = if let Some(patch) = Iterator::next(it.as_mut())
                                        {
                                            patch.as_u64().unwrap_or(0)
                                        } else {
                                            g_critical!(
                                                "MissionCenter::GathererDBusProxy",
                                                "Failed to get GpuStaticInfo(ApiVersion): Expected '{}-1: y', got None",
                                                i + 7
                                            );

                                            0
                                        };

                                        if major == 0 {
                                            None
                                        } else {
                                            Some(ApiVersion {
                                                major: major as u16,
                                                minor: minor as u16,
                                                patch: patch as u16,
                                            })
                                        }
                                    }
                                },
                            }
                        }

                        info.vulkan_version = api_versions[0];
                        info.metal_version = api_versions[1];
                        info.direct3d_version = api_versions[2];

                        info.pcie_gen = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '10: y', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '10: y', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(pcie_gen) => pcie_gen as u8,
                            },
                        };

                        info.pcie_lanes = match Iterator::next(static_info) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get GpuStaticInfo: Expected '11: y', got None",
                                );
                                return None;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get GpuStaticInfo: Expected '11: y', got {:?}",
                                        arg.arg_type(),
                                    );
                                    return None;
                                }
                                Some(pcie_lanes) => pcie_lanes as u8,
                            },
                        };

                        result.push(info);
                    }

                    Some(result.into())
                }
            },
        }
    }
}
