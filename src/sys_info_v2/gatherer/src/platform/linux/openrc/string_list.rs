#[repr(C)]
pub struct RC_STRING_QUEUE {
    pub tqe_next: *mut RC_STRING,
    pub tqe_prev: *mut *mut RC_STRING,
}

#[repr(C)]
pub struct RC_STRING {
    pub value: *mut libc::c_char,
    pub entries: RC_STRING_QUEUE,
}

#[repr(C)]
pub struct RC_STRINGLIST {
    pub tqh_first: *mut RC_STRING,
    pub tqh_last: *mut *mut RC_STRING,
}
