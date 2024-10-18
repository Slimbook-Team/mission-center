/* sys_info_v2/gatherer/src/platform/disk_info.rs
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

use dbus::arg::IterAppend;
use dbus::{
    arg::{Append, Arg, ArgType},
    Signature,
};

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum DiskType {
    Unknown = 0,
    HDD,
    SSD,
    NVMe,
    eMMC,
    SD,
    iSCSI,
    Optical,
}

impl Default for DiskType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Describes the static (unchanging) information about a physical disk
pub trait DiskInfoExt: Default + Append + Arg {
    /// The disk's unique identifier
    fn id(&self) -> &str;

    /// The disk's model in human-readable form
    fn model(&self) -> &str;

    /// The disk's type
    fn r#type(&self) -> DiskType;

    /// The disk's capacity in bytes
    fn capacity(&self) -> u64;

    /// The disk's formatted capacity in bytes
    fn formatted(&self) -> u64;

    /// Check if the disk is the system disk
    fn is_system_disk(&self) -> bool;

    /// The disk's busy percentage
    fn busy_percent(&self) -> f32;

    /// The disk's response time in milliseconds
    fn response_time_ms(&self) -> f32;

    /// The disk's read speed in bytes per second
    fn read_speed(&self) -> u64;

    /// The disk's write speed in bytes per second
    fn write_speed(&self) -> u64;
}

impl Arg for crate::platform::DiskInfo {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ssyttbddtt)")
    }
}

impl Append for crate::platform::DiskInfo {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append((
            self.id(),
            self.model(),
            self.r#type() as u8,
            self.capacity(),
            self.formatted(),
            self.is_system_disk(),
            self.busy_percent() as f64,
            self.response_time_ms() as f64,
            self.read_speed(),
            self.write_speed(),
        ));
    }
}

impl Append for crate::platform::DiskInfoIter<'_> {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append_array(&crate::platform::DiskInfo::signature(), |a| {
            for v in self.0.clone() {
                a.append(v);
            }
        });
    }
}

/// Provides an interface for gathering disk information
pub trait DisksInfoExt<'a> {
    type S: DiskInfoExt;
    type Iter: Iterator<Item = &'a Self::S>
    where
        <Self as DisksInfoExt<'a>>::S: 'a;

    /// Refresh the internal information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_cache(&mut self);

    /// Returns the static information for the disks present in the system.
    fn info(&'a self) -> Self::Iter;
}
