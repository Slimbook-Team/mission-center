use std::{num::NonZeroU32, sync::Arc};

use dbus::{
    arg::{Arg, ArgType, Get, Iter, RefArg},
    Signature,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LoadState {
    Loaded,
    Error,
    Masked,
    NotFound,
    Unknown,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Properties {
    pub enabled: bool,
    pub pid: u32,
    pub user: Arc<str>,
    pub uid: u32,
    pub group: Arc<str>,
    pub gid: u32,
}

impl Default for Properties {
    fn default() -> Self {
        let empty_str: Arc<str> = Arc::from("");
        Self {
            enabled: false,
            pid: 0,
            user: empty_str.clone(),
            uid: 0,
            group: empty_str.clone(),
            gid: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub load_state: LoadState,
    pub active_state: ActiveState,
    pub sub_state: Arc<str>,
    pub following: Arc<str>,
    pub unit_path: Arc<str>,
    pub job_id: Option<u32>,
    pub job_type: Arc<str>,
    pub job_path: Arc<str>,

    pub properties: Properties,
}

impl Default for Service {
    fn default() -> Self {
        let empty_str: Arc<str> = Arc::from("");
        Self {
            name: empty_str.clone(),
            description: empty_str.clone(),
            load_state: LoadState::Unknown,
            active_state: ActiveState::Unknown,
            sub_state: empty_str.clone(),
            following: empty_str.clone(),
            unit_path: empty_str.clone(),
            job_id: None,
            job_type: empty_str.clone(),
            job_path: empty_str.clone(),

            properties: Properties::default(),
        }
    }
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
        self.properties.enabled
    }

    #[inline]
    pub fn running(&self) -> bool {
        self.active_state == ActiveState::Active
    }

    #[inline]
    pub fn failed(&self) -> bool {
        self.active_state == ActiveState::Failed
    }

    #[inline]
    pub fn pid(&self) -> Option<NonZeroU32> {
        match self.properties.pid {
            0 => None,
            _ => NonZeroU32::new(self.properties.pid),
        }
    }

    #[inline]
    pub fn user(&self) -> Option<&str> {
        match self.properties.user.as_ref() {
            "" => None,
            _ => Some(self.properties.user.as_ref()),
        }
    }

    #[inline]
    pub fn group(&self) -> Option<&str> {
        match self.properties.group.as_ref() {
            "" => None,
            _ => Some(self.properties.group.as_ref()),
        }
    }
}

pub struct ServiceVec(pub Vec<Service>);

impl From<ServiceVec> for Vec<Service> {
    fn from(v: ServiceVec) -> Self {
        v.0
    }
}

impl Arg for ServiceVec {
    const ARG_TYPE: ArgType = ArgType::Struct;

    fn signature() -> Signature<'static> {
        Signature::from("a(ssssssouso)")
    }
}

impl<'a> Get<'a> for ServiceVec {
    fn get(i: &mut Iter<'a>) -> Option<Self> {
        use crate::critical;

        let mut result = vec![];

        match Iterator::next(i) {
            None => {
                critical!(
                    "Gatherer::SystemDServices",
                    "Failed to get Vec<SystemD Unit>: Expected '0: ARRAY', got None",
                );
                return None;
            }
            Some(arg) => match arg.as_iter() {
                None => {
                    critical!(
                        "Gatherer::SystemDServices",
                        "Failed to get Vec<SystemD Unit>: Expected '0: ARRAY', got {:?}",
                        arg.arg_type(),
                    );
                    return None;
                }
                Some(arr) => {
                    for i in arr {
                        let mut this = Service::default();

                        let mut i = match i.as_iter() {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '0: STRUCT', got None",
                                );
                                continue;
                            }
                            Some(i) => i,
                        };
                        let unit = i.as_mut();

                        this.name = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '0: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '0: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(n) => {
                                    if !n.ends_with(".service") {
                                        continue;
                                    }

                                    Arc::from(n)
                                }
                            },
                        };

                        this.description = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '1: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '1: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(d) => Arc::<str>::from(d),
                            },
                        };

                        this.load_state = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '2: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '2: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(ls) => match ls {
                                    "loaded" => LoadState::Loaded,
                                    "error" => LoadState::Error,
                                    "masked" => LoadState::Masked,
                                    "not-found" => LoadState::NotFound,
                                    _ => LoadState::Unknown,
                                },
                            },
                        };

                        this.active_state = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '3: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '3: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(a) => match a {
                                    "active" => ActiveState::Active,
                                    "reloading" => ActiveState::Reloading,
                                    "inactive" => ActiveState::Inactive,
                                    "failed" => ActiveState::Failed,
                                    "activating" => ActiveState::Activating,
                                    "deactivating" => ActiveState::Deactivating,
                                    _ => ActiveState::Unknown,
                                },
                            },
                        };

                        this.sub_state = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '4: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '4: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(ss) => Arc::from(ss),
                            },
                        };

                        this.following = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '5: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '5: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(f) => Arc::from(f),
                            },
                        };

                        this.unit_path = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '6: o', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '6: o', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(up) => Arc::from(up),
                            },
                        };

                        this.job_id = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '7: u', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_u64() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '7: u', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(ji) => match ji {
                                    0 => None,
                                    ji => Some(ji as u32),
                                },
                            },
                        };

                        this.job_type = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '8: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '8: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(jt) => Arc::from(jt),
                            },
                        };

                        this.job_path = match Iterator::next(unit) {
                            None => {
                                critical!(
                                    "Gatherer::SystemDServices",
                                    "Failed to get SystemD Unit: Expected '9: s', got None",
                                );
                                continue;
                            }
                            Some(arg) => match arg.as_str() {
                                None => {
                                    critical!(
                                        "Gatherer::SystemDServices",
                                        "Failed to get SystemD Unit: Expected '9: s', got {:?}",
                                        arg.arg_type(),
                                    );
                                    continue;
                                }
                                Some(jp) => Arc::from(jp),
                            },
                        };

                        result.push(this);
                    }
                }
            },
        }

        Some(ServiceVec(result))
    }
}
