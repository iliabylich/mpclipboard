use crate::{HostPort, ID, Token};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpgradeRequest {
    pub host: HostPort,
    pub token: Token,
    pub id: ID,
}
