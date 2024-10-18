/* sys_info_v2/dbus_interface/service.rs
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

use std::collections::HashMap;
use std::{num::NonZeroU32, sync::Arc};

use dbus::{
    arg::{Arg, ArgType, Get, Iter, ReadAll, RefArg, TypeMismatchError},
    Signature,
};

#[derive(Debug, Clone)]
pub struct Service {
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub enabled: bool,
    pub running: bool,
    pub failed: bool,
    pub pid: Option<NonZeroU32>,
    pub user: Option<Arc<str>>,
    pub group: Option<Arc<str>>,
}

impl Default for Service {
    fn default() -> Self {
        let empty = Arc::<str>::from("");
        Self {
            name: empty.clone(),
            description: empty.clone(),
            enabled: false,
            running: false,
            failed: false,
            pid: None,
            user: None,
            group: None,
        }
    }
}

pub struct ServiceMap(HashMap<Arc<str>, Service>);

impl From<HashMap<Arc<str>, Service>> for ServiceMap {
    fn from(value: HashMap<Arc<str>, Service>) -> Self {
        Self(value)
    }
}

impl From<ServiceMap> for HashMap<Arc<str>, Service> {
    fn from(value: ServiceMap) -> Self {
        value.0
    }
}

impl Arg for ServiceMap {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("a(ssbbbuss)")
    }
}

impl ReadAll for ServiceMap {
    fn read(i: &mut Iter) -> Result<Self, TypeMismatchError> {
        i.get().ok_or(super::TypeMismatchError::new(
            ArgType::Invalid,
            ArgType::Invalid,
            0,
        ))
    }
}

impl<'a> Get<'a> for ServiceMap {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use gtk::glib::g_critical;

        let mut result = HashMap::new();

        match Iterator::next(i) {
            None => {
                g_critical!(
                    "MissionCenter::GathererDBusProxy",
                    "Failed to get Vec<Service>: Expected '0: ARRAY', got None",
                );
                return Some(ServiceMap(result));
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    g_critical!(
                        "MissionCenter::GathererDBusProxy",
                        "Failed to get Vec<Service>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for i in arr {
                        let mut this = Service::default();

                        let mut i = match i.as_iter() {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '0: STRUCT', got None",
                                );
                                continue;
                            }
                            Some(i) => i,
                        };
                        let service = i.as_mut();

                        this.name = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '0: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(n) => Arc::<str>::from(n),
                            },
                        };

                        this.description = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '1: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '1: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(d) => Arc::<str>::from(d),
                            },
                        };

                        this.enabled = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '2: b', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_i64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '2: b', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(e) => e != 0,
                            },
                        };

                        this.running = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '3: b', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_i64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '3: b', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(r) => r != 0,
                            },
                        };

                        this.failed = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '4: b', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_i64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '4: b', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(r) => r != 0,
                            },
                        };

                        this.pid = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '5: u', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '5: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(p) => NonZeroU32::new(p as u32),
                            },
                        };

                        this.user = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '6: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '6: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(u) => {
                                    if u.is_empty() {
                                        None
                                    } else {
                                        Some(Arc::<str>::from(u))
                                    }
                                }
                            },
                        };

                        this.group = match Iterator::next(service) {
                            None => {
                                g_critical!(
                                    "MissionCenter::GathererDBusProxy",
                                    "Failed to get Service: Expected '7: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    g_critical!(
                                        "MissionCenter::GathererDBusProxy",
                                        "Failed to get Service: Expected '7: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(g) => {
                                    if g.is_empty() {
                                        None
                                    } else {
                                        Some(Arc::<str>::from(g))
                                    }
                                }
                            },
                        };

                        result.insert(this.name.clone(), this);
                    }
                }
            },
        }

        Some(ServiceMap(result))
    }
}
