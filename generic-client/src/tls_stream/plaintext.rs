use crate::{constants::TLS_BUFFER_SIZE, tls_stream::TlsError};

pub(super) struct Plaintext {
    buf: Box<[u8; TLS_BUFFER_SIZE]>,
    start: usize,
    end: usize,
}

impl Plaintext {
    pub(super) fn new() -> Self {
        Self {
            buf: Box::new([0; TLS_BUFFER_SIZE]),
            start: 0,
            end: 0,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<(), TlsError> {
        if self.buf.len() - self.end < bytes.len() {
            self.buf.copy_within(self.start..self.end, 0);
            self.end -= self.start;
            self.start = 0;
        }
        let Some(spare) = self.buf.get_mut(self.end..self.end + bytes.len()) else {
            return Err(TlsError::PlaintextBufferFull);
        };
        spare.copy_from_slice(bytes);
        self.end += bytes.len();
        Ok(())
    }

    pub(super) fn take(&mut self, out: &mut [u8]) -> usize {
        let available = &self.buf[self.start..self.end];
        let len = available.len().min(out.len());
        out[..len].copy_from_slice(&available[..len]);
        self.start += len;
        if self.start == self.end {
            self.start = 0;
            self.end = 0;
        }
        len
    }
}
