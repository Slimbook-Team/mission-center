/* sys_info_v2/gatherer/src/platform/linux/gpu_info/mod.rs
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

use std::{
    collections::HashMap,
    fs,
    sync::{Arc, RwLock},
    time::Instant,
};

use super::{INITIAL_REFRESH_TS, MIN_DELTA_REFRESH};
use crate::{
    gpu_info_valid,
    logging::{critical, debug, error, warning},
    platform::platform_impl::gpu_info::nvtop::GPUInfoDynamicInfoValid,
    platform::{
        platform_impl::run_forked, ApiVersion, GpuDynamicInfoExt, GpuInfoExt, GpuStaticInfoExt,
        OpenGLApiVersion, ProcessesExt,
    },
};

#[allow(unused)]
mod nvtop;
mod vulkan_info;

#[derive(Debug, Clone)]
pub struct LinuxGpuStaticInfo {
    id: Arc<str>,
    device_name: Arc<str>,
    vendor_id: u16,
    device_id: u16,
    total_memory: u64,
    total_gtt: u64,
    opengl_version: Option<OpenGLApiVersion>,
    vulkan_version: Option<ApiVersion>,
    pcie_gen: u8,
    pcie_lanes: u8,
}

impl LinuxGpuStaticInfo {}

impl Default for LinuxGpuStaticInfo {
    fn default() -> Self {
        Self {
            id: Arc::from(""),
            device_name: Arc::from(""),
            vendor_id: 0,
            device_id: 0,
            total_memory: 0,
            total_gtt: 0,
            opengl_version: None,
            vulkan_version: None,
            pcie_gen: 0,
            pcie_lanes: 0,
        }
    }
}

impl GpuStaticInfoExt for LinuxGpuStaticInfo {
    fn id(&self) -> &str {
        self.id.as_ref()
    }

    fn device_name(&self) -> &str {
        self.device_name.as_ref()
    }

    fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    fn device_id(&self) -> u16 {
        self.device_id
    }

    fn total_memory(&self) -> u64 {
        self.total_memory
    }

    fn total_gtt(&self) -> u64 {
        self.total_gtt
    }

    fn opengl_version(&self) -> Option<&OpenGLApiVersion> {
        self.opengl_version.as_ref()
    }

    fn vulkan_version(&self) -> Option<&ApiVersion> {
        self.vulkan_version.as_ref()
    }

    fn metal_version(&self) -> Option<&ApiVersion> {
        None
    }

    fn direct3d_version(&self) -> Option<&ApiVersion> {
        None
    }

    fn pcie_gen(&self) -> u8 {
        self.pcie_gen
    }

    fn pcie_lanes(&self) -> u8 {
        self.pcie_lanes
    }
}

#[derive(Debug, Clone)]
pub struct LinuxGpuDynamicInfo {
    id: Arc<str>,
    temp_celsius: u32,
    fan_speed_percent: u32,
    util_percent: u32,
    power_draw_watts: f32,
    power_draw_max_watts: f32,
    clock_speed_mhz: u32,
    clock_speed_max_mhz: u32,
    mem_speed_mhz: u32,
    mem_speed_max_mhz: u32,
    free_memory: u64,
    used_memory: u64,
    used_gtt: u64,
    encoder_percent: u32,
    decoder_percent: u32,
}

impl Default for LinuxGpuDynamicInfo {
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

impl LinuxGpuDynamicInfo {
    pub fn new() -> Self {
        Default::default()
    }
}

impl GpuDynamicInfoExt for LinuxGpuDynamicInfo {
    fn id(&self) -> &str {
        self.id.as_ref()
    }

    fn temp_celsius(&self) -> u32 {
        self.temp_celsius
    }

    fn fan_speed_percent(&self) -> u32 {
        self.fan_speed_percent
    }

    fn util_percent(&self) -> u32 {
        self.util_percent
    }

    fn power_draw_watts(&self) -> f32 {
        self.power_draw_watts
    }

    fn power_draw_max_watts(&self) -> f32 {
        self.power_draw_max_watts
    }

    fn clock_speed_mhz(&self) -> u32 {
        self.clock_speed_mhz
    }

    fn clock_speed_max_mhz(&self) -> u32 {
        self.clock_speed_max_mhz
    }

    fn mem_speed_mhz(&self) -> u32 {
        self.mem_speed_mhz
    }

    fn mem_speed_max_mhz(&self) -> u32 {
        self.mem_speed_max_mhz
    }

    fn free_memory(&self) -> u64 {
        self.free_memory
    }

    fn used_memory(&self) -> u64 {
        self.used_memory
    }

    fn used_gtt(&self) -> u64 {
        self.used_gtt
    }

    fn encoder_percent(&self) -> u32 {
        self.encoder_percent
    }

    fn decoder_percent(&self) -> u32 {
        self.decoder_percent
    }
}

pub struct LinuxGpuInfo {
    gpu_list: Arc<RwLock<nvtop::ListHead>>,
    static_info: HashMap<arrayvec::ArrayString<16>, LinuxGpuStaticInfo>,
    dynamic_info: HashMap<arrayvec::ArrayString<16>, LinuxGpuDynamicInfo>,

    gpu_list_refreshed: bool,

    static_refresh_timestamp: Instant,
    dynamic_refresh_timestamp: Instant,
}

impl Drop for LinuxGpuInfo {
    fn drop(&mut self) {
        use std::ops::DerefMut;

        let mut gl = self.gpu_list.write().unwrap();
        unsafe {
            nvtop::gpuinfo_shutdown_info_extraction(gl.deref_mut());
        }
    }
}

impl LinuxGpuInfo {
    pub fn new() -> Self {
        use std::ops::DerefMut;

        unsafe {
            nvtop::init_extract_gpuinfo_intel();
            nvtop::init_extract_gpuinfo_amdgpu();
            nvtop::init_extract_gpuinfo_nvidia();
        }

        let gpu_list = Arc::new(RwLock::new(nvtop::ListHead {
            next: std::ptr::null_mut(),
            prev: std::ptr::null_mut(),
        }));
        {
            let mut gl = gpu_list.write().unwrap();
            gl.next = gl.deref_mut();
            gl.prev = gl.deref_mut();
        }

        let mut this = Self {
            gpu_list,

            static_info: HashMap::new(),
            dynamic_info: HashMap::new(),

            gpu_list_refreshed: false,

            static_refresh_timestamp: *INITIAL_REFRESH_TS,
            dynamic_refresh_timestamp: *INITIAL_REFRESH_TS,
        };

        this.refresh_gpu_list();

        this
    }

    #[allow(non_snake_case)]
    unsafe fn supported_opengl_version(dri_path: &str) -> Option<OpenGLApiVersion> {
        use crate::platform::OpenGLApi;
        use gbm::AsRaw;
        use std::os::fd::*;

        type Void = std::ffi::c_void;

        pub struct DrmDevice(std::fs::File);

        impl AsFd for DrmDevice {
            fn as_fd(&self) -> BorrowedFd<'_> {
                self.0.as_fd()
            }
        }

        impl DrmDevice {
            pub fn open(path: &str) -> std::io::Result<Self> {
                let mut options = std::fs::OpenOptions::new();
                options.read(true);
                options.write(true);

                Ok(Self(options.open(path)?))
            }
        }

        impl drm::Device for DrmDevice {}

        let drm_device = match DrmDevice::open(dri_path) {
            Err(e) => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: {}", e
                );
                return None;
            }
            Ok(drm_device) => drm_device,
        };

        let gbm_device = match gbm::Device::new(drm_device) {
            Err(e) => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: {}", e
                );
                return None;
            }
            Ok(gbm_device) => gbm_device,
        };

        const EGL_CONTEXT_MAJOR_VERSION_KHR: egl::EGLint = 0x3098;
        const EGL_CONTEXT_MINOR_VERSION_KHR: egl::EGLint = 0x30FB;
        const EGL_PLATFORM_GBM_KHR: egl::EGLenum = 0x31D7;
        const EGL_OPENGL_ES3_BIT: egl::EGLint = 0x0040;

        let eglGetPlatformDisplayEXT =
            egl::get_proc_address("eglGetPlatformDisplayEXT") as *const Void;
        let egl_display = if !eglGetPlatformDisplayEXT.is_null() {
            let eglGetPlatformDisplayEXT: extern "C" fn(
                egl::EGLenum,
                *mut Void,
                *const egl::EGLint,
            ) -> egl::EGLDisplay = std::mem::transmute(eglGetPlatformDisplayEXT);
            eglGetPlatformDisplayEXT(
                EGL_PLATFORM_GBM_KHR,
                gbm_device.as_raw() as *mut Void,
                std::ptr::null(),
            )
        } else {
            let eglGetPlatformDisplay =
                egl::get_proc_address("eglGetPlatformDisplay") as *const Void;
            if !eglGetPlatformDisplay.is_null() {
                let eglGetPlatformDisplay: extern "C" fn(
                    egl::EGLenum,
                    *mut Void,
                    *const egl::EGLint,
                ) -> egl::EGLDisplay = std::mem::transmute(eglGetPlatformDisplay);
                eglGetPlatformDisplay(
                    EGL_PLATFORM_GBM_KHR,
                    gbm_device.as_raw() as *mut Void,
                    std::ptr::null(),
                )
            } else {
                egl::get_display(gbm_device.as_raw() as *mut Void)
                    .map_or(std::ptr::null_mut(), |d| d)
            }
        };
        if egl_display.is_null() {
            error!(
                "Gatherer::GpuInfo",
                "Failed to get OpenGL information: Failed to initialize an EGL display ({:X})",
                egl::get_error()
            );
            return None;
        }

        let mut egl_major = 0;
        let mut egl_minor = 0;
        if !egl::initialize(egl_display, &mut egl_major, &mut egl_minor) {
            error!(
                "Gathereer::GpuInfo",
                "Failed to get OpenGL information: Failed to initialize an EGL display ({:X})",
                egl::get_error()
            );
            return None;
        }

        if egl_major < 1 || (egl_major == 1 && egl_minor < 4) {
            error!(
                "Gatherer::GpuInfo",
                "Failed to get OpenGL information: EGL version 1.4 or higher is required to test OpenGL support"
            );
            return None;
        }

        let mut gl_api = egl::EGL_OPENGL_API;
        if !egl::bind_api(gl_api) {
            gl_api = egl::EGL_OPENGL_ES_API;
            if !egl::bind_api(gl_api) {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: Failed to bind an EGL API ({:X})",
                    egl::get_error()
                );
                return None;
            }
        }

        let egl_config = if gl_api == egl::EGL_OPENGL_ES_API {
            let mut config_attribs = [
                egl::EGL_SURFACE_TYPE,
                egl::EGL_WINDOW_BIT,
                egl::EGL_RENDERABLE_TYPE,
                EGL_OPENGL_ES3_BIT,
                egl::EGL_NONE,
            ];

            let mut egl_config = egl::choose_config(egl_display, &config_attribs, 1);
            if egl_config.is_some() {
                egl_config
            } else {
                config_attribs[3] = egl::EGL_OPENGL_ES2_BIT;
                egl_config = egl::choose_config(egl_display, &config_attribs, 1);
                if egl_config.is_some() {
                    egl_config
                } else {
                    config_attribs[3] = egl::EGL_OPENGL_ES_BIT;
                    egl::choose_config(egl_display, &config_attribs, 1)
                }
            }
        } else {
            let config_attribs = [
                egl::EGL_SURFACE_TYPE,
                egl::EGL_WINDOW_BIT,
                egl::EGL_RENDERABLE_TYPE,
                egl::EGL_OPENGL_BIT,
                egl::EGL_NONE,
            ];

            egl::choose_config(egl_display, &config_attribs, 1)
        };

        if egl_config.is_none() {
            return None;
        }
        let egl_config = match egl_config {
            Some(ec) => ec,
            None => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: Failed to choose an EGL config ({:X})",
                    egl::get_error()
                );
                return None;
            }
        };

        let mut ver_major = if gl_api == egl::EGL_OPENGL_API { 4 } else { 3 };
        let mut ver_minor = if gl_api == egl::EGL_OPENGL_API { 6 } else { 0 };

        let mut context_attribs = [
            EGL_CONTEXT_MAJOR_VERSION_KHR,
            ver_major,
            EGL_CONTEXT_MINOR_VERSION_KHR,
            ver_minor,
            egl::EGL_NONE,
        ];

        let mut egl_context;
        loop {
            egl_context = egl::create_context(
                egl_display,
                egl_config,
                egl::EGL_NO_CONTEXT,
                &context_attribs,
            );

            if egl_context.is_some() || (ver_major == 1 && ver_minor == 0) {
                break;
            }

            if ver_minor > 0 {
                ver_minor -= 1;
            } else {
                ver_major -= 1;
                ver_minor = 9;
            }

            context_attribs[1] = ver_major;
            context_attribs[3] = ver_minor;
        }

        match egl_context {
            Some(ec) => egl::destroy_context(egl_display, ec),
            None => {
                error!(
                    "Gatherer::GpuInfo",
                    "Failed to get OpenGL information: Failed to create an EGL context ({:X})",
                    egl::get_error()
                );
                return None;
            }
        };

        Some(OpenGLApiVersion {
            major: ver_major as u8,
            minor: ver_minor as u8,
            api: if gl_api != egl::EGL_OPENGL_API {
                OpenGLApi::OpenGLES
            } else {
                OpenGLApi::OpenGL
            },
        })
    }
}

impl<'a> GpuInfoExt<'a> for LinuxGpuInfo {
    type S = LinuxGpuStaticInfo;
    type D = LinuxGpuDynamicInfo;
    type P = crate::platform::Processes;
    type Iter = std::iter::Map<
        std::collections::hash_map::Keys<'a, arrayvec::ArrayString<16>, LinuxGpuStaticInfo>,
        fn(&arrayvec::ArrayString<16>) -> &str,
    >;

    fn refresh_gpu_list(&mut self) {
        use arrayvec::ArrayString;
        use std::{io::Read, ops::DerefMut};

        if self.gpu_list_refreshed {
            return;
        }

        self.gpu_list_refreshed = true;

        let mut gpu_list = self.gpu_list.write().unwrap();
        let gpu_list = gpu_list.deref_mut();

        let mut gpu_count: u32 = 0;
        let nvt_result = unsafe { nvtop::gpuinfo_init_info_extraction(&mut gpu_count, gpu_list) };
        if nvt_result == 0 {
            critical!(
                "Gatherer::GpuInfo",
                "Unable to initialize GPU info extraction"
            );
            return;
        }

        let nvt_result = unsafe { nvtop::gpuinfo_populate_static_infos(gpu_list) };
        if nvt_result == 0 {
            unsafe { nvtop::gpuinfo_shutdown_info_extraction(gpu_list) };

            critical!("Gatherer::GPUInfo", "Unable to populate static GPU info");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_refresh_dynamic_info(gpu_list) };
        if result == 0 {
            critical!("Gatherer::GpuInfo", "Unable to refresh dynamic GPU info");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_utilisation_rate(gpu_list) };
        if result == 0 {
            critical!("Gatherer::GpuInfo", "Unable to refresh utilization rate");
            return;
        }

        self.static_info.clear();
        self.dynamic_info.clear();

        let mut buffer = String::new();

        let mut device = gpu_list.next;
        while device != gpu_list {
            use std::fmt::Write;

            let dev: &nvtop::GPUInfo = unsafe { core::mem::transmute(device) };
            device = unsafe { (*device).next };

            let pdev = unsafe { std::ffi::CStr::from_ptr(dev.pdev.as_ptr()) };
            let pdev = match pdev.to_str() {
                Ok(pd) => pd,
                Err(_) => {
                    warning!(
                        "Gatherer::GpuInfo",
                        "Unable to convert PCI ID to string: {:?}",
                        pdev
                    );
                    continue;
                }
            };
            let mut pci_bus_id = ArrayString::<16>::new();
            match write!(pci_bus_id, "{}", pdev) {
                Ok(_) => {}
                Err(_) => {
                    warning!(
                        "Gatherer::GpuInfo",
                        "PCI ID exceeds 16 characters: {}",
                        pdev
                    );
                    continue;
                }
            }

            let device_name =
                unsafe { std::ffi::CStr::from_ptr(dev.static_info.device_name.as_ptr()) };
            let device_name = device_name.to_str().unwrap_or_else(|_| "Unknown");

            let mut uevent_path = ArrayString::<64>::new();
            let _ = write!(uevent_path, "/sys/bus/pci/devices/{}/uevent", pdev);
            let uevent_file = match std::fs::OpenOptions::new()
                .read(true)
                .open(uevent_path.as_str())
            {
                Ok(f) => Some(f),
                Err(_) => {
                    uevent_path.clear();
                    let _ = write!(
                        uevent_path,
                        "/sys/bus/pci/devices/{}/uevent",
                        pdev.to_lowercase()
                    );
                    match std::fs::OpenOptions::new()
                        .read(true)
                        .open(uevent_path.as_str())
                    {
                        Ok(f) => Some(f),
                        Err(_) => {
                            warning!(
                                "Gatherer::GPUInfo",
                                "Unable to open `uevent` file for device {}",
                                pdev
                            );
                            None
                        }
                    }
                }
            };

            let total_gtt = match fs::read_to_string(format!(
                "/sys/bus/pci/devices/{}/mem_info_gtt_total",
                pdev.to_lowercase()
            )) {
                Ok(x) => match x.trim().parse::<u64>() {
                    Ok(x) => x,
                    Err(x) => {
                        debug!("Gatherer::GpuInfo", "Failed to parse total gtt: {}", x);
                        0
                    }
                },
                Err(x) => {
                    debug!("Gatherer::GpuInfo", "Failed to read total gtt: {}", x);
                    0
                }
            };
            let ven_dev_id = if let Some(mut f) = uevent_file {
                buffer.clear();
                match f.read_to_string(&mut buffer) {
                    Ok(_) => {
                        let mut vendor_id = 0;
                        let mut device_id = 0;

                        for line in buffer.lines().map(|l| l.trim()) {
                            if line.starts_with("PCI_ID=") {
                                let mut ids = line[7..].split(':');
                                vendor_id = ids
                                    .next()
                                    .and_then(|id| u16::from_str_radix(id, 16).ok())
                                    .unwrap_or(0);
                                device_id = ids
                                    .next()
                                    .and_then(|id| u16::from_str_radix(id, 16).ok())
                                    .unwrap_or(0);
                                break;
                            }
                        }

                        (vendor_id, device_id)
                    }
                    Err(_) => {
                        warning!(
                            "Gatherer::GPUInfo",
                            "Unable to read `uevent` file content for device {}",
                            pdev
                        );
                        (0, 0)
                    }
                }
            } else {
                (0, 0)
            };

            let static_info = LinuxGpuStaticInfo {
                id: Arc::from(pdev),
                device_name: Arc::from(device_name),
                vendor_id: ven_dev_id.0,
                device_id: ven_dev_id.1,

                total_memory: dev.dynamic_info.total_memory,
                total_gtt,

                pcie_gen: dev.dynamic_info.pcie_link_gen as _,
                pcie_lanes: dev.dynamic_info.pcie_link_width as _,

                // Leave the rest for when static info is actually requested
                ..Default::default()
            };

            self.static_info.insert(pci_bus_id.clone(), static_info);
            self.dynamic_info
                .insert(pci_bus_id, LinuxGpuDynamicInfo::new());
        }
    }

    fn refresh_static_info_cache(&mut self) {
        use arrayvec::ArrayString;
        use std::fmt::Write;

        if !self.gpu_list_refreshed {
            return;
        }

        let now = Instant::now();
        if self.static_refresh_timestamp.elapsed() < MIN_DELTA_REFRESH {
            return;
        }
        self.static_refresh_timestamp = now;

        let vulkan_versions = unsafe {
            run_forked(|| {
                if let Some(vulkan_info) = vulkan_info::VulkanInfo::new() {
                    Ok(vulkan_info
                        .supported_vulkan_versions()
                        .unwrap_or(HashMap::new()))
                } else {
                    Ok(HashMap::new())
                }
            })
        };
        let vulkan_versions = vulkan_versions.unwrap_or_else(|e| {
            warning!(
                "Gatherer::GpuInfo",
                "Failed to get Vulkan information: {}",
                e
            );
            HashMap::new()
        });

        let mut dri_path = ArrayString::<64>::new_const();
        for (pci_id, static_info) in &mut self.static_info {
            let _ = write!(dri_path, "/dev/dri/by-path/pci-{}-card", pci_id);
            if !std::path::Path::new(dri_path.as_str()).exists() {
                dri_path.clear();
                let _ = write!(
                    dri_path,
                    "/dev/dri/by-path/pci-{}-card",
                    pci_id.to_ascii_lowercase()
                );
            }
            static_info.opengl_version = unsafe {
                run_forked(|| Ok(Self::supported_opengl_version(dri_path.as_str()))).unwrap_or_else(
                    |e| {
                        warning!(
                            "Gatherer::GpuInfo",
                            "Failed to get OpenGL information: {}",
                            e
                        );
                        None
                    },
                )
            };

            let device_id = ((static_info.vendor_id as u32) << 16) | static_info.device_id as u32;
            if let Some(vulkan_version) = vulkan_versions.get(&device_id) {
                static_info.vulkan_version = Some(*vulkan_version);
            }
        }
    }

    fn refresh_dynamic_info_cache(&mut self, processes: &mut Self::P) {
        use std::ops::DerefMut;

        if !self.gpu_list_refreshed {
            return;
        }

        let now = Instant::now();
        if self.dynamic_refresh_timestamp.elapsed() < MIN_DELTA_REFRESH {
            return;
        }
        self.dynamic_refresh_timestamp = now;

        let mut gpu_list = self.gpu_list.write().unwrap();
        let gpu_list = gpu_list.deref_mut();

        let result = unsafe { nvtop::gpuinfo_refresh_dynamic_info(gpu_list) };
        if result == 0 {
            error!("Gatherer::GpuInfo", "Unable to refresh dynamic GPU info");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_refresh_processes(gpu_list) };
        if result == 0 {
            error!("Gatherer::GpuInfo", "Unable to refresh GPU processes");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_utilisation_rate(gpu_list) };
        if result == 0 {
            critical!("Gatherer::GpuInfo", "Unable to refresh utilization rate");
            return;
        }

        let result = unsafe { nvtop::gpuinfo_fix_dynamic_info_from_process_info(gpu_list) };
        if result == 0 {
            error!(
                "Gatherer::GpuInfo",
                "Unable to fix dynamic GPU info from process info"
            );
            return;
        }

        let processes = processes.process_list_mut();

        let mut device: *mut nvtop::ListHead = gpu_list.next;
        while device != gpu_list {
            let dev: &nvtop::GPUInfo = unsafe { core::mem::transmute(device) };
            device = unsafe { (*device).next };

            let pdev = unsafe { std::ffi::CStr::from_ptr(dev.pdev.as_ptr()) };
            let pdev = match pdev.to_str() {
                Ok(pd) => pd,
                Err(_) => {
                    warning!(
                        "Gatherer::GpuInfo",
                        "Unable to convert PCI ID to string: {:?}",
                        pdev
                    );
                    continue;
                }
            };
            let pci_id = match arrayvec::ArrayString::<16>::from(pdev) {
                Ok(id) => id,
                Err(_) => {
                    warning!(
                        "Gatherer::GpuInfo",
                        "PCI ID exceeds 16 characters: {}",
                        pdev
                    );
                    continue;
                }
            };

            let used_gtt = match fs::read_to_string(format!(
                "/sys/bus/pci/devices/{}/mem_info_gtt_used",
                pdev.to_lowercase()
            )) {
                Ok(x) => match x.trim().parse::<u64>() {
                    Ok(x) => x,
                    Err(x) => {
                        debug!("Gatherer::GpuInfo", "Failed to parse used gtt: {}", x);
                        0
                    }
                },
                Err(x) => {
                    debug!("Gatherer::GpuInfo", "Failed to read used gtt: {}", x);
                    0
                }
            };

            let dynamic_info = self.dynamic_info.get_mut(&pci_id);
            if dynamic_info.is_none() {
                continue;
            }
            let dynamic_info = unsafe { dynamic_info.unwrap_unchecked() };
            dynamic_info.id = Arc::from(pdev);
            dynamic_info.temp_celsius = dev.dynamic_info.gpu_temp;
            dynamic_info.fan_speed_percent = dev.dynamic_info.fan_speed;
            dynamic_info.util_percent = dev.dynamic_info.gpu_util_rate;
            dynamic_info.power_draw_watts = dev.dynamic_info.power_draw as f32 / 1000.;
            dynamic_info.power_draw_max_watts = dev.dynamic_info.power_draw_max as f32 / 1000.;
            dynamic_info.clock_speed_mhz = dev.dynamic_info.gpu_clock_speed;
            dynamic_info.clock_speed_max_mhz = dev.dynamic_info.gpu_clock_speed_max;
            dynamic_info.mem_speed_mhz = dev.dynamic_info.mem_clock_speed;
            dynamic_info.mem_speed_max_mhz = dev.dynamic_info.mem_clock_speed_max;
            dynamic_info.free_memory = dev.dynamic_info.free_memory;
            dynamic_info.used_memory = dev.dynamic_info.used_memory;
            dynamic_info.used_gtt = used_gtt;
            dynamic_info.encoder_percent = {
                if gpu_info_valid!(dev.dynamic_info, GPUInfoDynamicInfoValid::EncoderRateValid) {
                    dev.dynamic_info.encoder_rate
                } else {
                    0
                }
            };
            dynamic_info.decoder_percent = {
                if gpu_info_valid!(dev.dynamic_info, GPUInfoDynamicInfoValid::DecoderRateValid) {
                    dev.dynamic_info.decoder_rate
                } else {
                    0
                }
            };

            for i in 0..dev.processes_count as usize {
                let process = unsafe { &*dev.processes.add(i) };
                if let Some(proc) = processes.get_mut(&(process.pid as u32)) {
                    proc.usage_stats.gpu_usage = process.gpu_usage as f32;
                    proc.usage_stats.gpu_memory_usage = process.gpu_memory_usage as f32;
                }
            }
        }
    }

    fn enumerate(&'a self) -> Self::Iter {
        self.static_info.keys().map(|k| k.as_str())
    }

    fn static_info(&self, id: &str) -> Option<&Self::S> {
        use arrayvec::ArrayString;

        self.static_info
            .get(&ArrayString::<16>::from(id).unwrap_or_default())
    }

    fn dynamic_info(&self, id: &str) -> Option<&Self::D> {
        use arrayvec::ArrayString;

        self.dynamic_info
            .get(&ArrayString::<16>::from(id).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_gpu_info() {
        use crate::platform::Processes;

        let mut gpu_info = LinuxGpuInfo::new();
        gpu_info.refresh_gpu_list();
        let pci_id = {
            let pci_ids = gpu_info.enumerate();
            dbg!(&pci_ids);

            gpu_info.enumerate().next().unwrap_or("").to_owned()
        };

        gpu_info.refresh_static_info_cache();
        let static_info = gpu_info.static_info(&pci_id);
        dbg!(&static_info);

        let mut p = Processes::default();
        gpu_info.refresh_dynamic_info_cache(&mut p);
        let _ = gpu_info.dynamic_info(&pci_id);

        std::thread::sleep(std::time::Duration::from_millis(500));

        gpu_info.refresh_dynamic_info_cache(&mut p);
        let dynamic_info = gpu_info.dynamic_info(&pci_id);
        dbg!(&dynamic_info);
    }
}
