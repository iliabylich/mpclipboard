//! Trimmed copy of `polling` 3.11.0 (`src/epoll.rs`), see `README.md`.
//!
//! Bindings to epoll (Linux, Android).

use std::io;
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, RawFd};
use std::time::Instant;

use rustix::buffer::spare_capacity;
use rustix::event::{Timespec, epoll};
use rustix::fd::OwnedFd;

use super::{Event, PollMode};

/// Interface to epoll.
#[derive(Debug)]
pub struct Poller {
    /// File descriptor for the epoll instance.
    epoll_fd: OwnedFd,
}

impl Poller {
    /// Creates a new poller.
    pub fn new() -> io::Result<Poller> {
        // Create an epoll instance.
        //
        // Use `epoll_create1` with `EPOLL_CLOEXEC`.
        let epoll_fd = epoll::create(epoll::CreateFlags::CLOEXEC)?;

        Ok(Poller { epoll_fd })
    }

    /// Adds a new file descriptor.
    ///
    /// # Safety
    ///
    /// The `fd` must be a valid file descriptor. The usual condition of remaining registered in
    /// the `Poller` doesn't apply to `epoll`.
    pub unsafe fn add(&self, fd: RawFd, ev: Event, mode: PollMode) -> io::Result<()> {
        epoll::add(
            &self.epoll_fd,
            unsafe { rustix::fd::BorrowedFd::borrow_raw(fd) },
            epoll::EventData::new_u64(ev.key as u64),
            epoll_flags(&ev, mode) | ev.extra.flags,
        )?;

        Ok(())
    }

    /// Modifies an existing file descriptor.
    pub fn modify(&self, fd: BorrowedFd<'_>, ev: Event, mode: PollMode) -> io::Result<()> {
        epoll::modify(
            &self.epoll_fd,
            fd,
            epoll::EventData::new_u64(ev.key as u64),
            epoll_flags(&ev, mode) | ev.extra.flags,
        )?;

        Ok(())
    }

    /// Deletes a file descriptor.
    #[inline(always)]
    pub fn delete(&self, fd: BorrowedFd<'_>) -> io::Result<()> {
        epoll::delete(&self.epoll_fd, fd)?;

        Ok(())
    }

    /// Waits for I/O events with an optional deadline.
    ///
    /// Patched: upstream re-arms an internal timerfd before and an eventfd notifier after every
    /// `epoll_wait`. Here the timeout goes straight to `epoll_wait` (upstream's Redox path).
    pub fn wait_deadline(&self, events: &mut Events, deadline: Option<Instant>) -> io::Result<()> {
        let timeout = deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));

        // Timeout for epoll. In case of overflow, use no timeout.
        let timeout = timeout.and_then(|t| Timespec::try_from(t).ok());

        // Wait for I/O events.
        epoll::wait(
            &self.epoll_fd,
            spare_capacity(&mut events.list),
            timeout.as_ref(),
        )?;

        Ok(())
    }
}

impl AsRawFd for Poller {
    fn as_raw_fd(&self) -> RawFd {
        self.epoll_fd.as_raw_fd()
    }
}

impl AsFd for Poller {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.epoll_fd.as_fd()
    }
}

/// Get the EPOLL flags for the interest.
fn epoll_flags(interest: &Event, mode: PollMode) -> epoll::EventFlags {
    let mut flags = match mode {
        PollMode::Oneshot => epoll::EventFlags::ONESHOT,
        PollMode::Level => epoll::EventFlags::empty(),
        PollMode::Edge => epoll::EventFlags::ET,
        PollMode::EdgeOneshot => epoll::EventFlags::ET | epoll::EventFlags::ONESHOT,
    };
    if interest.readable {
        flags |= read_flags();
    }
    if interest.writable {
        flags |= write_flags();
    }
    flags
}

/// Epoll flags for all possible readability events.
fn read_flags() -> epoll::EventFlags {
    use epoll::EventFlags as Epoll;
    Epoll::IN | Epoll::HUP | Epoll::ERR | Epoll::PRI
}

/// Epoll flags for all possible writability events.
fn write_flags() -> epoll::EventFlags {
    use epoll::EventFlags as Epoll;
    Epoll::OUT | Epoll::HUP | Epoll::ERR
}

/// A list of reported I/O events.
pub struct Events {
    list: Vec<epoll::Event>,
}

unsafe impl Send for Events {}

impl Events {
    /// Creates an empty list.
    pub fn with_capacity(cap: usize) -> Events {
        Events {
            list: Vec::with_capacity(cap),
        }
    }

    /// Iterates over I/O events.
    pub fn iter(&self) -> impl Iterator<Item = Event> + '_ {
        self.list.iter().map(|ev| {
            let flags = ev.flags;
            Event {
                key: ev.data.u64() as usize,
                readable: flags.intersects(read_flags()),
                writable: flags.intersects(write_flags()),
                extra: EventExtra { flags },
            }
        })
    }

    /// Clear the list.
    pub fn clear(&mut self) {
        self.list.clear();
    }
}

/// Extra information about this event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventExtra {
    flags: epoll::EventFlags,
}

impl EventExtra {
    /// Create an empty version of the data.
    #[inline]
    pub const fn empty() -> EventExtra {
        EventExtra {
            flags: epoll::EventFlags::empty(),
        }
    }

    /// Tell if the interrupt flag is set.
    #[inline]
    pub fn is_hup(&self) -> bool {
        self.flags.contains(epoll::EventFlags::HUP)
    }

    /// Tell if there is an error.
    #[inline]
    pub fn is_err(&self) -> Option<bool> {
        Some(self.flags.contains(epoll::EventFlags::ERR))
    }
}
