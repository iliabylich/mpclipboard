use core::time::Duration;

pub(crate) const MAX_CLIENTS: usize = 10;

pub(crate) const MAX_TASKS_COUNT: usize = MAX_CLIENTS;
pub(crate) const MAX_TASK_SIZE: usize = 4896;
pub(crate) const MAX_ROOT_TASK_SIZE: usize = 232;
pub(crate) const MAX_TIMERS_COUNT: usize = MAX_CLIENTS;
pub(crate) const MAX_IO_COUNT: usize = MAX_CLIENTS + 1;

pub(crate) const REAP_STRANGER_AFTER: Duration = Duration::from_secs(3);

pub(crate) const MAX_REQUEST_SIZE: usize = 2048;
pub(crate) const MAX_HEADERS: usize = 32;
pub(crate) const PROTOCOL_VERSION: &str = "6";
pub(crate) const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";

pub(crate) const SWITCHING_PROTOCOLS: &[u8] = concat!(
    "HTTP/1.1 101 Switching Protocols\r\n",
    "Connection: Upgrade\r\n",
    "Upgrade: mpclipboard-raw\r\n",
    "\r\n",
)
.as_bytes();
pub(crate) const UNAUTHORIZED: &[u8] = concat!(
    "HTTP/1.1 401 Unauthorized\r\n",
    "Content-Length: 0\r\n",
    "\r\n",
)
.as_bytes();

pub(crate) const MAX_TEXT_LEN: usize = 255;
