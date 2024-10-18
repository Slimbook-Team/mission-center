/* sys_info_v2/gatherer/src/platform/linux/apps.rs
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
use std::path::PathBuf;
use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::platform::apps::*;
use crate::platform::ProcessExt;

use super::{INITIAL_REFRESH_TS, MIN_DELTA_REFRESH};

const APP_IGNORELIST: &[&str] = &[
    "guake-prefs",
    "org.codeberg.dnkl.foot-server",
    "org.codeberg.dnkl.footclient",
];

type LinuxProcess = crate::platform::Process;

#[derive(Debug, Clone)]
pub struct LinuxApp {
    pub name: Arc<str>,
    pub icon: Option<Arc<str>>,
    pub id: Arc<str>,
    pub command: Arc<str>,
    pub pids: Vec<u32>,
}

impl Default for LinuxApp {
    fn default() -> Self {
        let empty_arc = Arc::<str>::from("");
        Self {
            name: empty_arc.clone(),
            icon: None,
            id: empty_arc.clone(),
            command: empty_arc.clone(),
            pids: vec![],
        }
    }
}

impl<'a> AppExt<'a> for LinuxApp {
    type Iter = core::slice::Iter<'a, u32>;

    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn icon(&self) -> Option<&str> {
        self.icon.as_ref().map(|s| s.as_ref())
    }

    fn id(&self) -> &str {
        self.id.as_ref()
    }

    fn command(&self) -> &str {
        self.command.as_ref()
    }

    fn pids(&'a self) -> Self::Iter {
        self.pids.iter()
    }
}

pub struct LinuxApps {
    app_cache: Vec<LinuxApp>,

    refresh_timestamp: Instant,
}

impl Default for LinuxApps {
    fn default() -> Self {
        Self {
            app_cache: vec![],
            refresh_timestamp: *INITIAL_REFRESH_TS,
        }
    }
}

impl LinuxApps {
    pub fn new() -> Self {
        Default::default()
    }
}

impl app_rummage::Process for LinuxProcess {
    fn pid(&self) -> NonZeroU32 {
        NonZeroU32::new(<LinuxProcess as ProcessExt>::pid(self)).unwrap()
    }

    fn executable_path(&self) -> Option<PathBuf> {
        if self.exe().is_empty() {
            return None;
        }

        Some(PathBuf::from(self.exe()))
    }

    fn name(&self) -> &str {
        <LinuxProcess as ProcessExt>::name(self)
    }
}

impl<'a> AppsExt<'a> for LinuxApps {
    type A = LinuxApp;
    type P = LinuxProcess;

    fn refresh_cache(&mut self, processes: &HashMap<u32, LinuxProcess>) {
        let now = Instant::now();
        if now.duration_since(self.refresh_timestamp) < MIN_DELTA_REFRESH {
            return;
        }
        self.refresh_timestamp = now;

        let empty_string: Arc<str> = Arc::from("");

        let mut installed_apps = app_rummage::installed_apps();
        for app in APP_IGNORELIST {
            installed_apps.remove(*app);
        }

        self.app_cache = app_rummage::running_apps(&installed_apps, processes.values())
            .drain(..)
            .map(|(app, mut pids)| LinuxApp {
                name: Arc::from(app.name.as_ref()),
                icon: app.icon.as_ref().map(|icon| {
                    let icon = icon.as_ref();
                    // We can't access `/snap` when packaged as a Snap, we can go through the hostfs though.
                    // So update the icon path to reflect this change.
                    if let Some(_) = std::env::var_os("SNAP_CONTEXT") {
                        if icon.starts_with("/snap") {
                            Arc::from(format!("{}{}", "/var/lib/snapd/hostfs", icon))
                        } else {
                            Arc::from(icon)
                        }
                    } else {
                        Arc::from(icon)
                    }
                }),
                id: Arc::from(app.id.as_ref()),
                command: app
                    .exec
                    .as_ref()
                    .map(|exec| Arc::from(exec.as_ref()))
                    .unwrap_or(empty_string.clone()),
                pids: pids.drain(..).map(NonZeroU32::get).collect(),
            })
            .collect();
    }

    fn app_list(&self) -> &[Self::A] {
        &self.app_cache
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::platform::{AppsExt, Processes, ProcessesExt};

    #[test]
    fn test_refresh_cache() {
        let mut p = Processes::new();
        p.refresh_cache();

        let mut apps = LinuxApps::new();
        assert!(apps.app_cache.is_empty());

        apps.refresh_cache(p.process_list());
        assert!(!apps.app_cache.is_empty());

        let sample = apps.app_cache.iter().take(20);
        for app in sample {
            eprintln!("{:?}", app);
        }

        assert!(false)
    }
}
