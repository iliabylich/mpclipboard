use core::time::Duration;

pub(crate) const MAX_CLIENTS: usize = 10;

pub(crate) const TASKS: usize = MAX_CLIENTS + 1;
pub(crate) const TASK_SIZE: usize = 5352;
pub(crate) const ROOT_TASK_SIZE: usize = 232;
pub(crate) const TIMERS: usize = MAX_CLIENTS;
pub(crate) const FDS: usize = MAX_CLIENTS + 1;

pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) const HANDSHAKE_REQUEST_BUFFER_SIZE: usize = 2048;
pub(crate) const HANDSHAKE_REQUEST_MAX_HEADERS: usize = 32;
pub(crate) const MAX_TOKEN_LENGTH: usize = 20;
pub(crate) const MAX_ID_LENGTH: usize = 20;
pub(crate) const PROTOCOL_VERSION: &str = "6";
pub(crate) const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";
pub(crate) const MAX_TEXT_LENGTH: usize = 255;
