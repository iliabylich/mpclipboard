//! Trimmed copy of `polling` 3.11.0 (`src/lib.rs`), see `README.md`.

#![allow(
    unsafe_op_in_unsafe_fn,
    trivial_casts,
    trivial_numeric_casts,
    unused_qualifications,
    clippy::pedantic,
    clippy::nursery,
    clippy::std_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use std::fmt;
use std::io;
use std::time::{Duration, Instant};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod epoll;
#[cfg(any(target_os = "linux", target_os = "android"))]
use epoll as sys;

#[cfg(target_os = "macos")]
mod kqueue;
#[cfg(target_os = "macos")]
use kqueue as sys;

pub mod os;

/// Indicates that a file descriptor or socket can read or write without blocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Key identifying the file descriptor or socket.
    pub key: usize,
    /// Can it do a read operation without blocking?
    pub readable: bool,
    /// Can it do a write operation without blocking?
    pub writable: bool,
    /// System-specific event data.
    extra: sys::EventExtra,
}

/// The mode in which the poller waits for I/O events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum PollMode {
    /// Poll in oneshot mode.
    Oneshot,
    /// Poll in level-triggered mode.
    Level,
    /// Poll in edge-triggered mode.
    Edge,
    /// Poll in both edge-triggered and oneshot mode.
    EdgeOneshot,
}

impl Event {
    /// Create a new event.
    pub const fn new(key: usize, readable: bool, writable: bool) -> Event {
        Event {
            key,
            readable,
            writable,
            extra: sys::EventExtra::empty(),
        }
    }

    /// Creates an event that requests both readable and writable interest.
    #[inline]
    pub const fn all(key: usize) -> Event {
        Event::new(key, true, true)
    }

    /// Creates an event that requests readable interest.
    #[inline]
    pub const fn readable(key: usize) -> Event {
        Event::new(key, true, false)
    }

    /// Creates an event that requests writable interest.
    #[inline]
    pub const fn writable(key: usize) -> Event {
        Event::new(key, false, true)
    }

    /// Creates an event that requests no interest.
    #[cfg(target_os = "macos")]
    #[inline]
    pub const fn none(key: usize) -> Event {
        Event::new(key, false, false)
    }

    /// Tell if this event is the result of an interrupt notification.
    #[inline]
    pub fn is_interrupt(&self) -> bool {
        self.extra.is_hup()
    }

    /// Tells if this event is the result of an error.
    ///
    /// Returns `None` if the platform doesn't report this information.
    #[inline]
    pub fn is_err(&self) -> Option<bool> {
        self.extra.is_err()
    }
}

/// Waits for I/O events.
pub struct Poller {
    poller: sys::Poller,
}

impl Poller {
    /// Creates a new poller.
    pub fn new() -> io::Result<Poller> {
        Ok(Poller {
            poller: sys::Poller::new()?,
        })
    }

    /// Adds a file descriptor or socket to the poller in the specified mode.
    ///
    /// # Safety
    ///
    /// The source must be [`delete()`](Poller::delete)d from this `Poller` before it is dropped.
    pub unsafe fn add_with_mode(
        &self,
        source: impl AsRawSource,
        interest: Event,
        mode: PollMode,
    ) -> io::Result<()> {
        self.poller.add(source.raw(), interest, mode)
    }

    /// Modifies interest in a file descriptor or socket to the poller, using the specified mode.
    pub fn modify_with_mode(
        &self,
        source: impl AsSource,
        interest: Event,
        mode: PollMode,
    ) -> io::Result<()> {
        self.poller.modify(source.source(), interest, mode)
    }

    /// Removes a file descriptor or socket from the poller.
    pub fn delete(&self, source: impl AsSource) -> io::Result<()> {
        self.poller.delete(source.source())
    }

    /// Waits for at least one I/O event and returns the number of new events.
    pub fn wait(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<usize> {
        self.wait_impl(
            events,
            timeout.and_then(|timeout| Instant::now().checked_add(timeout)),
        )
    }

    fn wait_impl(&self, events: &mut Events, deadline: Option<Instant>) -> io::Result<usize> {
        loop {
            // Wait for I/O events.
            if let Err(e) = self.poller.wait_deadline(&mut events.events, deadline) {
                // If the wait was interrupted by a signal, clear events and try again.
                if e.kind() == io::ErrorKind::Interrupted {
                    events.clear();
                    continue;
                } else {
                    return Err(e);
                }
            }

            // Indicate number of events.
            return Ok(events.len());
        }
    }
}

/// A container for I/O events.
pub struct Events {
    events: sys::Events,
}

impl Default for Events {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Events {
    /// Create a new container for events, using the default capacity.
    #[inline]
    pub fn new() -> Self {
        const DEFAULT_CAPACITY: usize = 1024;
        Self {
            events: sys::Events::with_capacity(DEFAULT_CAPACITY),
        }
    }

    /// Create a new iterator over I/O events.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Event> + '_ {
        self.events.iter()
    }

    /// Delete all of the events in the container.
    #[inline]
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Returns the number of events in the container.
    #[inline]
    pub fn len(&self) -> usize {
        self.iter().count()
    }
}

impl fmt::Debug for Events {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Events { .. }")
    }
}

mod raw_fd_impl {
    use super::Poller;
    use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd};

    impl AsRawFd for Poller {
        fn as_raw_fd(&self) -> RawFd {
            self.poller.as_raw_fd()
        }
    }

    impl AsFd for Poller {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.poller.as_fd()
        }
    }
}

impl fmt::Debug for Poller {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.poller.fmt(f)
    }
}

use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd};

/// A resource with a raw file descriptor.
pub trait AsRawSource {
    /// Returns the raw file descriptor.
    fn raw(&self) -> RawFd;
}

impl<T: AsRawFd> AsRawSource for &T {
    fn raw(&self) -> RawFd {
        self.as_raw_fd()
    }
}

impl AsRawSource for RawFd {
    fn raw(&self) -> RawFd {
        *self
    }
}

/// A resource with a borrowed file descriptor.
pub trait AsSource: AsFd {
    /// Returns the borrowed file descriptor.
    fn source(&self) -> BorrowedFd<'_> {
        self.as_fd()
    }
}

impl<T: AsFd> AsSource for T {}
