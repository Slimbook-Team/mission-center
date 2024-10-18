/* sys_info_v2/gatherer/src/platform/fan_info.rs
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

/// Describes the static (unchanging) information about a physical fan
pub trait FanInfoExt: Default + Append + Arg {
    /// The fan's identifier
    fn fan_label(&self) -> &str;

    /// The temp that the fan is meant to combat
    fn temp_name(&self) -> &str;

    /// The fan's temperature in mC (milli celcius)
    fn temp_amount(&self) -> i64;

    /// The fan's sped in rpm
    fn rpm(&self) -> u64;

    /// Commanded speed in pwm percent
    fn percent_vroomimg(&self) -> f32;

    /// The fan's index in its hwmon
    fn fan_index(&self) -> u64;

    /// The hwmon this fan is in
    fn hwmon_index(&self) -> u64;

    /// The max rpm reported by the system, 0 if none reported
    fn max_speed(&self) -> u64;
}

impl Arg for crate::platform::FanInfo {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("(ssxtdttt)")
    }
}

impl Append for crate::platform::FanInfo {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append((
            self.fan_label(),
            self.temp_name(),
            self.temp_amount(),
            self.rpm(),
            self.percent_vroomimg() as f64,
            self.fan_index(),
            self.hwmon_index(),
            self.max_speed(),
        ));
    }
}

impl Append for crate::platform::FanInfoIter<'_> {
    fn append_by_ref(&self, ia: &mut IterAppend) {
        ia.append_array(&crate::platform::FanInfo::signature(), |a| {
            for v in self.0.clone() {
                a.append(v);
            }
        });
    }
}

/// Provides an interface for gathering fan information
pub trait FansInfoExt<'a> {
    type S: FanInfoExt;
    type Iter: Iterator<Item = &'a Self::S>
    where
        <Self as FansInfoExt<'a>>::S: 'a;

    /// Refresh the internal information cache
    ///
    /// It is expected that implementors of this trait cache this information, once obtained
    /// from the underlying OS
    fn refresh_cache(&mut self);

    /// Returns the static information for the fans present in the system.
    fn info(&'a self) -> Self::Iter;
}
