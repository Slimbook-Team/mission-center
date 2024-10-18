/* sys_info_v2/gatherer/src/platform/gpu_info.rs
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
use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum OpenGLApi {
    OpenGL,
    OpenGLES,
    Invalid = 255,
}

impl Default for OpenGLApi {
    fn default() -> Self {
        Self::Invalid
    }
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct OpenGLApiVersion {
    pub major: u8,
    pub minor: u8,
    pub api: OpenGLApi,
}

impl Arg for OpenGLApiVersion {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(yyy)")
    }
}

impl Append for OpenGLApiVersion {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append((self.major, self.minor, self.api as u8));
    }
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Arg for ApiVersion {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(qqq)")
    }
}

impl Append for ApiVersion {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append((self.major, self.minor, self.patch));
    }
}

/// Describes the static (unchanging) information about a GPU
pub trait GpuStaticInfoExt: Default + Clone + Append + Arg {
    /// Platform specific unique identifier for a GPU
    ///
    /// Implementations must ensure that two separate GPUs never have the same id, even if they are
    /// identical models
    fn id(&self) -> &str;

    /// The human-readable name of the GPU
    fn device_name(&self) -> &str;

    /// The PCI vendor identifier
    fn vendor_id(&self) -> u16;

    /// The PCI device identifier
    fn device_id(&self) -> u16;

    /// The total amount of GPU memory available to the device
    ///
    /// It is platform/driver specific if this value includes any memory shared with system RAM
    fn total_memory(&self) -> u64;

    /// The total amount of gtt/gart available
    fn total_gtt(&self) -> u64;

    /// The version of OpenGL that the GPU supports
    ///
    /// If the platform does not provide OpenGL support it should return None
    fn opengl_version(&self) -> Option<&OpenGLApiVersion>;

    /// The version of Vulkan that the GPU supports
    ///
    /// If the platform does not provide Vulkan support it should return None
    fn vulkan_version(&self) -> Option<&ApiVersion>;

    /// The version of Metal that the GPU supports
    ///
    /// If the platform does not provide Metal support it should return None
    fn metal_version(&self) -> Option<&ApiVersion>;

    /// The version of Direct3D that the GPU supports
    ///
    /// If the platform does not provide Direct3D support it should return None
    fn direct3d_version(&self) -> Option<&ApiVersion>;

    /// The PCI express lane generation that the GPU is mounted on
    fn pcie_gen(&self) -> u8;

    /// The number of PCI express lanes in use by the GPU
    fn pcie_lanes(&self) -> u8;
}

impl Arg for crate::platform::GpuStaticInfo {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(ssqqtt(yyy)(qqq)(qqq)(qqq)yy)")
    }
}

impl Append for crate::platform::GpuStaticInfo {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append((
            self.id(),
            self.device_name(),
            self.vendor_id(),
            self.device_id(),
            self.total_memory(),
            self.total_gtt(),
            self.opengl_version().map(|v| *v).unwrap_or_default(),
            self.vulkan_version().map(|v| *v).unwrap_or_default(),
            self.metal_version().map(|v| *v).unwrap_or_default(),
            self.direct3d_version().map(|v| *v).unwrap_or_default(),
            self.pcie_gen(),
            self.pcie_lanes(),
        ));
    }
}

/// Describes GPU information that changes over time
pub trait GpuDynamicInfoExt: Default + Clone + Append + Arg {
    /// Platform specific unique identifier for a GPU
    ///
    /// Implementations must ensure that two separate GPUs never have the same id, even if they are
    /// identical models
    /// Note: This value is actually static but is part of this interface to help users of the type
    /// easily match these data points to a GPU
    fn id(&self) -> &str;

    /// The GPU temperature in degrees Celsius
    ///
    /// While all modern chips report several temperatures from the GPU card, it is expected that
    /// implementations provide the most user relevant value here
    fn temp_celsius(&self) -> u32;

    /// The speed of the fan represented as a percentage from it's maximum speed
    fn fan_speed_percent(&self) -> u32;

    /// Load of the graphics pipeline
    fn util_percent(&self) -> u32;

    /// The power draw in watts
    fn power_draw_watts(&self) -> f32;

    /// The maximum power that the GPU is allowed to draw
    fn power_draw_max_watts(&self) -> f32;

    /// The current GPU core clock frequency
    fn clock_speed_mhz(&self) -> u32;

    /// The maximum allowed GPU core clock frequency
    fn clock_speed_max_mhz(&self) -> u32;

    /// The current speed of the on-board memory
    fn mem_speed_mhz(&self) -> u32;

    /// The maximum speed of the on-board memory
    fn mem_speed_max_mhz(&self) -> u32;

    /// The amount of memory available
    fn free_memory(&self) -> u64;

    /// The memory that is currently being used
    fn used_memory(&self) -> u64;

    /// The amount of gtt/gart available
    fn used_gtt(&self) -> u64;
    /// Utilization percent of the encoding pipeline of the GPU
    fn encoder_percent(&self) -> u32;

    /// Utilization percent of the decoding pipeline of the GPU
    fn decoder_percent(&self) -> u32;
}

impl Arg for crate::platform::GpuDynamicInfo {
    const ARG_TYPE: dbus::arg::ArgType = dbus::arg::ArgType::Struct;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("(suuudduuuutttuu)")
    }
}

impl Append for crate::platform::GpuDynamicInfo {
    fn append_by_ref(&self, ia: &mut dbus::arg::IterAppend) {
        ia.append_struct(|ia| {
            ia.append(self.id());
            ia.append(self.temp_celsius());
            ia.append(self.fan_speed_percent());
            ia.append(self.util_percent());
            ia.append(self.power_draw_watts() as f64);
            ia.append(self.power_draw_max_watts() as f64);
            ia.append(self.clock_speed_mhz());
            ia.append(self.clock_speed_max_mhz());
            ia.append(self.mem_speed_mhz());
            ia.append(self.mem_speed_max_mhz());
            ia.append(self.free_memory());
            ia.append(self.used_memory());
            ia.append(self.used_gtt());
            ia.append(self.encoder_percent());
            ia.append(self.decoder_percent());
        });
    }
}

/// Trait that provides an interface for gathering GPU information.
pub trait GpuInfoExt<'a> {
    type S: GpuStaticInfoExt;
    type D: GpuDynamicInfoExt;
    type P: crate::platform::ProcessesExt<'a>;

    /// An iterator that yields the PCI identifiers for each GPU installed in the system
    type Iter: Iterator<Item = &'a str>;

    /// Refresh the list of available GPUs
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_gpu_list(&mut self);

    /// Refresh the internal static information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_static_info_cache(&mut self);

    /// Refresh the internal dynamic/continuously changing information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_dynamic_info_cache(&mut self, processes: &mut Self::P);

    /// Returns the number of GPUs present in the system
    fn enumerate(&'a self) -> Self::Iter;

    /// Returns the static information for GPU with the PCI id `pci_id`.
    fn static_info(&self, id: &str) -> Option<&Self::S>;

    /// Returns the dynamic information for the GPU with the PCI id `pci_id`.
    fn dynamic_info(&self, id: &str) -> Option<&Self::D>;
}
