use crate::{constants::TLS_BUFFER_SIZE, tls_stream::TlsError};

pub(super) struct Incoming {
    buf: Box<[u8; TLS_BUFFER_SIZE]>,
    len: usize,
}

impl Incoming {
    pub(super) fn new() -> Self {
        Self {
            buf: Box::new([0; TLS_BUFFER_SIZE]),
            len: 0,
        }
    }

    pub(super) fn filled_mut(&mut self) -> &mut [u8] {
        &mut self.buf[..self.len]
    }

    pub(super) fn spare_mut(&mut self) -> Result<&mut [u8], TlsError> {
        let spare = &mut self.buf[self.len..];
        if spare.is_empty() {
            return Err(TlsError::IncomingBufferFull);
        }
        Ok(spare)
    }

    pub(super) fn filled(&mut self, len: usize) {
        self.len += len;
    }

    pub(super) fn discard(&mut self, len: usize) {
        self.buf.copy_within(len..self.len, 0);
        self.len -= len;
    }
}
