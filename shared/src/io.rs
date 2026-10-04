use crate::{Buffer, prelude::*};
use core::num::NonZeroUsize;
use rustix::{fd::AsFd, io::Errno, net::SendFlags};

#[cfg(target_os = "macos")]
pub const SEND_FLAGS: SendFlags = SendFlags::empty();
#[cfg(not(target_os = "macos"))]
pub const SEND_FLAGS: SendFlags = SendFlags::NOSIGNAL;

pub fn read<const N: usize>(fd: impl AsFd) -> Result<Completion<Buffer<N>, ()>, ReadError> {
    let mut buf = [0; N];
    match rustix::io::read(fd, &mut buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => {
            let buf = buf
                .get(..len.get())
                .and_then(Buffer::from_slice)
                .unwrap_or_else(|| unreachable!("read() can't return more than N bytes"));
            Ok(Done(buf))
        }
        Err(Errno::AGAIN) => Ok(Pending(())),
        Ok(None) => Err(ReadError::EOF),
        Err(errno) => Err(ReadError::Errno(errno)),
    }
}

/// # Panics
///
/// Panics if `buf` is empty.
pub fn write(fd: impl AsFd, buf: &[u8]) -> Result<Completion<NonZeroUsize, ()>, WriteError> {
    assert!(!buf.is_empty(), "can't write an empty buffer");

    match rustix::net::send(fd, buf, SEND_FLAGS) {
        Ok(len) => {
            let len = NonZeroUsize::new(len)
                .unwrap_or_else(|| unreachable!("write() of a non-empty buffer never returns 0"));
            Ok(Done(len))
        }
        Err(Errno::AGAIN) => Ok(Pending(())),
        Err(errno) => Err(WriteError(errno)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    #[error("failed to read(): EOF")]
    EOF,
    #[error("failed to read(): {0:?}")]
    Errno(Errno),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("failed to write(): {0:?}")]
pub struct WriteError(pub Errno);
