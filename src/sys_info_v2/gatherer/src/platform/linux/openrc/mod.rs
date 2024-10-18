use std::collections::HashMap;
use std::ffi::CStr;
use std::fs::OpenOptions;
use std::sync::Arc;

use thiserror::Error;

pub use controller::Controller;
pub use service::*;
use string_list::RC_STRINGLIST;

use crate::warning;

mod controller;
mod service;
mod string_list;

type FnRcRunLevelList = unsafe extern "C" fn() -> *mut RC_STRINGLIST;
type FnRcServicesInRunlevel = unsafe extern "C" fn(*const libc::c_char) -> *mut RC_STRINGLIST;
type FnRcServiceState = unsafe extern "C" fn(*const libc::c_char) -> u32;
type FnRcServiceDescription =
    unsafe extern "C" fn(*const libc::c_char, *const libc::c_char) -> *mut libc::c_char;
type FnRcServiceValueGet =
    unsafe extern "C" fn(*const libc::c_char, *const libc::c_char) -> *mut libc::c_char;
type FnRCStringListFree = unsafe extern "C" fn(*mut RC_STRINGLIST);

#[derive(Debug, Error)]
pub enum OpenRCError {
    #[error("LibLoading error: {0}")]
    LibLoadingError(#[from] libloading::Error),
    #[error("Missing runlevels")]
    MissingRunLevels,
    #[error("Missing services")]
    MissingServices,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Command execution error: {0}. Exited with status code {1}")]
    CommandExecutionError(Arc<str>, i32),
}

pub struct ServiceManager {
    fn_rc_runlevel_list: libloading::Symbol<'static, FnRcRunLevelList>,
    fn_rc_services_in_runlevel: libloading::Symbol<'static, FnRcServicesInRunlevel>,
    fn_rc_service_state: libloading::Symbol<'static, FnRcServiceState>,
    fn_rc_service_description: libloading::Symbol<'static, FnRcServiceDescription>,
    fn_rc_service_value_get: libloading::Symbol<'static, FnRcServiceValueGet>,

    fn_rc_string_list_free: libloading::Symbol<'static, FnRCStringListFree>,
}

impl ServiceManager {
    pub fn new() -> Result<Self, OpenRCError> {
        let handle = Box::leak(Box::new(unsafe { libloading::Library::new("librc.so.1")? }));

        let fn_rc_runlevel_list = unsafe { handle.get::<FnRcRunLevelList>(b"rc_runlevel_list\0")? };

        let fn_rc_services_in_runlevel =
            unsafe { handle.get::<FnRcServicesInRunlevel>(b"rc_services_in_runlevel\0")? };

        let fn_rc_service_state = unsafe { handle.get::<FnRcServiceState>(b"rc_service_state\0")? };

        let fn_rc_service_description =
            unsafe { handle.get::<FnRcServiceDescription>(b"rc_service_description\0")? };

        let fn_rc_service_value_get =
            unsafe { handle.get::<FnRcServiceValueGet>(b"rc_service_value_get\0")? };

        let fn_rc_string_list_free =
            unsafe { handle.get::<FnRCStringListFree>(b"rc_stringlist_free\0")? };

        Ok(Self {
            fn_rc_runlevel_list,
            fn_rc_services_in_runlevel,
            fn_rc_service_state,
            fn_rc_service_description,
            fn_rc_service_value_get,

            fn_rc_string_list_free,
        })
    }

    pub fn controller(&self) -> Controller {
        Controller
    }

