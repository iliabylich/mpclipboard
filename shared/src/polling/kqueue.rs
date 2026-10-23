//! Trimmed copy of `polling` 3.11.0 (`src/kqueue.rs`), see `README.md`.
//!
//! Bindings to kqueue (macOS).

use std::collections::HashSet;
use std::io;
use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::sync::RwLock;
use std::time::Instant;

use rustix::buffer::spare_capacity;
use rustix::event::{Timespec, kqueue};
use rustix::io::{Errno, FdFlags, fcntl_setfd};

use super::{Event, PollMode};

/// Interface to kqueue.
#[derive(Debug)]
pub struct Poller {
    /// File descriptor for the kqueue instance.
    kqueue_fd: OwnedFd,

    /// List of sources currently registered in this poller.
    ///
    /// This is used to make sure the same source is not registered twice.
    sources: RwLock<HashSet<SourceId>>,
}

/// Identifier for a source.
#[doc(hidden)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum SourceId {
    /// Registered file descriptor.
    Fd(RawFd),

    /// Timer ID.
    Timer(usize),
}

impl Poller {
    /// Creates a new poller.
    ///
    /// Patched: upstream also registers an `EVFILT_USER` notifier here.
    pub fn new() -> io::Result<Poller> {
        // Create a kqueue instance.
        let kqueue_fd = kqueue::kqueue()?;
        fcntl_setfd(&kqueue_fd, FdFlags::CLOEXEC)?;

        Ok(Poller {
            kqueue_fd,
            sources: RwLock::new(HashSet::new()),
        })
    }

    /// Adds a new file descriptor.
    ///
    /// # Safety
    ///
    /// The file descriptor must be valid and it must last until it is deleted.
    pub unsafe fn add(&self, fd: RawFd, ev: Event, mode: PollMode) -> io::Result<()> {
        self.add_source(SourceId::Fd(fd))?;

        // File descriptors don't need to be added explicitly, so just modify the interest.
        self.modify(BorrowedFd::borrow_raw(fd), ev, mode)
    }

    /// Modifies an existing file descriptor.
    pub fn modify(&self, fd: BorrowedFd<'_>, ev: Event, mode: PollMode) -> io::Result<()> {
        self.has_source(SourceId::Fd(fd.as_raw_fd()))?;

        let mode_flags = mode_to_flags(mode);

        let read_flags = if ev.readable {
            kqueue::EventFlags::ADD | mode_flags
        } else {
            kqueue::EventFlags::DELETE
        };
        let write_flags = if ev.writable {
            kqueue::EventFlags::ADD | mode_flags
        } else {
            kqueue::EventFlags::DELETE
        };

        // A list of changes for kqueue.
        let changelist = [
            kqueue::Event::new(
                kqueue::EventFilter::Read(fd.as_raw_fd()),
                read_flags | kqueue::EventFlags::RECEIPT,
                ev.key as _,
            ),
            kqueue::Event::new(
                kqueue::EventFilter::Write(fd.as_raw_fd()),
                write_flags | kqueue::EventFlags::RECEIPT,
                ev.key as _,
            ),
        ];

        // Apply changes.
        self.submit_changes(changelist)
    }

    /// Submit one or more changes to the kernel queue and check to see if they succeeded.
    pub(crate) fn submit_changes<A>(&self, changelist: A) -> io::Result<()>
    where
        A: Copy + AsRef<[kqueue::Event]> + AsMut<[kqueue::Event]>,
    {
        let mut eventlist = Vec::with_capacity(changelist.as_ref().len());

        // Apply changes.
        {
            let changelist = changelist.as_ref();

            unsafe {
                kqueue::kevent_timespec(
                    &self.kqueue_fd,
                    changelist,
                    spare_capacity(&mut eventlist),
                    None,
                )?;
            }
        }

        // Check for errors.
        for &ev in &eventlist {
            let data = ev.data();

            // Explanation for ignoring EPIPE: https://github.com/tokio-rs/mio/issues/582
            if (ev.flags().contains(kqueue::EventFlags::ERROR))
                && data != 0
                && data != Errno::NOENT.raw_os_error() as _
                && data != Errno::PIPE.raw_os_error() as _
            {
                return Err(io::Error::from_raw_os_error(data as _));
            }
        }

        Ok(())
    }

