/* sys_info_v2/gatherer/src/platform/linux/utilities.rs
 *
 * Copyright 2023 Romeo Calota
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

use dbus::arg::RefArg;

use crate::platform::utilities::*;

#[derive(Default)]
pub struct LinuxPlatformUtilities {}

impl PlatformUtilitiesExt for LinuxPlatformUtilities {
    fn on_main_app_exit(&self, mut callback: Box<dyn FnMut() + Send>) {
        use crate::critical;
        use dbus::{blocking::Connection, channel::MatchingReceiver, message::MatchRule};
        use std::sync::{atomic::*, Arc};

        std::thread::spawn(move || {
            let c = match Connection::new_session() {
                Ok(c) => c,
                Err(e) => {
                    critical!(
                        "Gatherer::PlatformUtilities",
                        "Failed to connect to the D-Bus session bus, and set up monitoring: {}",
                        e
                    );
                    return;
                }
            };

            let mut rule = MatchRule::new();
            rule.strict_sender = true;
            rule.sender = Some("org.freedesktop.DBus".into());
            rule.interface = Some("org.freedesktop.DBus".into());
            rule.path = Some("/org/freedesktop/DBus".into());
            rule.member = Some("NameLost".into());

            let proxy = c.with_proxy(
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                std::time::Duration::from_millis(5000),
            );
            let result: Result<(), dbus::Error> = proxy.method_call(
                "org.freedesktop.DBus.Monitoring",
                "BecomeMonitor",
                (vec![rule.match_str()], 0u32),
            );
            match result {
                Ok(_) => {
                    let done = Arc::new(AtomicBool::new(false));
                    let d = Arc::clone(&done);
                    c.start_receive(
                        rule,
                        Box::new(move |msg, _: &Connection| {
                            if let Some(name) = msg
                                .iter_init()
                                .get_refarg()
                                .and_then(|a| a.as_str().and_then(|s| Some(s.to_string())))
                            {
                                if name == "io.missioncenter.MissionCenter" {
                                    d.store(true, Ordering::Release);
                                    callback();
                                    false
                                } else {
                                    true
                                }
                            } else {
                                true
                            }
                        }),
                    );

                    while !done.load(Ordering::Acquire) {
                        c.process(std::time::Duration::from_millis(1000)).unwrap();
                    }
                }
                Err(e) => {
                    critical!(
                        "Gatherer::PlatformUtilities",
                        "Failed to connect to the D-Bus session bus, and set up monitoring: {}",
                        e
                    );
                    return;
                }
            }
        });
    }
}
