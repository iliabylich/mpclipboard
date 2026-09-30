use crate::{Buffer, prelude::*};
use core::num::NonZeroUsize;
use rustix::io::Errno;
use std::os::fd::AsFd;

pub fn read<const N: usize>(fd: impl AsFd) -> Completion<Buffer<N>, ReadError, ()> {
    let mut buf = [0; N];
    match rustix::io::read(fd, &mut buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => {
            let Some(buf) = buf.get(..len.get()).and_then(Buffer::from_slice) else {
                unreachable!("read() can't return more than N bytes");
            };
            Done(buf)
        }
        Err(Errno::AGAIN) => Pending(()),
        Ok(None) => Failed(ReadError::EOF),
        Err(errno) => Failed(ReadError::Errno(errno)),
    }
}

/// # Panics
///
/// Panics if `buf` is empty.
pub fn write(fd: impl AsFd, buf: &[u8]) -> Completion<NonZeroUsize, WriteError, ()> {
    assert!(!buf.is_empty(), "can't write an empty buffer");

    match rustix::io::write(fd, buf) {
        Ok(len) => {
            let Some(len) = NonZeroUsize::new(len) else {
                unreachable!("write() of a non-empty buffer never returns 0");
            };
            Done(len)
        }
        Err(Errno::AGAIN) => Pending(()),
        Err(errno) => Failed(WriteError(errno)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    EOF,
    Errno(Errno),
}

impl core::fmt::Display for ReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EOF => write!(f, "failed to read(): EOF"),
            Self::Errno(errno) => write!(f, "failed to read(): {errno:?}"),
        }
    }
}

impl core::error::Error for ReadError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteError(pub Errno);

impl core::fmt::Display for WriteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to write(): {:?}", self.0)
    }
}

impl core::error::Error for WriteError {}
