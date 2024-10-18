use std::num::NonZeroU32;
use std::{
    mem::{align_of, size_of},
    ops::Deref,
    slice::from_raw_parts,
    str::from_utf8_unchecked,
    sync::Arc,
    time::Duration,
};

use dbus::{
    arg::{AppendAll, IterAppend, ReadAll, RefArg},
    blocking::{stdintf::org_freedesktop_dbus::Properties, BlockingSender, Proxy, SyncConnection},
    channel::Sender,
    strings::{Interface, Member},
    Error, Message,
};
use libloading::Symbol;
use static_assertions::const_assert;
use thiserror::Error;

pub use controller::Controller;
use service::ServiceVec;
pub use service::*;

use crate::error;

mod controller;
mod service;

const_assert!(size_of::<Message>() == size_of::<*mut ()>());
const_assert!(align_of::<Message>() == align_of::<*mut ()>());

#[allow(dead_code)]
mod ffi {
    #[repr(C)]
    pub struct DBusMessage {
        _private: [u8; 0],
    }

    #[repr(C)]
    #[allow(non_camel_case_types)]
    pub struct sd_journal {
        _unused: [u8; 0],
    }

    pub type FnSdJournalOpen =
        unsafe extern "C" fn(ret: *mut *mut sd_journal, flags: i32) -> libc::c_int;
    pub type FnSdJournalClose = unsafe extern "C" fn(j: *mut sd_journal);

    pub type FnSdJournalAddMatch = unsafe extern "C" fn(
        j: *mut sd_journal,
        match_: *const libc::c_void,
        size: libc::size_t,
    ) -> libc::c_int;
    pub type FnSdJournalAddDisjunction = unsafe extern "C" fn(j: *mut sd_journal) -> libc::c_int;
    pub type FnSdJournalAddConjunction = unsafe extern "C" fn(j: *mut sd_journal) -> libc::c_int;

    pub type FnSdJournalSeekTail = unsafe extern "C" fn(j: *mut sd_journal) -> libc::c_int;
    pub type FnSdJournalPrevious = unsafe extern "C" fn(j: *mut sd_journal) -> libc::c_int;

    pub type FnSdJournalGetData = unsafe extern "C" fn(
        j: *mut sd_journal,
        field: *const libc::c_char,
        data: *mut *const libc::c_void,
        length: *mut libc::size_t,
    ) -> libc::c_int;

    pub const SD_JOURNAL_LOCAL_ONLY: i32 = 1 << 0;
    pub const SD_JOURNAL_RUNTIME_ONLY: i32 = 1 << 1;
    pub const SD_JOURNAL_SYSTEM: i32 = 1 << 2;
    pub const SD_JOURNAL_CURRENT_USER: i32 = 1 << 3;
    pub const SD_JOURNAL_OS_ROOT: i32 = 1 << 4;
    pub const SD_JOURNAL_ALL_NAMESPACES: i32 = 1 << 5;
    pub const SD_JOURNAL_INCLUDE_DEFAULT_NAMESPACE: i32 = 1 << 6;
    pub const SD_JOURNAL_TAKE_DIRECTORY_FD: i32 = 1 << 7;
    pub const SD_JOURNAL_ASSUME_IMMUTABLE: i32 = 1 << 8;

    pub const SD_JOURNAL_NOP: i32 = 0;
    pub const SD_JOURNAL_APPEND: i32 = 1;
    pub const SD_JOURNAL_INVALIDATE: i32 = 2;

    extern "C" {
        pub fn dbus_message_set_allow_interactive_authorization(msg: *mut DBusMessage, allow: u32);
    }
}

const MAX_LOG_MESSAGE_COUNT: usize = 5;

pub fn dbus_call_wait_reply<
    'a,
    'i,
    'm,
    R: ReadAll,
    A: AppendAll,
    I: Into<Interface<'i>>,
    M: Into<Member<'m>>,
>(
    proxy: &Proxy<'a, &'a SyncConnection>,
    i: I,
    m: M,
    args: A,
) -> Result<R, Error> {
    let mut msg = Message::method_call(&proxy.destination, &proxy.path, &i.into(), &m.into());
    unsafe {
        ffi::dbus_message_set_allow_interactive_authorization(std::mem::transmute_copy(&msg), 1);
    }
    args.append(&mut IterAppend::new(&mut msg));
    let r = proxy
        .connection
        .send_with_reply_and_block(msg, proxy.timeout)?;
    Ok(R::read(&mut r.iter_init())?)
}

