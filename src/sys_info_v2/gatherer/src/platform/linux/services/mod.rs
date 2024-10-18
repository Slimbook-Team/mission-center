use std::{
    fmt::{Display, Formatter},
    num::NonZeroU32,
    sync::Arc,
    time::Duration,
};

use dbus::blocking::{stdintf::org_freedesktop_dbus::Peer, LocalConnection};

use crate::{
    logging::error,
    platform::{ServiceControllerExt, ServiceExt, ServicesExt},
};

mod openrc;
mod systemd;

#[derive(Debug)]
pub enum LinuxServicesError {
    UnsupportedServiceManager,
    DBusError(dbus::Error),
    TypeMismatchError(dbus::arg::TypeMismatchError),
    LibLoadingError(libloading::Error),
    MissingRunLevels,
    MissingServices,
    IoError(std::io::Error),
    JournalError(Arc<str>),
    CommandExecutionError(Arc<str>, i32),
    MissingServiceController,
}

impl Display for LinuxServicesError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LinuxServicesError::UnsupportedServiceManager => {
                write!(f, "Unsupported service manager")
            }
            LinuxServicesError::DBusError(e) => {
                write!(f, "DBus error: {}", e)
            }
            LinuxServicesError::TypeMismatchError(e) => {
                write!(f, "Type mismatch error: {}", e)
            }
            LinuxServicesError::LibLoadingError(e) => {
                write!(f, "Library loading error: {}", e)
            }
            LinuxServicesError::MissingRunLevels => {
                write!(f, "Missing run levels")
            }
            LinuxServicesError::MissingServices => {
                write!(f, "Missing services")
            }
            LinuxServicesError::IoError(e) => {
                write!(f, "IO error: {}", e)
            }
            LinuxServicesError::CommandExecutionError(stderr, exit_code) => {
                write!(
                    f,
                    "Command execution error: {} (exit code: {})",
                    stderr, exit_code
                )
            }
            LinuxServicesError::MissingServiceController => {
                write!(f, "Missing service controller")
            }
            LinuxServicesError::JournalError(e) => {
                write!(f, "Journal error: {}", e)
            }
        }
    }
}

impl From<systemd::SystemDError> for LinuxServicesError {
    fn from(value: systemd::SystemDError) -> Self {
        match value {
            systemd::SystemDError::DBusError(e) => Self::DBusError(e),
            systemd::SystemDError::TypeMismatchError(e) => Self::TypeMismatchError(e),
            systemd::SystemDError::LibLoadingError(e) => Self::LibLoadingError(e),
            systemd::SystemDError::IoError(e) => Self::IoError(e),
            systemd::SystemDError::JournalOpenError(e) => Self::JournalError(e),
            systemd::SystemDError::JournalSeekError(e) => Self::JournalError(e),
            systemd::SystemDError::JournalAddMatchError(e) => Self::JournalError(e),
            systemd::SystemDError::JournalAddDisjunctionError(e) => Self::JournalError(e),
            systemd::SystemDError::JournalAddConjunctionError(e) => Self::JournalError(e),
            systemd::SystemDError::JournalIterateError(e) => Self::JournalError(e),
        }
    }
}

impl From<openrc::OpenRCError> for LinuxServicesError {
    fn from(value: openrc::OpenRCError) -> Self {
        match value {
            openrc::OpenRCError::LibLoadingError(e) => Self::LibLoadingError(e),
            openrc::OpenRCError::MissingRunLevels => Self::MissingRunLevels,
            openrc::OpenRCError::MissingServices => Self::MissingServices,
            openrc::OpenRCError::IoError(e) => Self::IoError(e),
            openrc::OpenRCError::CommandExecutionError(err, exit_code) => {
                Self::CommandExecutionError(err, exit_code)
            }
        }
    }
}

#[derive(Debug)]
pub enum LinuxService {
    SystemD(systemd::Service),
    OpenRC(openrc::Service),
}

pub enum LinuxServiceController<'a> {
    SystemD(systemd::SystemDController<'a>),
    OpenRC(openrc::OpenRCController),
}

pub enum LinuxServices<'a> {
    Unimplemented,
    SystemD(systemd::SystemD<'a>),
    OpenRC(openrc::OpenRC),
}

impl ServiceExt for LinuxService {
    fn name(&self) -> &str {
        match self {
            LinuxService::SystemD(s) => s.name(),
            LinuxService::OpenRC(s) => s.name(),
        }
    }

    fn description(&self) -> &str {
        match self {
            LinuxService::SystemD(s) => s.description(),
            LinuxService::OpenRC(s) => s.description(),
        }
    }

    fn enabled(&self) -> bool {
        match self {
            LinuxService::SystemD(s) => s.enabled(),
            LinuxService::OpenRC(s) => s.enabled(),
        }
    }

    fn running(&self) -> bool {
        match self {
            LinuxService::SystemD(s) => s.running(),
            LinuxService::OpenRC(s) => s.running(),
        }
    }

    fn failed(&self) -> bool {
        match self {
            LinuxService::SystemD(s) => s.failed(),
            LinuxService::OpenRC(s) => s.failed(),
        }
    }

    fn pid(&self) -> Option<NonZeroU32> {
        match self {
            LinuxService::SystemD(s) => s.pid(),
            LinuxService::OpenRC(s) => s.pid(),
        }
    }

    fn user(&self) -> Option<&str> {
        match self {
            LinuxService::SystemD(s) => s.user(),
            LinuxService::OpenRC(s) => s.user(),
        }
    }

