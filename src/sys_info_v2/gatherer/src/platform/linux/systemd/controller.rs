use std::sync::Arc;
use std::time::Duration;

use dbus::blocking::{Proxy, SyncConnection};

use crate::logging::error;

use super::{dbus_call_forget, dbus_call_wait_reply, SystemDError};

pub struct Controller<'a> {
    _connection: Arc<SyncConnection>,
    proxy: Proxy<'a, &'a SyncConnection>,
}

impl<'a> Controller<'a> {
    pub fn new(connection: Arc<SyncConnection>, proxy: Proxy<'a, &'a SyncConnection>) -> Self {
        Self {
            _connection: connection,
            proxy,
        }
    }
}

impl<'a> Controller<'a> {
    pub fn enable_service<'i, 'm>(&self, service: &str) -> Result<(), SystemDError> {
        rayon::spawn({
            let service = service.to_string();
            move || {
                let Ok(connection) = SyncConnection::new_system() else {
                    error!(
                        "Gatherer::SystemD",
                        "Failed to create connection to system bus"
                    );
                    return;
                };

                let proxy = connection.with_proxy(
                    "org.freedesktop.systemd1",
                    "/org/freedesktop/systemd1",
                    Duration::from_millis(30_000),
                );

                let r: Result<(bool, Vec<(String, String, String)>), dbus::Error> =
                    dbus_call_wait_reply(
                        &proxy,
                        "org.freedesktop.systemd1.Manager",
                        "EnableUnitFiles",
                        (vec![service.as_str()], false, true),
                    );
                if let Err(e) = r {
                    error!("Gatherer::SystemD", "Failed to enable service: {}", e);
                    return;
                }

                let r: Result<(), dbus::Error> =
                    dbus_call_wait_reply(&proxy, "org.freedesktop.systemd1.Manager", "Reload", ());
                if let Err(e) = r {
                    error!(
                        "Gatherer::SystemD",
                        "Failed to reload Systemd daemon: {}", e
                    );
                    return;
                }
            }
        });

        Ok(())
    }

    pub fn disable_service<'i, 'm>(&self, service: &str) -> Result<(), SystemDError> {
        rayon::spawn({
            let service = service.to_string();
            move || {
                let Ok(connection) = SyncConnection::new_system() else {
                    error!(
                        "Gatherer::SystemD",
                        "Failed to create connection to system bus"
                    );
                    return;
                };

                let proxy = connection.with_proxy(
                    "org.freedesktop.systemd1",
                    "/org/freedesktop/systemd1",
                    Duration::from_millis(30_000),
                );

                let r: Result<(Vec<(String, String, String)>,), dbus::Error> = dbus_call_wait_reply(
                    &proxy,
                    "org.freedesktop.systemd1.Manager",
                    "DisableUnitFiles",
                    (vec![service], false),
                );
                if let Err(e) = r {
                    error!("Gatherer::SystemD", "Failed to disable service: {}", e);
                    return;
                }

                let r: Result<(), dbus::Error> =
                    dbus_call_wait_reply(&proxy, "org.freedesktop.systemd1.Manager", "Reload", ());
                if let Err(e) = r {
                    error!(
                        "Gatherer::SystemD",
                        "Failed to reload Systemd daemon: {}", e
                    );
                    return;
                }
            }
        });

        Ok(())
    }

    pub fn start_service(&self, service: &str) -> Result<(), SystemDError> {
        dbus_call_forget(
            &self.proxy,
            "org.freedesktop.systemd1.Manager",
            "StartUnit",
            (service, "fail"),
        )?;

        Ok(())
    }

    pub fn stop_service(&self, service: &str) -> Result<(), SystemDError> {
        dbus_call_forget(
            &self.proxy,
            "org.freedesktop.systemd1.Manager",
            "StopUnit",
            (service, "replace"),
        )?;

        Ok(())
    }

    pub fn restart_service(&self, service: &str) -> Result<(), SystemDError> {
        dbus_call_forget(
            &self.proxy,
            "org.freedesktop.systemd1.Manager",
            "RestartUnit",
            (service, "fail"),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::ServiceManager;
    use super::*;

    #[test]
    fn test_disable_enable_service() -> Result<(), anyhow::Error> {
        let svc_mgr = ServiceManager::new()?;
        let controller = svc_mgr.controller()?;
        let services = svc_mgr.list_services()?;
        assert!(!services.is_empty());

        let service = services
            .iter()
            .find(|s| s.name.as_ref() == "NetworkManager.service")
            .unwrap();

        eprintln!("{:?}", std::env::args());

        let nm_service_path = std::path::Path::new(
            "/etc/systemd/system/multi-user.target.wants/NetworkManager.service",
        );

        controller.disable_service(service.name())?;
        assert!(!nm_service_path.exists());

        controller.enable_service(service.name())?;
        std::thread::sleep(Duration::from_secs(1));
        assert!(nm_service_path.exists());

        Ok(())
    }

    #[test]
    fn test_stop_start_service() -> Result<(), anyhow::Error> {
        let svc_mgr = ServiceManager::new()?;
        let controller = svc_mgr.controller()?;
        let services = svc_mgr.list_services()?;
        assert!(!services.is_empty());

        let service = services
            .iter()
            .find(|s| s.name.as_ref() == "NetworkManager.service")
            .unwrap();

        controller.stop_service(service.name())?;
        unsafe {
            libc::sleep(5);
        }
        controller.start_service(service.name())?;

        Ok(())
    }

    #[test]
    fn test_restart_service() -> Result<(), anyhow::Error> {
        let svc_mgr = ServiceManager::new()?;
        let controller = svc_mgr.controller()?;
        let services = svc_mgr.list_services()?;
        assert!(!services.is_empty());

        let service = services
            .iter()
            .find(|s| s.name.as_ref() == "NetworkManager.service")
            .unwrap();

        controller.restart_service(service.name())?;

        Ok(())
    }
}
