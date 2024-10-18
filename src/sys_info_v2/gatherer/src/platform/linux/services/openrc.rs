pub use openrc::{OpenRCError, Service};

use crate::logging::error;

use super::super::openrc;

pub struct OpenRC {
    manager: openrc::ServiceManager,
    services: Vec<Service>,
}

impl OpenRC {
    pub fn new() -> Result<Self, OpenRCError> {
        Ok(OpenRC {
            manager: openrc::ServiceManager::new()?,
            services: Vec::new(),
        })
    }
}

pub type OpenRCController = openrc::Controller;

impl<'a> OpenRC {
    #[inline]
    pub fn refresh_cache(&mut self) -> Result<(), OpenRCError> {
        self.services = match self.manager.list_services() {
            Ok(services) => services,
            Err(e) => {
                error!("Gatherer::OpenRC", "Failed to list services: {}", &e);
                return Err(e);
            }
        };

        Ok(())
    }

    #[inline]
    pub fn services(&'a self) -> Result<Vec<Service>, OpenRCError> {
        Ok(self.services.clone())
    }

    pub fn controller(&self) -> Result<OpenRCController, OpenRCError> {
        Ok(self.manager.controller())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc_disable_enable_service() -> Result<(), OpenRCError> {
        let openrc = openrc::ServiceManager::new().unwrap();
        let controller = openrc.controller();

        let services = openrc.list_services().unwrap();
        assert!(!services.is_empty());

        let service = services.iter().find(|s| s.name.as_ref() == "sshd").unwrap();

        match controller.disable_service(service.name()) {
            Ok(_) => {
                println!("Service disabled successfully.");
            }
            Err(e) => {
                return Err(e);
            }
        }

        match controller.enable_service(service.name()) {
            Ok(_) => {
                println!("Service enabled successfully.");
            }
            Err(e) => {
                return Err(e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_rc_stop_start_service() -> Result<(), OpenRCError> {
        let openrc = openrc::ServiceManager::new().unwrap();
        let controller = openrc.controller();

        let services = openrc.list_services().unwrap();
        assert!(!services.is_empty());

        let service = services.iter().find(|s| s.name.as_ref() == "sshd").unwrap();

        match controller.stop_service(service.name()) {
            Ok(_) => {
                println!("Service stopped successfully.");
            }
            Err(e) => {
                return Err(e);
            }
        }

        match controller.start_service(service.name()) {
            Ok(_) => {
                println!("Service started successfully.");
            }
            Err(e) => {
                return Err(e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_rc_restart_service() -> Result<(), OpenRCError> {
        let openrc = openrc::ServiceManager::new().unwrap();
        let controller = openrc.controller();

        let services = openrc.list_services().unwrap();
        assert!(!services.is_empty());

        let service = services.iter().find(|s| s.name.as_ref() == "sshd").unwrap();

        match controller.restart_service(service.name()) {
            Ok(_) => {
                println!("Service restarted successfully.");
            }
            Err(e) => {
                return Err(e);
            }
        }

        Ok(())
    }
}
