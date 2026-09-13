#[must_use]
pub struct UpgradeResponse;

impl UpgradeResponse {
    pub(crate) const BYTES: &[u8] = b"HTTP/1.1 101 Switching Protocols\r\n\
Connection: Upgrade\r\n\
Upgrade: mpclipboard-raw\r\n\
\r\n";
}
