#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub enum Connectivity {
    Connecting,
    Connected,
    Disconnected,
}
