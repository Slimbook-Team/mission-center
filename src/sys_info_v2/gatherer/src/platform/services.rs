/* sys_info_v2/gatherer/src/platform/services.rs
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

use std::num::NonZeroU32;
use std::sync::Arc;

use dbus::{
    arg::{Append, Arg, ArgType, IterAppend},
    Signature,
};

/// High-level description of a service
pub trait ServiceExt: Append + Arg {
    /// The name of the service, also used as the unique identifier for the service
    fn name(&self) -> &str;

    /// A human-readable description of the service
    fn description(&self) -> &str;

    /// Whether the service is enabled to start at boot
    fn enabled(&self) -> bool;

    /// Whether the service is currently running
    fn running(&self) -> bool;

    /// If the service isn't running did it finish successfully
    fn failed(&self) -> bool;

    /// The process id of the service
    fn pid(&self) -> Option<NonZeroU32>;

    /// The user that the service runs as
    fn user(&self) -> Option<&str>;

    /// The group that the service runs as
    fn group(&self) -> Option<&str>;
}

impl Append for crate::platform::Service {
    #[inline]
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append((
            self.name(),
            self.description(),
            self.enabled(),
            self.running(),
            self.failed(),
            self.pid().map(|pid| pid.get()).unwrap_or(0),
            self.user().unwrap_or(""),
            self.group().unwrap_or(""),
        ))
    }
}

impl Arg for crate::platform::Service {
    const ARG_TYPE: ArgType = ArgType::Struct;

    #[inline]
    fn signature() -> Signature<'static> {
        Signature::from("(ssbbbuss)")
    }
}

/// An object that can control services and their state
pub trait ServiceControllerExt {
    type E;

    /// Enable a service to start at boot
    fn enable_service(&self, name: &str) -> Result<(), Self::E>;

    /// Disable a service from starting at boot
    fn disable_service(&self, name: &str) -> Result<(), Self::E>;

    /// Start a service
    fn start_service(&self, name: &str) -> Result<(), Self::E>;

    /// Stop a service
    fn stop_service(&self, name: &str) -> Result<(), Self::E>;

    /// Restart a service
    fn restart_service(&self, name: &str) -> Result<(), Self::E>;
}

/// The public interface that describes how the list of running processes is obtained
pub trait ServicesExt<'a> {
    type S: ServiceExt;
    type C: ServiceControllerExt<E = Self::E>;
    type E;

    /// Refreshes the internal service cache
    ///
    /// It is expected that implementors of this trait cache the list once obtained from
    /// the underlying OS
    fn refresh_cache(&mut self) -> Result<(), Self::E>;

    /// Return the (cached) list of services
    fn services(&'a self) -> Result<Vec<Self::S>, Self::E>;

    /// An instance of a service controller
    fn controller(&self) -> Result<Self::C, Self::E>;

    /// The logs of a service
    fn service_logs(&self, name: &str, pid: Option<NonZeroU32>) -> Result<Arc<str>, Self::E>;
}
