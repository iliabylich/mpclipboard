use rustix::io::Errno;

use crate::{
    Completion::{self, *},
    error,
};
use core::num::NonZeroUsize;
use std::os::fd::AsFd;

pub fn read(fd: impl AsFd, buf: &mut [u8]) -> Completion<NonZeroUsize, ()> {
    match rustix::io::read(fd, buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => Done(len),
        Err(Errno::AGAIN) => Pending(()),
        Ok(None) => {
            error!("failed to read(): EOF");
            Failed
        }
        Err(errno) => {
            error!("failed to read(): {errno:?}");
            Failed
        }
    }
}

pub fn write(fd: impl AsFd, buf: &[u8]) -> Completion<NonZeroUsize, ()> {
    match rustix::io::write(fd, buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => Done(len),
        Err(Errno::AGAIN) => Pending(()),
        Ok(None) => {
            error!("failed to write(): EOF");
            Failed
        }
        Err(errno) => {
            error!("failed to write(): {errno:?}");
            Failed
        }
    }
}
