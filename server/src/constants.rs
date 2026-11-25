pub(crate) const MAX_CLIENTS: usize = 10;

pub(crate) const EVENTS: usize = 1;
pub(crate) const TASKS: usize = MAX_CLIENTS;
pub(crate) const TASK_SIZE: usize = 7896;
pub(crate) const ROOT_TASK_SIZE: usize = 480;
pub(crate) const TIMERS: usize = MAX_CLIENTS;
pub(crate) const FDS: usize = MAX_CLIENTS + 1;

pub(crate) const UPGRADE_TIMEOUT_IN_SECS: u64 = 3;

pub(crate) const UPGRADE_REQUEST_BUFFER_SIZE: usize = 2048;
pub(crate) const UPGRADE_REQUEST_MAX_HEADERS: usize = 32;

pub(crate) mod vars {
    use core::ffi::CStr;

    pub(crate) const BIND_ADDRESS: &CStr = c"MPCLIPBOARD_BIND_ADDRESS";
    pub(crate) const TOKEN: &CStr = c"MPCLIPBOARD_TOKEN";
    pub(crate) const RUST_LOG: &CStr = c"RUST_LOG";
}