    /// Add a source to the sources set.
    #[inline]
    pub(crate) fn add_source(&self, source: SourceId) -> io::Result<()> {
        if self
            .sources
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(source)
        {
            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::AlreadyExists))
        }
    }

    /// Tell if a source is currently inside the set.
    #[inline]
    pub(crate) fn has_source(&self, source: SourceId) -> io::Result<()> {
        if self
            .sources
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&source)
        {
            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
    }

    /// Remove a source from the sources set.
    #[inline]
    pub(crate) fn remove_source(&self, source: SourceId) -> io::Result<()> {
        if self
            .sources
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&source)
        {
            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
    }

    /// Deletes a file descriptor.
    pub fn delete(&self, fd: BorrowedFd<'_>) -> io::Result<()> {
        // Simply delete interest in the file descriptor.
        self.modify(fd, Event::none(0), PollMode::Oneshot)?;

        self.remove_source(SourceId::Fd(fd.as_raw_fd()))
    }

    /// Waits for I/O events with an optional deadline.
    ///
    /// Patched: upstream re-registers its notifier afterwards (a no-op on macOS).
    pub fn wait_deadline(&self, events: &mut Events, deadline: Option<Instant>) -> io::Result<()> {
        let timeout = deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));

        // Timeout for kevent. In case of overflow, use no timeout.
        let timeout = match timeout {
            Some(t) => Timespec::try_from(t).ok(),
            None => None,
        };

        // Wait for I/O events.
        let changelist = [];
        unsafe {
            kqueue::kevent_timespec(
                &self.kqueue_fd,
                &changelist,
                spare_capacity(&mut events.list),
                timeout.as_ref(),
            )?
        };

        Ok(())
    }
}

impl AsRawFd for Poller {
    fn as_raw_fd(&self) -> RawFd {
        self.kqueue_fd.as_raw_fd()
    }
}

impl AsFd for Poller {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.kqueue_fd.as_fd()
    }
}

/// A list of reported I/O events.
pub struct Events {
    list: Vec<kqueue::Event>,
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
        // On some platforms, closing the read end of a pipe wakes up writers, but the
        // event is reported as EVFILT_READ with the EV_EOF flag.
        //
        // https://github.com/golang/go/commit/23aad448b1e3f7c3b4ba2af90120bde91ac865b4
        self.list.iter().map(|ev| Event {
            key: ev.udata() as usize,
            readable: matches!(
                ev.filter(),
                kqueue::EventFilter::Read(..)
                    | kqueue::EventFilter::Vnode { .. }
                    | kqueue::EventFilter::Proc { .. }
                    | kqueue::EventFilter::Signal { .. }
                    | kqueue::EventFilter::Timer { .. }
            ),
            writable: matches!(ev.filter(), kqueue::EventFilter::Write(..))
                || (matches!(ev.filter(), kqueue::EventFilter::Read(..))
                    && (ev.flags().intersects(kqueue::EventFlags::EOF))),
            extra: EventExtra,
        })
    }

    /// Clears the list.
    pub fn clear(&mut self) {
        self.list.clear();
    }
}

/// Extra information associated with an event.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EventExtra;

impl EventExtra {
    /// Create a new, empty version of this struct.
    #[inline]
    pub const fn empty() -> EventExtra {
        EventExtra
    }

    /// Is the interrupt flag set?
    #[inline]
    pub fn is_hup(&self) -> bool {
        false
    }

    /// Is there an error?
    #[inline]
    pub fn is_err(&self) -> Option<bool> {
        None
    }
}

pub(crate) fn mode_to_flags(mode: PollMode) -> kqueue::EventFlags {
    use kqueue::EventFlags as EV;

    match mode {
        PollMode::Oneshot => EV::ONESHOT,
        PollMode::Level => EV::empty(),
        PollMode::Edge => EV::CLEAR,
        PollMode::EdgeOneshot => EV::ONESHOT | EV::CLEAR,
    }
}