fn dbus_call_forget<A: AppendAll>(
    proxy: &Proxy<&SyncConnection>,
    i: &str,
    m: &str,
    args: A,
) -> Result<(), Error> {
    let mut msg = Message::new_method_call(&proxy.destination, &proxy.path, i, m)
        .map_err(|e| Error::new_failed(e.as_str()))?;
    args.append(&mut IterAppend::new(&mut msg));
    unsafe {
        ffi::dbus_message_set_allow_interactive_authorization(std::mem::transmute_copy(&msg), 1);
    }

    proxy
        .connection
        .send(msg)
        .map_err(|_| Error::new_failed(&format!("Failed to send message `{m}`")))?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum SystemDError {
    #[error("DBus error: {0}")]
    DBusError(#[from] dbus::Error),
    #[error("DBus error: {0}")]
    TypeMismatchError(#[from] dbus::arg::TypeMismatchError),
    #[error("Library loading error: {0}")]
    LibLoadingError(#[from] libloading::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to open journal: {0}")]
    JournalOpenError(Arc<str>),
    #[error("Seek failed: {0}")]
    JournalSeekError(Arc<str>),
    #[error("Failed to add match: {0}")]
    JournalAddMatchError(Arc<str>),
    #[error("Failed to add disjunction: {0}")]
    JournalAddDisjunctionError(Arc<str>),
    #[error("Failed to add conjunction: {0}")]
    JournalAddConjunctionError(Arc<str>),
    #[error("Failed to iterate journal entries: {0}")]
    JournalIterateError(Arc<str>),
}

pub struct ServiceManager<'a> {
    connection: Arc<SyncConnection>,
    systemd1: Proxy<'a, &'a SyncConnection>,

    fn_sd_journal_open: Symbol<'static, ffi::FnSdJournalOpen>,
    fn_sd_journal_close: Symbol<'static, ffi::FnSdJournalClose>,
    fn_sd_journal_seek_tail: Symbol<'static, ffi::FnSdJournalSeekTail>,
    fn_sd_journal_add_match: Symbol<'static, ffi::FnSdJournalAddMatch>,
    fn_sd_journal_add_disjunction: Symbol<'static, ffi::FnSdJournalAddDisjunction>,
    fn_sd_journal_add_conjunction: Symbol<'static, ffi::FnSdJournalAddConjunction>,
    fn_sd_journal_previous: Symbol<'static, ffi::FnSdJournalPrevious>,
    fn_sd_journal_get_data: Symbol<'static, ffi::FnSdJournalGetData>,

    boot_id: Arc<str>,
}

impl<'a> ServiceManager<'a> {
    pub fn new() -> Result<Self, SystemDError> {
        let connection = Arc::new(SyncConnection::new_system()?);
        let connection_ptr = connection.deref() as *const SyncConnection;

        let systemd1 = unsafe {
            (&*connection_ptr).with_proxy(
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                Duration::from_millis(30_000),
            )
        };

        let handle = Box::leak(Box::new(unsafe {
            libloading::Library::new("libsystemd.so.0")?
        }));

        let fn_sd_journal_open =
            unsafe { handle.get::<ffi::FnSdJournalOpen>(b"sd_journal_open\0")? };
        let fn_sd_journal_close =
            unsafe { handle.get::<ffi::FnSdJournalClose>(b"sd_journal_close\0")? };
        let fn_sd_journal_seek_tail =
            unsafe { handle.get::<ffi::FnSdJournalSeekTail>(b"sd_journal_seek_tail\0")? };
        let fn_sd_journal_add_match =
            unsafe { handle.get::<ffi::FnSdJournalAddMatch>(b"sd_journal_add_match\0")? };
        let fn_sd_journal_add_disjunction = unsafe {
            handle.get::<ffi::FnSdJournalAddDisjunction>(b"sd_journal_add_disjunction\0")?
        };
        let fn_sd_journal_add_conjunction = unsafe {
            handle.get::<ffi::FnSdJournalAddConjunction>(b"sd_journal_add_conjunction\0")?
        };
        let fn_sd_journal_previous =
            unsafe { handle.get::<ffi::FnSdJournalPrevious>(b"sd_journal_previous\0")? };
        let fn_sd_journal_get_data =
            unsafe { handle.get::<ffi::FnSdJournalGetData>(b"sd_journal_get_data\0")? };

        let boot_id = Arc::<str>::from(
            std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?
                .trim()
                .replace("-", ""),
        );

        Ok(Self {
            connection,
            systemd1,

            fn_sd_journal_open,
            fn_sd_journal_close,
            fn_sd_journal_seek_tail,
            fn_sd_journal_add_match,
            fn_sd_journal_add_conjunction,
            fn_sd_journal_add_disjunction,
            fn_sd_journal_previous,
            fn_sd_journal_get_data,

            boot_id,
        })
    }

    pub fn controller(&self) -> Result<Controller<'a>, SystemDError> {
        Ok(Controller::new(
            self.connection.clone(),
            self.systemd1.clone(),
        ))
    }

    pub fn list_services(&self) -> Result<Vec<Service>, SystemDError> {
        let (mut services,): (ServiceVec,) = dbus_call_wait_reply(
            &self.systemd1,
            "org.freedesktop.systemd1.Manager",
            "ListUnits",
            (),
        )?;

        let mut services = services
            .0
            .drain(..)
            .filter(|s| s.load_state != LoadState::NotFound)
            .collect::<Vec<_>>();

        for service in &mut services {
            service.properties = self.service_properties(&service.unit_path)?;
        }

        Ok(services)
    }

    pub fn service_logs(
        &self,
        name: &str,
        pid: Option<NonZeroU32>,
    ) -> Result<Arc<str>, SystemDError> {
        fn error_string(mut errno: i32) -> Arc<str> {
            if errno < 0 {
                errno = -errno;
            }

            unsafe {
                let mut buf = [0; 1024];
                let _ = libc::strerror_r(errno, buf.as_mut_ptr(), buf.len());
                let c_str = std::ffi::CStr::from_ptr(buf.as_ptr());

                c_str.to_string_lossy().into()
            }
        }

        struct JournalHandle {
            j: *mut ffi::sd_journal,
            close: Symbol<'static, ffi::FnSdJournalClose>,
        }

        impl Drop for JournalHandle {
            fn drop(&mut self) {
                unsafe { (self.close)(self.j) };
            }
        }

        let mut j: *mut ffi::sd_journal = std::ptr::null_mut();
        let ret = unsafe { (self.fn_sd_journal_open)(&mut j, ffi::SD_JOURNAL_SYSTEM) };
        if ret < 0 {
            let err_string = error_string(ret);
            error!(
                "Gatherer::SystemD",
                "Failed to open journal: {}",
                err_string.as_ref()
            );

            return Err(SystemDError::JournalOpenError(err_string));
        }

        let raii_handle = JournalHandle {
            j,
            close: self.fn_sd_journal_close.clone(),
        };

        let ret = unsafe {
            (self.fn_sd_journal_add_match)(j, format!("UNIT={}\0", name).as_ptr() as _, 0)
        };
        if ret < 0 {
            let err_string = error_string(ret);
            error!(
                "Gatherer::SystemD",
                "Failed to add match: {}",
                err_string.as_ref()
            );

            return Err(SystemDError::JournalAddMatchError(err_string));
        }

        let ret = unsafe { (self.fn_sd_journal_add_disjunction)(j) };
        if ret < 0 {
            let err_string = error_string(ret);
            error!(
                "Gatherer::SystemD",
                "Failed to add disjunction: {}",
                err_string.as_ref()
            );

            return Err(SystemDError::JournalAddDisjunctionError(err_string));
        }

        if let Some(pid) = pid {
            let ret = unsafe {
                (self.fn_sd_journal_add_match)(j, format!("_PID={}\0", pid).as_ptr() as _, 0)
            };
            if ret < 0 {
                let err_string = error_string(ret);
                error!(
                    "Gatherer::SystemD",
                    "Failed to add match: {}",
                    err_string.as_ref()
                );

                return Err(SystemDError::JournalAddMatchError(err_string));
            }
        }

        let ret = unsafe { (self.fn_sd_journal_add_conjunction)(j) };
        if ret < 0 {
            let err_string = error_string(ret);
            error!(
                "Gatherer::SystemD",
                "Failed to add disjunction: {}",
                err_string.as_ref()
            );

            return Err(SystemDError::JournalAddConjunctionError(err_string));
        }

        let ret = unsafe {
            (self.fn_sd_journal_add_match)(
                j,
                format!("_BOOT_ID={}\0", self.boot_id).as_ptr() as _,
                0,
            )
        };
        if ret < 0 {
            let err_string = error_string(ret);
            error!(
                "Gatherer::SystemD",
                "Failed to add match: {}",
                err_string.as_ref()
            );

            return Err(SystemDError::JournalAddMatchError(err_string));
        }

        let ret = unsafe { (self.fn_sd_journal_seek_tail)(j) };
        if ret < 0 {
            let err_string = error_string(ret);
            error!(
                "Gatherer::SystemD",
                "Failed to seek to tail: {}",
                err_string.as_ref()
            );

            return Err(SystemDError::JournalSeekError(err_string));
        }

        let mut messages = Vec::with_capacity(MAX_LOG_MESSAGE_COUNT);
        loop {
            let ret = unsafe { (self.fn_sd_journal_previous)(j) };
            if ret == 0 {
                break;
            }

            if ret < 0 {
                let err_string = error_string(ret);
                error!(
                    "Gatherer::SystemD",
                    "Failed to iterate journal entries: {}",
                    err_string.as_ref()
                );

                return Err(SystemDError::JournalIterateError(err_string));
            }

            let mut data: *const libc::c_void = std::ptr::null_mut();
            let mut length: libc::size_t = 0;

            let ret = unsafe {
                (self.fn_sd_journal_get_data)(j, "MESSAGE\0".as_ptr() as _, &mut data, &mut length)
            };
            if ret == 0 {
                if messages.len() >= MAX_LOG_MESSAGE_COUNT {
                    break;
                }

                let message = Arc::<str>::from(
                    &unsafe { from_utf8_unchecked(from_raw_parts(data as *const u8, length)) }[8..],
                );
                messages.push(message);
            }
        }

        drop(raii_handle);

        messages.reverse();
        Ok(Arc::from(messages.join("\n")))
    }

    #[inline]
    fn service_properties(&self, unit_path: &str) -> Result<service::Properties, SystemDError> {
        let unit_proxy = self.connection.with_proxy(
            "org.freedesktop.systemd1",
            unit_path,
            Duration::from_millis(5000),
        );

        let enabled: Box<dyn RefArg> =
            unit_proxy.get("org.freedesktop.systemd1.Unit", "UnitFileState")?;
        let enabled = enabled.as_str().unwrap_or_default();

        let pid: Box<dyn RefArg> = unit_proxy.get("org.freedesktop.systemd1.Service", "MainPID")?;
        let pid = pid.as_u64().unwrap() as u32;

        let user: Box<dyn RefArg> = unit_proxy.get("org.freedesktop.systemd1.Service", "User")?;
        let user = user.as_str().unwrap();

        let uid: Box<dyn RefArg> = unit_proxy.get("org.freedesktop.systemd1.Service", "UID")?;
        let uid = uid.as_u64().unwrap() as u32;

        let group: Box<dyn RefArg> = unit_proxy.get("org.freedesktop.systemd1.Service", "Group")?;
        let group = group.as_str().unwrap();

        let gid: Box<dyn RefArg> = unit_proxy.get("org.freedesktop.systemd1.Service", "GID")?;
        let gid = gid.as_u64().unwrap() as u32;

        Ok(service::Properties {
            enabled: enabled.to_ascii_lowercase() == "enabled",
            pid,
            user: Arc::from(user),
            uid,
            group: Arc::from(group),
            gid,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_services() -> Result<(), anyhow::Error> {
        let systemd = ServiceManager::new()?;
        let services = systemd.list_services()?;
        assert!(!services.is_empty());
        dbg!(services);

        Ok(())
    }

    #[test]
    fn test_service_logs() -> Result<(), anyhow::Error> {
        let systemd = ServiceManager::new()?;
        let logs = systemd.service_logs("NetworkManager.service", NonZeroU32::new(883))?;
        dbg!(logs);

        Ok(())
    }
}