    pub fn list_services(&self) -> Result<Vec<Service>, OpenRCError> {
        let runlevels = unsafe { (self.fn_rc_runlevel_list)() };
        if runlevels.is_null() {
            warning!(
                "Gatherer::OpenRC",
                "Empty runlevel list returned from OpenRC."
            );
            return Err(OpenRCError::MissingRunLevels);
        }

        let mut result = HashMap::new();

        let empty_string = Arc::<str>::from("");
        let mut buffer = String::new();

        let runlevel_list = unsafe { &*runlevels };
        let mut current_rl = runlevel_list.tqh_first;
        while !current_rl.is_null() {
            let rl_rc_string = unsafe { &*current_rl };
            let rl_name = unsafe { CStr::from_ptr(rl_rc_string.value) }.to_string_lossy();
            let rl_name = Arc::<str>::from(rl_name.as_ref());

            let services_names = unsafe { (self.fn_rc_services_in_runlevel)(rl_rc_string.value) };
            if services_names.is_null() {
                warning!(
                    "Gatherer::OpenRC",
                    "Empty service list returned for runlevel '{}'.",
                    rl_name.as_ref()
                );
                continue;
            }

            let rc_string_list = unsafe { &*services_names };
            let mut current = rc_string_list.tqh_first;
            while !current.is_null() {
                let rc_string = unsafe { &*current };
                let service_name = unsafe { CStr::from_ptr(rc_string.value) };

                let description_cstr =
                    unsafe { (self.fn_rc_service_description)(rc_string.value, std::ptr::null()) };
                let description = if !description_cstr.is_null() {
                    let description =
                        Arc::from(unsafe { CStr::from_ptr(description_cstr) }.to_string_lossy());
                    unsafe {
                        libc::free(description_cstr as *mut libc::c_void);
                    }
                    description
                } else {
                    empty_string.clone()
                };

                let state = unsafe { (self.fn_rc_service_state)(rc_string.value) };

                let pidfile = unsafe {
                    (self.fn_rc_service_value_get)(
                        rc_string.value,
                        b"pidfile\0".as_ptr() as *const libc::c_char,
                    )
                };
                let pid = if !pidfile.is_null() {
                    let pidfile_str = unsafe { CStr::from_ptr(pidfile) }.to_string_lossy();
                    let pid = if let Some(mut pf) = OpenOptions::new()
                        .read(true)
                        .open(pidfile_str.as_ref())
                        .ok()
                    {
                        buffer.clear();
                        if let Ok(_) = std::io::Read::read_to_string(&mut pf, &mut buffer) {
                            if let Ok(pid) = buffer.trim().parse::<u32>() {
                                pid
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    unsafe {
                        libc::free(pidfile as *mut libc::c_void);
                    }

                    pid
                } else {
                    0
                };

                let service_name = Arc::<str>::from(service_name.to_string_lossy().as_ref());
                result.insert(
                    service_name.clone(),
                    Service {
                        name: service_name,
                        description,
                        runlevel: rl_name.clone(),
                        state: unsafe { std::mem::transmute(state & 0xFF) },
                        pid,
                    },
                );

                current = rc_string.entries.tqe_next;
            }

            unsafe { (self.fn_rc_string_list_free)(services_names) };

            current_rl = rl_rc_string.entries.tqe_next;
        }

        unsafe { (self.fn_rc_string_list_free)(runlevels) };

        let services_names = unsafe { (self.fn_rc_services_in_runlevel)(std::ptr::null()) };
        if !services_names.is_null() {
            let rc_string_list = unsafe { &*services_names };
            let mut current = rc_string_list.tqh_first;
            while !current.is_null() {
                let rc_string = unsafe { &*current };
                let service_name = unsafe { CStr::from_ptr(rc_string.value) };

                let description_cstr =
                    unsafe { (self.fn_rc_service_description)(rc_string.value, std::ptr::null()) };
                let description = if !description_cstr.is_null() {
                    let description =
                        Arc::from(unsafe { CStr::from_ptr(description_cstr) }.to_string_lossy());
                    unsafe {
                        libc::free(description_cstr as *mut libc::c_void);
                    }
                    description
                } else {
                    empty_string.clone()
                };

                let state = unsafe { (self.fn_rc_service_state)(rc_string.value) };

                let pidfile = unsafe {
                    (self.fn_rc_service_value_get)(
                        rc_string.value,
                        b"pidfile\0".as_ptr() as *const libc::c_char,
                    )
                };
                let pid = if !pidfile.is_null() {
                    let pidfile_str = unsafe { CStr::from_ptr(pidfile) }.to_string_lossy();
                    let pid = if let Some(mut pf) = OpenOptions::new()
                        .read(true)
                        .open(pidfile_str.as_ref())
                        .ok()
                    {
                        buffer.clear();
                        if let Ok(_) = std::io::Read::read_to_string(&mut pf, &mut buffer) {
                            if let Ok(pid) = buffer.trim().parse::<u32>() {
                                pid
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    unsafe {
                        libc::free(pidfile as *mut libc::c_void);
                    }

                    pid
                } else {
                    0
                };

                let service_name = Arc::<str>::from(service_name.to_string_lossy().as_ref());
                if !result.contains_key(&service_name) {
                    result.insert(
                        service_name.clone(),
                        Service {
                            name: service_name,
                            description,
                            runlevel: empty_string.clone(),
                            state: unsafe { std::mem::transmute(state & 0xFF) },
                            pid,
                        },
                    );
                }

                current = rc_string.entries.tqe_next;
            }

            unsafe { (self.fn_rc_string_list_free)(services_names) };
        }

        if result.is_empty() {
            return Err(OpenRCError::MissingServices);
        }

        Ok(result.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc_services_in_runlevel() {
        let openrc = ServiceManager::new().unwrap();
        let services = openrc.list_services().unwrap();
        assert!(!services.is_empty());
        dbg!(services);
    }
}