    fn group(&self) -> Option<&str> {
        match self {
            LinuxService::SystemD(s) => s.group(),
            LinuxService::OpenRC(s) => s.group(),
        }
    }
}

impl LinuxServices<'_> {
    pub fn new() -> Self {
        fn systemd_available() -> bool {
            // If we're a Snap we can't access the Ping method on the org.freedesktop.DBus interface
            // so fall back to checking if a standard SystemD specific path exists
            if let Some(_) = std::env::var_os("SNAP_CONTEXT") {
                return std::path::Path::new("/lib/systemd/systemd").exists();
            }

            let connection = match LocalConnection::new_system() {
                Ok(c) => c,
                Err(_) => {
                    return false;
                }
            };

            let systemd1 = connection.with_proxy(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                Duration::from_millis(30_000),
            );

            systemd1.ping().is_ok()
        }

        let librc_exists = std::path::Path::new("/lib/librc.so.1").exists()
            || std::path::Path::new("/lib64/librc.so.1").exists();

        if librc_exists && std::path::Path::new("/sbin/rc-service").exists() {
            match openrc::OpenRC::new() {
                Ok(openrc) => LinuxServices::OpenRC(openrc),
                Err(e) => {
                    error!(
                        "Gatherer::ServiceManager",
                        "Failed to initialize OpenRC: {}", e
                    );
                    LinuxServices::Unimplemented
                }
            }
        } else if systemd_available() {
            match systemd::SystemD::new() {
                Ok(systemd) => LinuxServices::SystemD(systemd),
                Err(e) => {
                    error!(
                        "Gatherer::ServiceManager",
                        "Failed to initialize SystemD: {}", e
                    );
                    LinuxServices::Unimplemented
                }
            }
        } else {
            LinuxServices::Unimplemented
        }
    }
}

impl<'a> ServiceControllerExt for LinuxServiceController<'a> {
    type E = LinuxServicesError;

    fn enable_service(&self, name: &str) -> Result<(), LinuxServicesError> {
        match self {
            LinuxServiceController::SystemD(s) => s.enable_service(name).map_err(|e| e.into()),
            LinuxServiceController::OpenRC(s) => s.enable_service(name).map_err(|e| e.into()),
        }
    }

    fn disable_service(&self, name: &str) -> Result<(), LinuxServicesError> {
        match self {
            LinuxServiceController::SystemD(s) => s.disable_service(name).map_err(|e| e.into()),
            LinuxServiceController::OpenRC(s) => s.disable_service(name).map_err(|e| e.into()),
        }
    }

    fn start_service(&self, name: &str) -> Result<(), LinuxServicesError> {
        match self {
            LinuxServiceController::SystemD(s) => s.start_service(name).map_err(|e| e.into()),
            LinuxServiceController::OpenRC(s) => s.start_service(name).map_err(|e| e.into()),
        }
    }

    fn stop_service(&self, name: &str) -> Result<(), LinuxServicesError> {
        match self {
            LinuxServiceController::SystemD(s) => s.stop_service(name).map_err(|e| e.into()),
            LinuxServiceController::OpenRC(s) => s.stop_service(name).map_err(|e| e.into()),
        }
    }

    fn restart_service(&self, name: &str) -> Result<(), LinuxServicesError> {
        match self {
            LinuxServiceController::SystemD(s) => s.restart_service(name).map_err(|e| e.into()),
            LinuxServiceController::OpenRC(s) => s.restart_service(name).map_err(|e| e.into()),
        }
    }
}

impl<'a> ServicesExt<'a> for LinuxServices<'a> {
    type S = LinuxService;
    type C = LinuxServiceController<'a>;
    type E = LinuxServicesError;

    fn refresh_cache(&mut self) -> Result<(), LinuxServicesError> {
        match self {
            LinuxServices::Unimplemented => Err(LinuxServicesError::UnsupportedServiceManager),
            LinuxServices::SystemD(s) => s.refresh_cache().map_err(|e| e.into()),
            LinuxServices::OpenRC(s) => s.refresh_cache().map_err(|e| e.into()),
        }
    }

    fn services(&'a self) -> Result<Vec<LinuxService>, LinuxServicesError> {
        match self {
            LinuxServices::Unimplemented => Err(LinuxServicesError::UnsupportedServiceManager),
            LinuxServices::SystemD(s) => Ok(s
                .services()?
                .iter()
                .map(|s| LinuxService::SystemD(s.clone()))
                .collect()),
            LinuxServices::OpenRC(s) => Ok(s
                .services()?
                .iter()
                .map(|s| LinuxService::OpenRC(s.clone()))
                .collect()),
        }
    }

    fn controller(&self) -> Result<LinuxServiceController<'a>, LinuxServicesError> {
        match self {
            LinuxServices::Unimplemented => Err(LinuxServicesError::UnsupportedServiceManager),
            LinuxServices::SystemD(s) => Ok(LinuxServiceController::SystemD(s.controller()?)),
            LinuxServices::OpenRC(s) => Ok(LinuxServiceController::OpenRC(s.controller()?)),
        }
    }

    fn service_logs(&self, name: &str, pid: Option<NonZeroU32>) -> Result<Arc<str>, Self::E> {
        match self {
            LinuxServices::Unimplemented => Err(LinuxServicesError::UnsupportedServiceManager),
            LinuxServices::SystemD(s) => s.service_logs(name, pid).map_err(|e| e.into()),
            LinuxServices::OpenRC(_) => Ok(Arc::<str>::from("")),
        }
    }
}
