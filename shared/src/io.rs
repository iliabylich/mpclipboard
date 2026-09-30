use crate::{Buffer, prelude::*};
use core::num::NonZeroUsize;
use rustix::io::Errno;
use std::os::fd::AsFd;

pub fn read<const N: usize>(fd: impl AsFd) -> Completion<Buffer<N>, anyhow::Error, ()> {
    let mut buf = [0; N];
    match rustix::io::read(fd, &mut buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => {
            let Some(buf) = buf.get(..len.get()).and_then(Buffer::from_slice) else {
                unreachable!("read() can't return more than N bytes");
            };
            Done(buf)
        }
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
