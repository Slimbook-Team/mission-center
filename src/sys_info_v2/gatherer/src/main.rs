/* sys_info_v2/gatherer/src/main.rs
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

use std::sync::{
    atomic::{self, AtomicBool, AtomicU64},
    Arc, Mutex, PoisonError, RwLock,
};

use dbus::arg::RefArg;
use dbus::{arg, blocking::SyncConnection, channel::MatchingReceiver};
use dbus_crossroads::Crossroads;
use lazy_static::lazy_static;

use crate::platform::{FanInfo, FansInfo, FansInfoExt};
use logging::{critical, debug, error, message, warning};
use platform::{
    Apps, AppsExt, CpuDynamicInfo, CpuInfo, CpuInfoExt, CpuStaticInfo, CpuStaticInfoExt, DiskInfo,
    DisksInfo, DisksInfoExt, GpuDynamicInfo, GpuInfo, GpuInfoExt, GpuStaticInfo,
    PlatformUtilitiesExt, Processes, ProcessesExt, Service, ServiceController,
    ServiceControllerExt, Services, ServicesError, ServicesExt,
};

#[allow(unused_imports)]
mod logging;
mod platform;
mod utils;

const DBUS_OBJECT_PATH: &str = "/io/missioncenter/MissionCenter/Gatherer";

lazy_static! {
    static ref SYSTEM_STATE: SystemState<'static> = {
        let system_state = SystemState::new();

        let service_controller = system_state
            .services
            .read()
            .unwrap()
            .controller()
            .map(|sc| Some(sc))
            .unwrap_or_else(|e| {
                error!(
                    "Gatherer::Main",
                    "Failed to create service controller: {}", e
                );
                None
            });

        *system_state.service_controller.write().unwrap() = service_controller;

        system_state
            .cpu_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_static_info_cache();

        system_state.gpu_info.write().unwrap().refresh_gpu_list();
        system_state
            .gpu_info
            .write()
            .unwrap()
            .refresh_static_info_cache();

        system_state.snapshot();

        system_state
    };
    static ref LOGICAL_CPU_COUNT: u32 = {
        SYSTEM_STATE
            .cpu_info
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .static_info()
            .logical_cpu_count()
    };
}

#[derive(Debug)]
pub struct OrgFreedesktopDBusNameLost {
    pub arg0: String,
}

impl arg::AppendAll for OrgFreedesktopDBusNameLost {
    fn append(&self, i: &mut arg::IterAppend) {
        arg::RefArg::append(&self.arg0, i);
    }
}

impl arg::ReadAll for OrgFreedesktopDBusNameLost {
    fn read(i: &mut arg::Iter) -> Result<Self, arg::TypeMismatchError> {
        Ok(OrgFreedesktopDBusNameLost { arg0: i.read()? })
    }
}

impl dbus::message::SignalArgs for OrgFreedesktopDBusNameLost {
    const NAME: &'static str = "NameLost";
    const INTERFACE: &'static str = "org.freedesktop.DBus";
}

struct SystemState<'a> {
    cpu_info: Arc<RwLock<CpuInfo>>,
    disk_info: Arc<RwLock<DisksInfo>>,
    gpu_info: Arc<RwLock<GpuInfo>>,
    fan_info: Arc<RwLock<FansInfo>>,
    services: Arc<RwLock<Services<'a>>>,
    service_controller: Arc<RwLock<Option<ServiceController<'a>>>>,
    processes: Arc<RwLock<Processes>>,
    apps: Arc<RwLock<Apps>>,

    refresh_interval: Arc<AtomicU64>,
    core_count_affects_percentages: Arc<AtomicBool>,
}

impl SystemState<'_> {
    pub fn snapshot(&self) {
        {
            let mut processes = self
                .processes
                .write()
                .unwrap_or_else(PoisonError::into_inner);

            let timer = std::time::Instant::now();
            processes.refresh_cache();
            if !self
                .core_count_affects_percentages
                .load(atomic::Ordering::Relaxed)
            {
                let logical_cpu_count = *LOGICAL_CPU_COUNT as f32;
                for (_, p) in processes.process_list_mut() {
                    p.usage_stats.cpu_usage /= logical_cpu_count;
                }
            }
            debug!(
                "Gatherer::Perf",
                "Refreshed process cache in {:?}",
                timer.elapsed()
            );
        }

        let timer = std::time::Instant::now();
        self.cpu_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_dynamic_info_cache(
                &self
                    .processes
                    .read()
                    .unwrap_or_else(PoisonError::into_inner),
            );
        debug!(
            "Gatherer::Perf",
            "Refreshed CPU dynamic info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.disk_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache();
        debug!(
            "Gatherer::Perf",
            "Refreshed disk info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.gpu_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_dynamic_info_cache(
                &mut self
                    .processes
                    .write()
                    .unwrap_or_else(PoisonError::into_inner),
            );
        debug!(
            "Gatherer::Perf",
            "Refreshed GPU dynamic info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.fan_info
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache();
        debug!(
            "Gatherer::Perf",
            "Refreshed fan info cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.apps
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache(
                self.processes
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .process_list(),
            );
        debug!(
            "Gatherer::Perf",
            "Refreshed app cache in {:?}",
            timer.elapsed()
        );

        let timer = std::time::Instant::now();
        self.services
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .refresh_cache()
            .unwrap_or_else(|e| {
                debug!("Gatherer::Main", "Failed to refresh service cache: {}", e);
            });
        debug!(
            "Gatherer::Perf",
            "Refreshed service cache in {:?}",
            timer.elapsed()
        );
    }
}

impl<'a> SystemState<'a> {
    pub fn new() -> Self {
        Self {
            cpu_info: Arc::new(RwLock::new(CpuInfo::new())),
            disk_info: Arc::new(RwLock::new(DisksInfo::new())),
            gpu_info: Arc::new(RwLock::new(GpuInfo::new())),
            fan_info: Arc::new(RwLock::new(FansInfo::new())),
            services: Arc::new(RwLock::new(Services::new())),
            service_controller: Arc::new(RwLock::new(None)),
            processes: Arc::new(RwLock::new(Processes::new())),
            apps: Arc::new(RwLock::new(Apps::new())),

            refresh_interval: Arc::new(AtomicU64::new(1000)),
            core_count_affects_percentages: Arc::new(AtomicBool::new(true)),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Exit if any arguments are passed to this executable. This is done since the main app needs
    // to check if the executable can be run in its current environment (glibc or musl libc)
    for (i, _) in std::env::args().enumerate() {
        if i > 0 {
            eprintln!("👋");
            std::process::exit(0);
        }
    }

    #[cfg(target_os = "linux")]
    unsafe {
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
    }

    message!(
        "Gatherer::Main",
        "Starting v{}...",
        env!("CARGO_PKG_VERSION")
    );

    message!("Gatherer::Main", "Initializing system state...");
    let _ = &*SYSTEM_STATE;
    let _ = &*LOGICAL_CPU_COUNT;

    message!(
        "Gatherer::Main",
        "Setting up background data refresh thread..."
    );
    std::thread::spawn({
        move || loop {
            let refresh_interval = SYSTEM_STATE
                .refresh_interval
                .load(atomic::Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_millis(refresh_interval));

            SYSTEM_STATE.snapshot();
        }
    });

    message!("Gatherer::Main", "Initializing platform utilities...");
    let plat_utils = platform::PlatformUtilities::default();

    message!("Gatherer::Main", "Setting up connection to main app...");
    // Set up so that the Gatherer exists when the main app exits
    plat_utils.on_main_app_exit(Box::new(|| {
        message!("Gatherer::Main", "Parent process exited, exiting...");
        std::process::exit(0);
    }));

    message!("Gatherer::Main", "Setting up D-Bus connection...");
    let c = Arc::new(SyncConnection::new_session()?);

    message!("Gatherer::Main", "Requesting bus name...");
    c.request_name("io.missioncenter.MissionCenter.Gatherer", true, true, true)?;
    message!("Gatherer::Main", "Bus name acquired");

    message!("Gatherer::Main", "Setting up D-Bus proxy...");
    let proxy = c.with_proxy(
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        std::time::Duration::from_millis(5000),
    );

    message!("Gatherer::Main", "Setting up D-Bus signal match...");
    let _id = proxy.match_signal(
        |h: OrgFreedesktopDBusNameLost, _: &SyncConnection, _: &dbus::Message| {
            if h.arg0 != "io.missioncenter.MissionCenter.Gatherer" {
                return true;
            }
            message!("Gatherer::Main", "Bus name {} lost, exiting...", &h.arg0);
            std::process::exit(0);
        },
    )?;

    message!("Gatherer::Main", "Setting up D-Bus crossroads...");
    let mut cr = Crossroads::new();
    let iface_token = cr.register("io.missioncenter.MissionCenter.Gatherer", |builder| {
        message!(
            "Gatherer::Main",
            "Registering D-Bus properties and methods..."
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus property `RefreshInterval`..."
        );
        builder
            .property("RefreshInterval")
            .get_with_cr(|_, _| {
                Ok(SYSTEM_STATE
                    .refresh_interval
                    .load(atomic::Ordering::Relaxed))
            })
            .set_with_cr(|_, _, value| {
                if let Some(value) = value.as_u64() {
                    SYSTEM_STATE
                        .refresh_interval
                        .store(value, atomic::Ordering::Relaxed);
                    Ok(Some(value))
                } else {
                    Err(dbus::MethodErr::failed(&"Invalid value"))
                }
            });

        builder
            .property("CoreCountAffectsPercentages")
            .get_with_cr(|_, _| {
                Ok(SYSTEM_STATE
                    .core_count_affects_percentages
                    .load(atomic::Ordering::Relaxed))
            })
            .set_with_cr(|_, _, value| {
                if let Some(value) = value.as_u64() {
                    let value = value != 0;
                    SYSTEM_STATE
                        .core_count_affects_percentages
                        .store(value, atomic::Ordering::Relaxed);
                    Ok(Some(value))
                } else {
                    Err(dbus::MethodErr::failed(&"Invalid value"))
                }
            });

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetCPUStaticInfo`..."
        );
        builder.method_with_cr_custom::<(), (CpuStaticInfo,), &str, _>(
            "GetCPUStaticInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                    .cpu_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .static_info(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetCPUDynamicInfo`..."
        );
        builder.method_with_cr_custom::<(), (CpuDynamicInfo,), &str, _>(
            "GetCPUDynamicInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                    .cpu_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .dynamic_info(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetDisksInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<DiskInfo>,), &str, _>(
            "GetDisksInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                    .disk_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .info()
                    .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!("Gatherer::Main", "Registering D-Bus method `GetGPUList`...");
        builder.method_with_cr_custom::<(), (Vec<String>,), &str, _>(
            "GetGPUList",
            (),
            ("gpu_list",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                    .gpu_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .enumerate()
                    .map(|id| id.to_owned())
                    .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetGPUStaticInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<GpuStaticInfo>,), &str, _>(
            "GetGPUStaticInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                let gpu_info = SYSTEM_STATE
                    .gpu_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner);
                ctx.reply(Ok((gpu_info
                    .enumerate()
                    .map(|id| gpu_info.static_info(id).cloned().unwrap())
                    .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetGPUDynamicInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<GpuDynamicInfo>,), &str, _>(
            "GetGPUDynamicInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                let gpu_info = SYSTEM_STATE
                    .gpu_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner);
                ctx.reply(Ok((gpu_info
                    .enumerate()
                    .map(|id| gpu_info.dynamic_info(id).cloned().unwrap())
                    .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetFansInfo`..."
        );
        builder.method_with_cr_custom::<(), (Vec<FanInfo>,), &str, _>(
            "GetFansInfo",
            (),
            ("info",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                    .fan_info
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .info()
                    .collect::<Vec<_>>(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetProcesses`..."
        );
        builder.method_with_cr_custom::<(), (Processes,), &str, _>(
            "GetProcesses",
            (),
            ("process_list",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((&*SYSTEM_STATE
                    .processes
                    .write()
                    .unwrap_or_else(PoisonError::into_inner),)));

                Some(ctx)
            },
        );

        message!("Gatherer::Main", "Registering D-Bus method `GetApps`...");
        builder.method_with_cr_custom::<(), (Apps,), &str, _>(
            "GetApps",
            (),
            ("app_list",),
            move |mut ctx, _, (): ()| {
                ctx.reply(Ok((SYSTEM_STATE
                    .apps
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .app_list(),)));

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetServices`..."
        );
        builder.method_with_cr_custom::<(), (Vec<Service>,), &str, _>(
            "GetServices",
            (),
            ("service_list",),
            move |mut ctx, _, (): ()| {
                match SYSTEM_STATE
                    .services
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .services()
                {
                    Ok(s) => {
                        ctx.reply(Ok((s,)));
                    }
                    Err(e) => {
                        error!("Gatherer::Main", "Failed to get services: {}", e);
                        ctx.reply::<(Vec<Service>,)>(Ok((vec![],)));
                    }
                }

                Some(ctx)
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `TerminateProcess`..."
        );
        builder.method(
            "TerminateProcess",
            ("process_id",),
            (),
            move |_, _: &mut (), (pid,): (u32,)| {
                execute_no_reply(
                    SYSTEM_STATE.processes.clone(),
                    move |processes| -> Result<(), u8> { Ok(processes.terminate_process(pid)) },
                    "terminating process",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `KillProcess`..."
        );
        builder.method(
            "KillProcess",
            ("process_id",),
            (),
            move |_, _: &mut (), (pid,): (u32,)| {
                execute_no_reply(
                    SYSTEM_STATE.processes.clone(),
                    move |processes| -> Result<(), u8> { Ok(processes.kill_process(pid)) },
                    "terminating process",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `EnableService`..."
        );
        builder.method(
            "EnableService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service,): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.enable_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "enabling service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `DisableService`..."
        );
        builder.method(
            "DisableService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service,): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.disable_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "disabling service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `StartService`..."
        );
        builder.method(
            "StartService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service,): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.start_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "starting service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `StopService`..."
        );
        builder.method(
            "StopService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service,): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.stop_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "stopping service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `RestartService`..."
        );
        builder.method(
            "RestartService",
            ("service_name",),
            (),
            move |_, _: &mut (), (service,): (String,)| {
                execute_no_reply(
                    SYSTEM_STATE.service_controller.clone(),
                    move |sc| {
                        if let Some(sc) = sc.as_ref() {
                            sc.restart_service(&service)
                        } else {
                            Err(ServicesError::MissingServiceController)
                        }
                    },
                    "restarting service",
                )
            },
        );

        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetServiceLogs`..."
        );
        builder.method_with_cr_custom::<(String, u32), (String,), &str, _>(
            "GetServiceLogs",
            ("name", "pid"),
            ("service_list",),
            move |mut ctx, _, (name, pid): (String, u32)| {
                match SYSTEM_STATE
                    .services
                    .read()
                    .unwrap_or_else(PoisonError::into_inner)
                    .service_logs(&name, std::num::NonZeroU32::new(pid))
                {
                    Ok(s) => {
                        ctx.reply(Ok((s.as_ref().to_owned(),)));
                    }
                    Err(e) => {
                        ctx.reply(Result::<(Vec<Service>,), dbus::MethodErr>::Err(
                            dbus::MethodErr::failed::<String>(&format!(
                                "Failed to get service logs: {e}"
                            )),
                        ));
                    }
                }

                Some(ctx)
            },
        );
    });

    message!(
        "Gatherer::Main",
        "Registering D-Bus interface `org.freedesktop.DBus.Peer`..."
    );
    let peer_itf = cr.register("org.freedesktop.DBus.Peer", |builder| {
        message!(
            "Gatherer::Main",
            "Registering D-Bus method `GetMachineId`..."
        );
        builder.method("GetMachineId", (), ("machine_uuid",), |_, _, (): ()| {
            Ok((std::fs::read_to_string("/var/lib/dbus/machine-id")
                .map_or("UNKNOWN".into(), |s| s.trim().to_owned()),))
        });

        message!("Gatherer::Main", "Registering D-Bus method `Ping`...");
        builder.method("Ping", (), (), |_, _, (): ()| Ok(()));
    });

    message!(
        "Gatherer::Main",
        "Instantiating System and inserting it into Crossroads..."
    );
    cr.insert(DBUS_OBJECT_PATH, &[peer_itf, iface_token], ());

    message!("Gatherer::Main", "Creating thread pool...");
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build_global()?;

    message!("Gatherer::Main", "Serving D-Bus requests...");

    let cr = Arc::new(Mutex::new(cr));
    c.start_receive(dbus::message::MatchRule::new_method_call(), {
        Box::new(move |msg, conn| {
            cr.lock()
                .unwrap()
                .handle_message(msg, conn)
                .unwrap_or_else(|_| error!("Gatherer::Main", "Failed to handle message"));
            true
        })
    });

    loop {
        c.process(std::time::Duration::from_millis(1000))?;
    }
}

fn execute_no_reply<SF: Send + Sync + 'static, E: std::fmt::Display>(
    stats: Arc<RwLock<SF>>,
    command: impl FnOnce(&SF) -> Result<(), E> + Send + 'static,
    description: &'static str,
) -> Result<(), dbus::MethodErr> {
    rayon::spawn(move || {
        let stats = match stats.read() {
            Ok(s) => s,
            Err(poisoned_lock) => {
                warning!(
                    "Gatherer::Main",
                    "Lock poisoned while executing command for {}",
                    description
                );
                poisoned_lock.into_inner()
            }
        };

        if let Err(e) = command(&stats) {
            error!("Gatherer::Main", "Failed to execute command: {}", e);
        }
    });

    Ok(())
}
