use crate::prelude::*;
use core::num::NonZeroUsize;
use rustix::io::Errno;
use std::os::fd::AsFd;

pub fn read(fd: impl AsFd, buf: &mut [u8]) -> Completion<NonZeroUsize, anyhow::Error, ()> {
    match rustix::io::read(fd, buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => Done(len),
        Err(Errno::AGAIN) => Pending(()),
        Ok(None) => Failed(anyhow::anyhow!("failed to read(): EOF")),
        Err(errno) => Failed(anyhow::anyhow!("failed to read(): {errno:?}")),
    }
}

pub fn write(fd: impl AsFd, buf: &[u8]) -> Completion<NonZeroUsize, anyhow::Error, ()> {
    match rustix::io::write(fd, buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => Done(len),
        Err(Errno::AGAIN) => Pending(()),
        Ok(None) => Failed(anyhow::anyhow!("failed to read(): EOF")),
        Err(errno) => Failed(anyhow::anyhow!("failed to read(): {errno:?}")),
    }
}
