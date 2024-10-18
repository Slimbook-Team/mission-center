/* sys_info_v2/net_info.rs
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

use std::path::Path;

use crate::i18n::*;

#[allow(non_camel_case_types)]
mod if_nameindex {
    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub struct if_nameindex {
        pub if_index: u32,
        pub if_name: *mut libc::c_char,
    }

    extern "C" {
        #[link_name = "if_freenameindex"]
        pub fn free(ptr: *mut if_nameindex);
        #[link_name = "if_nameindex"]
        pub fn new() -> *mut if_nameindex;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum NetDeviceType {
    Bluetooth,
    Bridge,
    Docker,
    InfiniBand,
    Multipass,
    Virtual,
    VPN,
    Wired,
    Wireless,
    WWAN,
    Other = 255,
}

impl ToString for NetDeviceType {
    fn to_string(&self) -> String {
        match self {
            NetDeviceType::Bluetooth => i18n("Bluetooth"),
            NetDeviceType::Bridge => i18n("Bridge"),
            NetDeviceType::Docker => i18n("Docker"),
            NetDeviceType::InfiniBand => i18n("InfiniBand"),
            NetDeviceType::Multipass => i18n("Multipass"),
            NetDeviceType::Virtual => i18n("Virtual"),
            NetDeviceType::VPN => i18n("VPN"),
            NetDeviceType::Wired => i18n("Ethernet"),
            NetDeviceType::Wireless => i18n("Wi-Fi"),
            NetDeviceType::WWAN => i18n("WWAN"),
            NetDeviceType::Other => i18n("Other"),
        }
    }
}

impl From<u8> for NetDeviceType {
    fn from(v: u8) -> Self {
        match v {
            0 => NetDeviceType::Bluetooth,
            1 => NetDeviceType::Bridge,
            2 => NetDeviceType::Docker,
            3 => NetDeviceType::InfiniBand,
            4 => NetDeviceType::Multipass,
            5 => NetDeviceType::Virtual,
            6 => NetDeviceType::VPN,
            7 => NetDeviceType::Wired,
            8 => NetDeviceType::Wireless,
            9 => NetDeviceType::WWAN,
            _ => NetDeviceType::Other,
        }
    }
}

impl From<NetDeviceType> for u8 {
    fn from(v: NetDeviceType) -> Self {
        match v {
            NetDeviceType::Bluetooth => 0,
            NetDeviceType::Bridge => 1,
            NetDeviceType::Docker => 2,
            NetDeviceType::InfiniBand => 3,
            NetDeviceType::Multipass => 4,
            NetDeviceType::Virtual => 5,
            NetDeviceType::VPN => 6,
            NetDeviceType::Wired => 7,
            NetDeviceType::Wireless => 8,
            NetDeviceType::WWAN => 9,
            NetDeviceType::Other => 255,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDeviceDescriptor {
    pub kind: NetDeviceType,
    pub if_name: String,
    pub adapter_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkAddress {
    pub hw_address: Option<[u8; 6]>,
    pub ip4_address: Option<u32>,
    pub ip6_address: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirelessInfo {
    pub ssid: Option<String>,
    pub frequency_mhz: Option<u32>,
    pub bitrate_kbps: Option<u32>,
    pub signal_strength_percent: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkDevice {
    pub descriptor: NetworkDeviceDescriptor,
    pub address: NetworkAddress,
    pub wireless_info: Option<WirelessInfo>,

    pub send_bps: f32,
    pub sent_bytes: u64,
    pub recv_bps: f32,
    pub recv_bytes: u64,

    pub max_speed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NetworkDeviceCacheEntry {
    pub descriptor: NetworkDeviceDescriptor,
    pub hw_address: Option<[u8; 6]>,

    pub tx_bytes: u64,
    pub rx_bytes: u64,

    pub update_timestamp: std::time::Instant,
}

pub struct NetInfo {
    udev: *mut libudev_sys::udev,
    nm_proxy: *mut gtk::gio::ffi::GDBusProxy,

    device_cache: std::collections::HashMap<String, NetworkDeviceCacheEntry>,

    hwdb_conn: Option<rusqlite::Connection>,
    device_name_cache: std::collections::HashMap<String, String>,
}

unsafe impl Send for NetInfo {}
unsafe impl Sync for NetInfo {}

impl Drop for NetInfo {
    fn drop(&mut self) {
        use gtk::glib::gobject_ffi::*;
        use libudev_sys::*;

        unsafe {
            if self.nm_proxy != std::ptr::null_mut() {
                g_object_unref(self.nm_proxy as _);
            }

            if self.udev != std::ptr::null_mut() {
                udev_unref(self.udev);
            }
        }
    }
}

impl NetInfo {
    pub fn new() -> Option<Self> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, *};
        use libudev_sys::*;
        use std::{collections::*, path::*};

        let mut error: *mut GError = std::ptr::null_mut();

        let nm_proxy = unsafe {
            g_dbus_proxy_new_for_bus_sync(
                G_BUS_TYPE_SYSTEM,
                G_DBUS_PROXY_FLAGS_NONE,
                std::ptr::null_mut(),
                b"org.freedesktop.NetworkManager\0".as_ptr() as _,
                b"/org/freedesktop/NetworkManager\0".as_ptr() as _,
                b"org.freedesktop.NetworkManager\0".as_ptr() as _,
                std::ptr::null_mut(),
                &mut error,
            )
        };
        if nm_proxy.is_null() {
            if !error.is_null() {
                let error: Error = unsafe { from_glib_full(error) };
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to connect to NetworkManager: {}",
                    error.message()
                );
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to connect to NetworkManager: Unknown error"
                );
            }
            return None;
        }

        let udev = unsafe { udev_new() };
        if nm_proxy == std::ptr::null_mut() {
            g_critical!("MissionCenter::NetInfo", "Failed to create udev context");
            return None;
        }

        let conn = if let Ok(conn) =
            rusqlite::Connection::open(Path::new(crate::HW_DB_DIR.as_str()).join("hw.db"))
        {
            Some(conn)
        } else {
            g_warning!(
                "MissionCenter::NetInfo",
                "Failed to load hardware database, network devices will (probably) have missing names",
            );

            None
        };

        Some(Self {
            udev,
            nm_proxy,

            device_cache: HashMap::new(),

            hwdb_conn: conn,
            device_name_cache: HashMap::new(),
        })
    }

    pub fn load_devices(&mut self) -> Vec<NetworkDevice> {
        use gtk::glib::{gobject_ffi::*, *};

        let mut result = vec![];

        let if_list = unsafe { if_nameindex::new() };
        if if_list.is_null() {
            g_warning!(
                "MissionCenter::NetInfo",
                "Failed to list network interfaces"
            );

            return result;
        }

        let mut if_list_it = unsafe { if_list.offset(-1) };
        loop {
            unsafe {
                if_list_it = if_list_it.offset(1);
            }

            let if_entry = unsafe { &*if_list_it };
            if if_entry.if_index == 0 || if_entry.if_name.is_null() {
                break;
            }

            let if_name = unsafe { std::ffi::CStr::from_ptr(if_entry.if_name) };

            let if_name_str = if_name.to_string_lossy();
            if if_name_str.starts_with("lo") {
                continue;
            }

            let device_path = match unsafe { self.nm_device_obj_path_new(if_name) } {
                None => {
                    g_debug!(
                        "MissionCenter::NetInfo",
                        "Failed to get device path for {}",
                        if_name_str
                    );
                    continue;
                }
                Some(dp) => dp,
            };

            let device_proxy = unsafe {
                Self::create_nm_dbus_proxy(
                    device_path.as_bytes_with_nul(),
                    b"org.freedesktop.NetworkManager.Device\0",
                )
            };
            if device_proxy.is_null() {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to create dbus proxy for {}",
                    if_name_str
                );

                continue;
            }

            let if_name = if_name_str.to_string();

            let max_speed = Self::get_max_speed(&if_name);

            let (tx_bytes, rx_bytes) = Self::tx_rx_bytes(&if_name);

            let (descriptor, hw_address, send_bps, send_bytes, recv_bps, rec_bytes) =
                if let Some(cached_device) = self.device_cache.get_mut(&if_name) {
                    let prev_tx_bytes = if cached_device.tx_bytes > tx_bytes {
                        tx_bytes
                    } else {
                        cached_device.tx_bytes
                    };

                    let prev_rx_bytes = if cached_device.rx_bytes > rx_bytes {
                        rx_bytes
                    } else {
                        cached_device.rx_bytes
                    };

                    let elapsed = cached_device.update_timestamp.elapsed().as_secs_f32();
                    let send_bps = (tx_bytes - prev_tx_bytes) as f32 / elapsed;
                    let recv_bps = (rx_bytes - prev_rx_bytes) as f32 / elapsed;

                    cached_device.tx_bytes = tx_bytes;
                    cached_device.rx_bytes = rx_bytes;
                    cached_device.update_timestamp = std::time::Instant::now();

                    (
                        cached_device.descriptor.clone(),
                        cached_device.hw_address.clone(),
                        send_bps,
                        tx_bytes,
                        recv_bps,
                        rx_bytes,
                    )
                } else {
                    let kind = Self::device_kind(&if_name);
                    let adapter_name = unsafe { self.adapter_name(device_proxy) };
                    let hw_address = Self::hw_address(device_proxy);

                    self.device_cache.insert(
                        if_name.clone(),
                        NetworkDeviceCacheEntry {
                            descriptor: NetworkDeviceDescriptor {
                                kind,
                                if_name: if_name.clone(),
                                adapter_name: adapter_name.clone(),
                            },
                            hw_address: hw_address.clone(),

                            tx_bytes,
                            rx_bytes,

                            update_timestamp: std::time::Instant::now(),
                        },
                    );

                    (
                        NetworkDeviceDescriptor {
                            kind,
                            if_name,
                            adapter_name,
                        },
                        hw_address,
                        0.,
                        0,
                        0.,
                        0,
                    )
                };

            let ip4_address = unsafe { Self::ip4_address(device_proxy) };
            let ip6_address = unsafe { Self::ip6_address(device_proxy) };

            let address = NetworkAddress {
                hw_address,
                ip4_address,
                ip6_address,
            };

            let wireless_info = if descriptor.kind == NetDeviceType::Wireless {
                unsafe { Self::wireless_info(device_proxy) }
            } else {
                None
            };

            unsafe { g_object_unref(device_proxy as _) };

            result.push(NetworkDevice {
                descriptor,
                address,
                wireless_info,

                send_bps,
                sent_bytes: send_bytes,
                recv_bps,
                recv_bytes: rec_bytes,

                max_speed,
            });
        }

        unsafe {
            if_nameindex::free(if_list);
        }

        result
    }

    fn device_kind(device_if: &str) -> NetDeviceType {
        if device_if.starts_with("bn") {
            NetDeviceType::Bluetooth
        } else if device_if.starts_with("br") || device_if.starts_with("virbr") {
            NetDeviceType::Bridge
        } else if device_if.starts_with("docker") {
            NetDeviceType::Docker
        } else if device_if.starts_with("eth") || device_if.starts_with("en") {
            NetDeviceType::Wired
        } else if device_if.starts_with("ib") {
            NetDeviceType::InfiniBand
        } else if device_if.starts_with("mp") {
            NetDeviceType::Multipass
        } else if device_if.starts_with("veth") {
            NetDeviceType::Virtual
        } else if device_if.starts_with("vpn") || device_if.starts_with("wg") {
            NetDeviceType::VPN
        } else if device_if.starts_with("wl") || device_if.starts_with("ww") {
            NetDeviceType::Wireless
        } else if device_if.starts_with("mlan") {
            let path = Path::new("/sys/class/net").join(device_if).join("wireless");
            if path.exists() {
                NetDeviceType::Wireless
            } else {
                NetDeviceType::Other
            }
        } else {
            NetDeviceType::Other
        }
    }

    fn hw_address(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<[u8; 6]> {
        if let Some(hw_address_variant) =
            unsafe { Self::nm_device_property(dbus_proxy, b"HwAddress\0") }
        {
            if let Some(hw_address_str) = hw_address_variant.str() {
                let mut hw_address = [0; 6];

                hw_address_str
                    .split(':')
                    .take(6)
                    .enumerate()
                    .map(|(i, s)| (i, u8::from_str_radix(s, 16).map_or(0, |v| v)))
                    .for_each(|(i, v)| hw_address[i] = v);

                Some(hw_address)
            } else {
                None
            }
        } else {
            None
        }
    }

    unsafe fn ip4_address(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<u32> {
        use gtk::glib::gobject_ffi::*;

        if let Some(ip4_address_obj_path) = Self::nm_device_property(dbus_proxy, b"Ip4Config\0") {
            if let Some(ip4_address_obj_path_str) = ip4_address_obj_path.str() {
                let ip4_config_proxy = Self::create_nm_dbus_proxy(
                    ip4_address_obj_path_str.as_bytes(),
                    b"org.freedesktop.NetworkManager.IP4Config\0",
                );
                if ip4_config_proxy.is_null() {
                    return None;
                }

                let result = if let Some(ip4_address_variant) =
                    Self::nm_device_property(ip4_config_proxy, b"Addresses\0")
                {
                    // Just take the first entry in the list of lists
                    if let Some(ip4_address_info) = ip4_address_variant.iter().next() {
                        // The first entry in the inner list is the IP address
                        ip4_address_info
                            .iter()
                            .next()
                            .map_or(None, |v| v.get::<u32>())
                    } else {
                        None
                    }
                } else {
                    None
                };

                g_object_unref(ip4_config_proxy as _);

                result
            } else {
                None
            }
        } else {
            None
        }
    }

    unsafe fn ip6_address(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<u128> {
        use gtk::glib::gobject_ffi::*;

        // The space for link-local addresses is fe80::/10
        const LL_PREFIX: u128 = 0xfe80 << 112;
        // Using this mask, we check if the first 10 bit are equal to the prefix above
        const LL_MASK: u128 = 0xffc0 << 112;

        if let Some(ip6_address_obj_path) = Self::nm_device_property(dbus_proxy, b"Ip6Config\0") {
            if let Some(ip6_address_obj_path_str) = ip6_address_obj_path.str() {
                let ip6_config_proxy = Self::create_nm_dbus_proxy(
                    ip6_address_obj_path_str.as_bytes(),
                    b"org.freedesktop.NetworkManager.IP6Config\0",
                );
                if ip6_config_proxy.is_null() {
                    return None;
                }

                let result = if let Some(ip6_address_variant) =
                    Self::nm_device_property(ip6_config_proxy, b"Addresses\0")
                {
                    let parsed_ips: Vec<u128> = ip6_address_variant
                        .iter()
                        .map(|ip6_with_mask| {
                            if let Some(ip6_bytes) = ip6_with_mask.iter().next() {
                                let mut ip6_address = [0; 16];
                                ip6_bytes.iter().enumerate().for_each(|(i, v)| {
                                    ip6_address[i] = v.get::<u8>().unwrap_or(0);
                                });

                                Some(u128::from_be_bytes(ip6_address))
                            } else {
                                None
                            }
                        })
                        .flatten()
                        .collect();

                    let first_non_ll = parsed_ips
                        .iter()
                        .filter(|ip| (*ip & LL_MASK) != LL_PREFIX)
                        .next()
                        .copied();

                    first_non_ll.or(parsed_ips.first().copied())
                } else {
                    None
                };

                g_object_unref(ip6_config_proxy as _);

                result
            } else {
                None
            }
        } else {
            None
        }
    }

    unsafe fn wireless_info(dbus_proxy: *mut gtk::gio::ffi::GDBusProxy) -> Option<WirelessInfo> {
        use gtk::{gio::ffi::*, glib::gobject_ffi::*};

        use std::ffi::CStr;

        let wireless_obj_path = CStr::from_ptr(g_dbus_proxy_get_object_path(dbus_proxy));

        let wireless_info_proxy = Self::create_nm_dbus_proxy(
            wireless_obj_path.to_bytes_with_nul(),
            b"org.freedesktop.NetworkManager.Device.Wireless\0",
        );
        if wireless_info_proxy.is_null() {
            return None;
        }

        let result = if let Some(wireless_info_variant) =
            Self::nm_device_property(wireless_info_proxy, b"ActiveAccessPoint\0")
        {
            if let Some(wireless_info_obj_path) = wireless_info_variant.str() {
                let wireless_info_proxy = Self::create_nm_dbus_proxy(
                    wireless_info_obj_path.as_bytes(),
                    b"org.freedesktop.NetworkManager.AccessPoint\0",
                );
                if wireless_info_proxy.is_null() {
                    return None;
                }

                let ssid = if let Some(ssid_variant) =
                    Self::nm_device_property(wireless_info_proxy, b"Ssid\0")
                {
                    let ssid = ssid_variant
                        .iter()
                        .filter_map(|v| v.get::<u8>())
                        .collect::<Vec<_>>();

                    String::from_utf8(ssid).ok()
                } else {
                    None
                };

                let frequency = if let Some(frequency) =
                    Self::nm_device_property(wireless_info_proxy, b"Frequency\0")
                {
                    frequency.get::<u32>()
                } else {
                    None
                };

                let bitrate = if let Some(bitrate) =
                    Self::nm_device_property(wireless_info_proxy, b"MaxBitrate\0")
                {
                    bitrate.get::<u32>()
                } else {
                    None
                };

                let signal_strength = if let Some(signal_strength) =
                    Self::nm_device_property(wireless_info_proxy, b"Strength\0")
                {
                    signal_strength.get::<u8>()
                } else {
                    None
                };

                g_object_unref(wireless_info_proxy as _);
                Some(WirelessInfo {
                    ssid,
                    frequency_mhz: frequency,
                    bitrate_kbps: bitrate,
                    signal_strength_percent: signal_strength,
                })
            } else {
                None
            }
        } else {
            None
        };

        g_object_unref(wireless_info_proxy as _);

        result
    }

    fn get_max_speed(if_name: &str) -> u64 {
        let speed = std::fs::read_to_string(format!("/sys/class/net/{}/speed", if_name));

        if let Ok(str) = speed {
            // Convert from megabits to bytes
            (str.trim().parse::<u64>().unwrap_or(0) / 8) * 1_000_000
        } else {
            0
        }
    }

    fn tx_rx_bytes(if_name: &str) -> (u64, u64) {
        let tx_bytes =
            std::fs::read_to_string(format!("/sys/class/net/{}/statistics/tx_bytes", if_name));
        let rx_bytes =
            std::fs::read_to_string(format!("/sys/class/net/{}/statistics/rx_bytes", if_name));

        let tx_bytes = if let Ok(str) = tx_bytes {
            str.trim().parse::<u64>().unwrap_or(0)
        } else {
            0
        };

        let rx_bytes = if let Ok(str) = rx_bytes {
            str.trim().parse::<u64>().unwrap_or(0)
        } else {
            0
        };

        (tx_bytes, rx_bytes)
    }

    fn device_name_from_hw_db(&mut self, udi: &str) -> Option<String> {
        use gtk::glib::*;
        use std::{fs::*, io::*, path::*};

        let device_name_cache = &mut self.device_name_cache;
        if let Some(device_name) = device_name_cache.get(udi) {
            return Some(device_name.clone());
        }

        let conn = match self.hwdb_conn.as_ref() {
            None => return None,
            Some(c) => c,
        };

        let mut stmt = match conn.prepare("SELECT value FROM key_len WHERE key = 'min'") {
            Ok(s) => s,
            Err(e) => {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to extract min key length from {}/hw.db: Prepare query failed: {}",
                    crate::HW_DB_DIR.as_str(),
                    e,
                );
                return None;
            }
        };
        let mut query_result = match stmt.query_map([], |row| row.get::<usize, i32>(0)) {
            Ok(qr) => qr,
            Err(e) => {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to extract min key length from {}/hw.db: Query map failed: {}",
                    crate::HW_DB_DIR.as_str(),
                    e,
                );
                return None;
            }
        };
        let min_key_len = if let Some(min_len) = query_result.next() {
            min_len.unwrap_or(0)
        } else {
            0
        };

        let mut stmt = match conn.prepare("SELECT value FROM key_len WHERE key = 'max'") {
            Ok(s) => s,
            Err(e) => {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to extract max key length from {}/hw.db: Prepare query failed: {}",
                    crate::HW_DB_DIR.as_str(),
                    e,
                );
                return None;
            }
        };
        let mut query_result = match stmt.query_map([], |row| row.get::<usize, i32>(0)) {
            Ok(qr) => qr,
            Err(e) => {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to extract max key length from {}/hw.db: Query map failed: {}",
                    crate::HW_DB_DIR.as_str(),
                    e,
                );
                return None;
            }
        };
        let mut max_key_len = if let Some(max_len) = query_result.next() {
            max_len.unwrap_or(i32::MAX)
        } else {
            i32::MAX
        };

        let device_id = format!("{}/device", udi);
        let mut sys_device_path = Path::new(&device_id);
        let mut modalias = String::new();
        for _ in 0..4 {
            if let Some(p) = sys_device_path.parent() {
                sys_device_path = p;
            } else {
                break;
            }

            let modalias_path = sys_device_path.join("modalias");
            if modalias_path.exists() {
                if let Ok(mut modalias_file) = File::options()
                    .create(false)
                    .read(true)
                    .write(false)
                    .open(modalias_path)
                {
                    modalias.clear();

                    if let Ok(_) = modalias_file.read_to_string(&mut modalias) {
                        modalias = modalias.trim().to_owned();
                        if max_key_len == i32::MAX {
                            max_key_len = modalias.len() as i32;
                        }

                        for i in (min_key_len..max_key_len).rev() {
                            modalias.truncate(i as usize);
                            let mut stmt = match conn.prepare(
                                "SELECT value FROM models WHERE key LIKE ?1 || '%' LIMIT 1",
                            ) {
                                Ok(s) => s,
                                Err(e) => {
                                    g_warning!(
                                        "MissionCenter::NetInfo",
                                        "Failed to find model in {}/hw.db: Prepare query failed: {}",
                                        crate::HW_DB_DIR.as_str(),
                                        e,
                                    );
                                    continue;
                                }
                            };
                            let mut query_result = match stmt
                                .query_map([modalias.trim()], |row| row.get::<usize, String>(0))
                            {
                                Ok(qr) => qr,
                                Err(e) => {
                                    g_warning!(
                                        "MissionCenter::NetInfo",
                                        "Failed to find model in {}/hw.db: Query map failed: {}",
                                        crate::HW_DB_DIR.as_str(),
                                        e,
                                    );
                                    continue;
                                }
                            };

                            let model_name = if let Some(model) = query_result.next() {
                                model.ok()
                            } else {
                                None
                            };

                            if let Some(model_name) = model_name {
                                let device_name_cache = &mut self.device_name_cache;
                                device_name_cache.insert(udi.to_owned(), model_name.clone());
                                return Some(model_name);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    unsafe fn adapter_name(
        &mut self,
        dbus_proxy: *mut gtk::gio::ffi::GDBusProxy,
    ) -> Option<String> {
        use errno_sys::errno_location;
        use gtk::glib::*;
        use libudev_sys::*;

        use std::ffi::CStr;

        if let Some(udi_variant) = Self::nm_device_property(dbus_proxy, b"Udi\0") {
            if let Some(udi) = udi_variant.str() {
                if let Some(device_name) = self.device_name_from_hw_db(udi) {
                    return Some(device_name);
                }

                let udev_device = udev_device_new_from_syspath(self.udev, udi.as_ptr() as _);
                if udev_device.is_null() {
                    let err = *errno_location();
                    let error_message = CStr::from_ptr(libc::strerror(err))
                        .to_str()
                        .map_or("Unknown error", |s| s)
                        .to_owned();

                    g_debug!(
                        "MissionCenter::NetInfo",
                        "Failed to create udev device from {:?}. {}",
                        udi,
                        error_message
                    );
                    return None;
                }

                let dev_name =
                    Self::get_udev_property(udev_device, b"ID_MODEL_ENC\0".as_ptr() as _);

                udev_device_unref(udev_device);

                dev_name
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to get udev device path, cannot extract device sys path from variant: Unknown error"
                );
                None
            }
        } else {
            None
        }
    }

    unsafe fn create_nm_dbus_proxy(
        path: &[u8],
        interface: &[u8],
    ) -> *mut gtk::gio::ffi::GDBusProxy {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, *};
        use std::ffi::CStr;

        let mut error: *mut GError = std::ptr::null_mut();

        let proxy = g_dbus_proxy_new_for_bus_sync(
            G_BUS_TYPE_SYSTEM,
            G_DBUS_PROXY_FLAGS_NONE,
            std::ptr::null_mut(),
            b"org.freedesktop.NetworkManager\0".as_ptr() as _,
            path.as_ptr() as _,
            interface.as_ptr() as _,
            std::ptr::null_mut(),
            &mut error,
        );
        if proxy.is_null() {
            if !error.is_null() {
                let error: Error = from_glib_full(error);
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to create dbus proxy for interface '{:?}': {}",
                    CStr::from_ptr(interface.as_ptr() as _),
                    error.message()
                );
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to create dbus proxy for interface '{:?}': Unknown error",
                    CStr::from_ptr(interface.as_ptr() as _),
                );
            }
        }

        proxy
    }

    unsafe fn nm_device_obj_path_new(
        &self,
        device_if: &std::ffi::CStr,
    ) -> Option<std::ffi::CString> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, *};
        use std::ffi::CStr;

        let mut error: *mut GError = std::ptr::null_mut();

        let device_path_variant = unsafe {
            g_dbus_proxy_call_sync(
                self.nm_proxy,
                b"GetDeviceByIpIface\0".as_ptr() as _,
                g_variant_new(b"(s)\0".as_ptr() as _, device_if.as_ptr()),
                G_DBUS_CALL_FLAGS_NONE,
                -1,
                std::ptr::null_mut(),
                &mut error,
            )
        };
        if device_path_variant.is_null() {
            if !error.is_null() {
                let error: Error = unsafe { from_glib_full(error) };
                g_debug!(
                    "MissionCenter::NetInfo",
                    "Failed to get device info for {:?}: {}",
                    device_if,
                    error.message()
                );
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to get device info for {:?}: Unknown error",
                    device_if,
                );
            }

            return None;
        }

        let mut device_path: *mut libc::c_char = std::ptr::null_mut();
        unsafe {
            g_variant_get(
                device_path_variant,
                b"(&o)\0".as_ptr() as _,
                &mut device_path,
            )
        };
        if device_path.is_null() {
            g_critical!(
                "MissionCenter::NetInfo",
                "Failed to get device info for {:?}: Variant error",
                device_if,
            );
            return None;
        }

        let device_path = CStr::from_ptr(device_path).to_owned();
        let _: Variant = from_glib_full(device_path_variant);

        Some(device_path)
    }

    unsafe fn nm_device_property(
        dbus_proxy: *mut gtk::gio::ffi::GDBusProxy,
        property: &[u8],
    ) -> Option<gtk::glib::Variant> {
        use gtk::gio::ffi::*;
        use gtk::glib::{ffi::*, translate::from_glib_full, *};
        use std::ffi::CStr;

        let mut error: *mut GError = std::ptr::null_mut();

        let variant = g_dbus_proxy_call_sync(
            dbus_proxy,
            b"org.freedesktop.DBus.Properties.Get\0".as_ptr() as _,
            g_variant_new(
                b"(ss)\0".as_ptr() as _,
                g_dbus_proxy_get_interface_name(dbus_proxy),
                property.as_ptr() as *const i8,
            ),
            G_DBUS_CALL_FLAGS_NONE,
            -1,
            std::ptr::null_mut(),
            &mut error,
        );
        if variant.is_null() {
            if !error.is_null() {
                let error: Error = from_glib_full(error);
                g_debug!(
                    "MissionCenter::NetInfo",
                    "Failed to get property {:?}: {}",
                    CStr::from_ptr(property.as_ptr() as _),
                    error.message()
                );
            } else {
                g_critical!(
                    "MissionCenter::NetInfo",
                    "Failed to get property {:?}: Unknown error",
                    CStr::from_ptr(property.as_ptr() as _),
                );
            }

            return None;
        }

        let mut inner: *mut GVariant = std::ptr::null_mut();
        g_variant_get(variant, b"(v)\0".as_ptr() as _, &mut inner);
        if inner.is_null() {
            g_variant_unref(variant);

            g_critical!(
                "MissionCenter::NetInfo",
                "Failed to get property {:?}, cannot extract inner variant: Unknown error",
                CStr::from_ptr(property.as_ptr() as _),
            );

            return None;
        }

        g_variant_ref_sink(inner);
        g_variant_unref(variant);

        from_glib_full(inner)
    }

    // Yanked from NetworkManager: src/libnm-client-impl/nm-device.c: _get_udev_property()
    unsafe fn get_udev_property(
        device: *mut libudev_sys::udev_device,
        property: *const libc::c_char,
    ) -> Option<String> {
        use libudev_sys::*;
        use std::ffi::CStr;

        let mut value: *const libc::c_char = std::ptr::null_mut();
        let mut tmpdev: *mut udev_device = device;

        let mut count = 0;
        while (count < 3) && !tmpdev.is_null() && value.is_null() {
            count += 1;

            if value.is_null() {
                value = udev_device_get_property_value(tmpdev, property);
            }

            tmpdev = udev_device_get_parent(tmpdev);
        }

        if !value.is_null() {
            CStr::from_ptr(value)
                .to_str()
                .map_or(None, |s| Some(s.to_owned()))
        } else {
            None
        }
    }
}
