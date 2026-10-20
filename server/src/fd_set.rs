use crate::reaper::CanBeReaped;
use mpclipboard_shared::Now;
use std::{
    collections::HashMap,
    os::fd::{AsFd, AsRawFd, RawFd},
};

pub struct FdSet<const MAX: usize, T: AsFd> {
    map: HashMap<i32, T>,
}

impl<const MAX: usize, T: AsFd> FdSet<MAX, T> {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, value: T) {
        if self.map.len() < MAX {
            self.map.insert(value.as_fd().as_raw_fd(), value);
        }
    }

    pub(crate) fn remove(&mut self, fd: i32) -> Option<T> {
        self.map.remove(&fd)
    }

    pub(crate) fn fds_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.map.values_mut()
    }
}

impl<const MAX: usize, T: AsFd + CanBeReaped> FdSet<MAX, T> {
    pub(crate) fn reap(&mut self, now: Now) {
        let mut fds_to_drop = vec![];
        for (fd, stranger_source) in &self.map {
            if stranger_source.must_be_reaped(now) {
                fds_to_drop.push(*fd);
            }
        }
        for fd in fds_to_drop {
            self.map.remove(&fd);
        }
    }
}

impl<const MAX: usize, T: AsFd> Default for FdSet<MAX, T> {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn key(fd: &impl AsFd) -> usize {
    usize::try_from(fd.as_fd().as_raw_fd())
        .unwrap_or_else(|_| unreachable!("open fds are never negative"))
}

pub(crate) fn fd_from_key(key: usize) -> RawFd {
    RawFd::try_from(key).unwrap_or_else(|_| unreachable!("keys are always created from fds"))
}
