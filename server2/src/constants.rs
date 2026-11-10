use core::time::Duration;

pub(crate) const CLIENT_COUNT: usize = 10;

pub(crate) const TASK_COUNT: usize = CLIENT_COUNT;
pub(crate) const TASK_SIZE: usize = 5048;
pub(crate) const ROOT_TASK_SIZE: usize = 232;
pub(crate) const TIMER_COUNT: usize = CLIENT_COUNT;
pub(crate) const IO_COUNT: usize = CLIENT_COUNT + 1;

pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) const HANDSHAKE_REQUEST_SIZE: usize = 2048;
pub(crate) const HANDSHAKE_REQUEST_MAX_HEADERS: usize = 32;
pub(crate) const MAX_TOKEN_LENGTH: usize = 20;
pub(crate) const MAX_ID_LENGTH: usize = 20;
pub(crate) const PROTOCOL_VERSION: &str = "6";
pub(crate) const UPGRADE_PROTOCOL: &str = "mpclipboard-raw";
pub(crate) const MAX_TEXT_LENGTH: usize = 255;
