#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Transmit,
    Plaintext,
    Blocked,
    Writable,
    PeerClosed,
}
