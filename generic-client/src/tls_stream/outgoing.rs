use crate::constants::TLS_BUFFER_SIZE;

pub(super) struct Outgoing {
    buf: Box<[u8; TLS_BUFFER_SIZE]>,
    len: usize,
    sent: usize,
}

impl Outgoing {
    pub(super) fn new() -> Self {
        Self {
            buf: Box::new([0; TLS_BUFFER_SIZE]),
            len: 0,
            sent: 0,
        }
    }

    pub(super) fn spare_mut(&mut self) -> &mut [u8] {
        &mut self.buf[self.len..]
    }

    pub(super) fn filled(&mut self, len: usize) {
        self.len += len;
    }

    pub(super) fn pending(&self) -> &[u8] {
        &self.buf[self.sent..self.len]
    }

    pub(super) fn sent(&mut self, len: usize) {
        self.sent += len;
        if self.sent == self.len {
            self.sent = 0;
            self.len = 0;
        }
    }
}
