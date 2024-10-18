/* sys_info_v2/gatherer/src/gpu/nvtop.rs
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

#[macro_export]
macro_rules! gpu_info_valid {
    ($info: expr, $field: expr) => {{
        let field = $field as usize;
        ((($info).valid)[field / 8] & (1 << (field % 8))) != 0
    }};
}

const MAX_DEVICE_NAME: usize = 128;
const PDEV_LEN: usize = 16;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ListHead {
    pub next: *mut ListHead,
    pub prev: *mut ListHead,
}

unsafe impl Send for ListHead {}

unsafe impl Sync for ListHead {}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GPUVendor {
    pub list: ListHead,

    pub init: Option<fn() -> u8>,
    pub shutdown: Option<fn()>,

    pub last_error_string: Option<extern "C" fn() -> *const i8>,

    pub get_device_handles: Option<extern "C" fn(devices: *mut ListHead, count: *mut u32) -> u8>,

    pub populate_static_info: Option<extern "C" fn(gpu_info: *mut GPUInfo)>,
    pub refresh_dynamic_info: Option<extern "C" fn(gpu_info: *mut GPUInfo)>,
    pub refresh_utilisation_rate: Option<extern "C" fn(gpu_info: *mut GPUInfo)>,

    pub refresh_running_processes: Option<extern "C" fn(gpu_info: *mut GPUInfo)>,

    pub name: *mut i8,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum GPUInfoStaticInfoValid {
    DeviceNameValid = 0,
    MaxPcieGenValid,
    MaxPcieLinkWidthValid,
    TemperatureShutdownThresholdValid,
    TemperatureSlowdownThresholdValid,
    NumberSharedCoresValid,
    L2CacheSizeValid,
    NumberExecEnginesValid,
    StaticInfoCount,
}

const GPU_INFO_STATIC_INFO_COUNT: usize = GPUInfoStaticInfoValid::StaticInfoCount as usize;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GPUInfoStaticInfo {
    pub device_name: [libc::c_char; MAX_DEVICE_NAME],
    pub max_pcie_gen: u32,
    pub max_pcie_link_width: u32,
    pub temperature_shutdown_threshold: u32,
    pub temperature_slowdown_threshold: u32,
    pub n_shared_cores: u32,
    pub l2cache_size: u32,
    pub n_exec_engines: u32,
    pub integrated_graphics: u8,
    pub encode_decode_shared: u8,
    pub valid: [u8; (GPU_INFO_STATIC_INFO_COUNT + 7) / 8],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum GPUInfoDynamicInfoValid {
    GpuClockSpeedValid = 0,
    GpuClockSpeedMaxValid,
    MemClockSpeedValid,
    MemClockSpeedMaxValid,
    GpuUtilRateValid,
    MemUtilRateValid,
    EncoderRateValid,
    DecoderRateValid,
    TotalMemoryValid,
    FreeMemoryValid,
    UsedMemoryValid,
    PcieLinkGenValid,
    PcieLinkWidthValid,
    PcieRxValid,
    PcieTxValid,
    FanSpeedValid,
    GpuTempValid,
    PowerDrawValid,
    PowerDrawMaxValid,
    MultiInstanceModeValid,
    DynamicInfoCount,
}

const GPU_INFO_DYNAMIC_INFO_COUNT: usize = GPUInfoDynamicInfoValid::DynamicInfoCount as usize;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GPUInfoDynamicInfo {
    pub gpu_clock_speed: u32,
    pub gpu_clock_speed_max: u32,
    pub mem_clock_speed: u32,
    pub mem_clock_speed_max: u32,
    pub gpu_util_rate: u32,
    pub mem_util_rate: u32,
    pub encoder_rate: u32,
    pub decoder_rate: u32,
    pub total_memory: u64,
    pub free_memory: u64,
    pub used_memory: u64,
    pub pcie_link_gen: u32,
    pub pcie_link_width: u32,
    pub pcie_rx: u32,
    pub pcie_tx: u32,
    pub fan_speed: u32,
    pub gpu_temp: u32,
    pub power_draw: u32,
    pub power_draw_max: u32,
    pub multi_instance_mode: u8,
    pub valid: [u8; (GPU_INFO_DYNAMIC_INFO_COUNT + 7) / 8],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum GPUProcessType {
    Unknown = 0,
    Graphical = 1,
    Compute = 2,
    GraphicalCompute = 3,
    Count,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum GPUInfoProcessInfoValid {
    CmdlineValid,
    UserNameValid,
    GfxEngineUsedValid,
    ComputeEngineUsedValid,
    EncEngineUsedValid,
    DecEngineUsedValid,
    GpuUsageValid,
    EncodeUsageValid,
    DecodeUsageValid,
    GpuMemoryUsageValid,
    GpuMemoryPercentageValid,
    CpuUsageValid,
    CpuMemoryVirtValid,
    CpuMemoryResValid,
    GpuCyclesValid,
    SampleDeltaValid,
    ProcessValidInfoCount,
}

const GPU_PROCESS_INFO_VALID_INFO_COUNT: usize =
    GPUInfoProcessInfoValid::ProcessValidInfoCount as usize;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GPUProcess {
    pub r#type: GPUProcessType,
    pub pid: i32,
    pub cmdline: *mut libc::c_char,
    pub user_name: *mut libc::c_char,
    pub sample_delta: u64, // Time spent between two successive samples
    pub gfx_engine_used: u64,
    pub compute_engine_used: u64,
    pub enc_engine_used: u64,
    pub dec_engine_used: u64,
    pub gpu_cycles: u64, // Number of GPU cycles spent in the GPU gfx engine
    pub gpu_usage: u32,
    pub encode_usage: u32,
    pub decode_usage: u32,
    pub gpu_memory_usage: libc::c_ulonglong,
    pub gpu_memory_percentage: u32,
    pub cpu_usage: u32,
    pub cpu_memory_virt: libc::c_ulong,
    pub cpu_memory_res: libc::c_ulong,
    pub valid: [u8; (GPU_PROCESS_INFO_VALID_INFO_COUNT + 7) / 8],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GPUInfo {
    pub list: ListHead,
    pub vendor: *mut GPUVendor,
    pub static_info: GPUInfoStaticInfo,
    pub dynamic_info: GPUInfoDynamicInfo,
    pub processes_count: u32,
    pub processes: *mut GPUProcess,
    pub processes_array_size: u32,
    pub pdev: [libc::c_char; PDEV_LEN],
}

extern "C" {
    pub fn gpuinfo_init_info_extraction(
        monitored_dev_count: *mut u32,
        devices: *mut ListHead,
    ) -> u8;

    pub fn gpuinfo_shutdown_info_extraction(devices: *mut ListHead) -> u8;

    pub fn init_extract_gpuinfo_amdgpu();
    pub fn init_extract_gpuinfo_intel();
    pub fn init_extract_gpuinfo_msm();
    pub fn init_extract_gpuinfo_nvidia();

    pub fn gpuinfo_populate_static_infos(devices: *mut ListHead) -> u8;
    pub fn gpuinfo_refresh_dynamic_info(devices: *mut ListHead) -> u8;
    pub fn gpuinfo_refresh_processes(devices: *mut ListHead) -> u8;
    pub fn gpuinfo_utilisation_rate(devices: *mut ListHead) -> u8;
    pub fn gpuinfo_fix_dynamic_info_from_process_info(devices: *mut ListHead) -> u8;
}
