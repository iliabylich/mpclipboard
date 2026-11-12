use core::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

pub(crate) const TASKS: usize = 1;
pub(crate) const TASK_SIZE: usize = 0;
pub(crate) const ROOT_TASK_SIZE: usize = 7368;
pub(crate) const TIMERS: usize = 1;
pub(crate) const FDS: usize = 1;

pub(crate) const FREEZE_TIME: Duration = Duration::from_secs(3);

pub(crate) const DNS_SERVER: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 53);

pub(crate) const TLS_BUFFER_SIZE: usize = 17 * 1024;

pub(crate) const UPGRADE_RESPONSE_BUFFER_SIZE: usize = 2048;
pub(crate) const UPGRADE_RESPONSE_MAX_HEADERS: usize = 32;
