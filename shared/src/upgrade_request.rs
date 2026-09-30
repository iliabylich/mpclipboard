use crate::{
    CONNECTION_UPGRADE_HEADER, HOST_PREFIX, HostPort, ID, ID_PREFIX, MAX_HOST_PORT_LENGTH,
    MAX_ID_LENGTH, MAX_TOKEN_LENGTH, MAX_VERSION_LENGTH, START_LINE, TOKEN_PREFIX, Token,
    UPGRADE_MPCLIPBOARD_RAW_HEADER, VERSION_PREFIX, Version,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpgradeRequest {
    pub host: HostPort,
    pub token: Token,
    pub id: ID,
    pub version: Version,
}

impl UpgradeRequest {
    pub(crate) const MAX_LENGTH: usize = START_LINE.len()
        + 2
        + HOST_PREFIX.len()
        + MAX_HOST_PORT_LENGTH
        + 2
        + TOKEN_PREFIX.len()
        + MAX_TOKEN_LENGTH
        + 2
        + ID_PREFIX.len()
        + MAX_ID_LENGTH
        + 2
        + VERSION_PREFIX.len()
        + MAX_VERSION_LENGTH
        + 2
        + CONNECTION_UPGRADE_HEADER.len()
        + 2
        + UPGRADE_MPCLIPBOARD_RAW_HEADER.len()
        + 2
        + 2;
}
