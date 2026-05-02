/// Connectivity of the `MPClipboard`, emitted in `on_connectivity_changed`
#[derive(Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Connectivity {
    /// Connecting to remote server, performing handshake/auth
    Connecting,
    /// Connected, ready to talk
    Connected,
    /// Disconnected
    Disconnected,
}
