use std::num::NonZeroU32;
use std::sync::Arc;

use crate::platform::platform_impl::openrc;

#[derive(Debug, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum State {
    RcServiceStopped = 0x0001,
    RcServiceStarted = 0x0002,
    RcServiceStopping = 0x0004,
    RcServiceStarting = 0x0008,
    RcServiceInactive = 0x0010,

    /* Service may or may not have been hotplugged */
    RcServiceHotplugged = 0x0100,

    /* Optional states service could also be in */
    RcServiceFailed = 0x0200,
    RcServiceScheduled = 0x0400,
    RcServiceWasinactive = 0x0800,
    RcServiceCrashed = 0x1000,
}

impl From<State> for u32 {
    fn from(state: State) -> u32 {
        state as u32
    }
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub runlevel: Arc<str>,
    pub state: State,
    pub pid: u32,
}

impl Service {
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[inline]
    pub fn description(&self) -> &str {
        self.description.as_ref()
    }

    #[inline]
    pub fn enabled(&self) -> bool {
        !self.runlevel.as_ref().is_empty()
    }

    #[inline]
    pub fn running(&self) -> bool {
        self.state == openrc::State::RcServiceStarted
    }

    #[inline]
    pub fn failed(&self) -> bool {
        self.state == openrc::State::RcServiceFailed
    }

    #[inline]
    pub fn pid(&self) -> Option<NonZeroU32> {
        match self.pid {
            0 => None,
            _ => NonZeroU32::new(self.pid),
        }
    }

    #[inline]
    pub fn user(&self) -> Option<&str> {
        Some("root")
    }

    #[inline]
    pub fn group(&self) -> Option<&str> {
        Some("root")
    }
}
