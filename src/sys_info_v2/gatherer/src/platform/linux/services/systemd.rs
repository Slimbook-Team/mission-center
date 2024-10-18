use std::num::NonZeroU32;
use std::sync::Arc;

pub use systemd::{Service, SystemDError};

use crate::logging::error;

use super::super::systemd;

pub struct SystemD<'a> {
    manager: systemd::ServiceManager<'a>,
    services: Vec<Service>,
}

impl<'a> SystemD<'a> {
    pub fn new() -> Result<Self, SystemDError> {
        Ok(SystemD {
            manager: systemd::ServiceManager::new()?,
            services: Vec::new(),
        })
    }
}

pub type SystemDController<'a> = systemd::Controller<'a>;

impl<'a> SystemD<'a> {
    #[inline]
    pub fn refresh_cache(&mut self) -> Result<(), SystemDError> {
        self.services = match self.manager.list_services() {
            Ok(services) => services,
            Err(e) => {
                error!("Gatherer::SystemD", "Failed to list services: {}", &e);
                return Err(e);
            }
        };

        Ok(())
    }

    #[inline]
    pub fn services(&'a self) -> Result<Vec<Service>, SystemDError> {
        Ok(self.services.clone())
    }

    pub fn controller(&self) -> Result<SystemDController<'a>, SystemDError> {
        self.manager.controller()
    }

    pub fn service_logs(
        &self,
        name: &str,
        pid: Option<NonZeroU32>,
    ) -> Result<Arc<str>, SystemDError> {
        self.manager.service_logs(name, pid)
    }
}
