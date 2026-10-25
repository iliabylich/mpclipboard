//! Trimmed copy of `polling` 3.11.0 (`src/os/kqueue.rs`), see `README.md`.
//!
//! Functionality that is only available for `kqueue`-based platforms.

use super::super::kqueue::{SourceId, mode_to_flags};
use super::super::{PollMode, Poller};

use std::io;
use std::time::Duration;

use rustix::event::kqueue;

use super::__private::PollerSealed;
use __private::FilterSealed;

/// Functionality that is only available for `kqueue`-based platforms.
pub trait PollerKqueueExt<F: Filter>: PollerSealed {
    /// Add a filter to the poller.
    fn add_filter(&self, filter: F, key: usize, mode: PollMode) -> io::Result<()>;

    /// Modify an existing filter.
    fn modify_filter(&self, filter: F, key: usize, mode: PollMode) -> io::Result<()>;
}

impl<F: Filter> PollerKqueueExt<F> for Poller {
    #[inline(always)]
    fn add_filter(&self, filter: F, key: usize, mode: PollMode) -> io::Result<()> {
        // No difference between adding and modifying in kqueue.
        self.poller.add_source(filter.source_id())?;
        self.modify_filter(filter, key, mode)
    }

    fn modify_filter(&self, filter: F, key: usize, mode: PollMode) -> io::Result<()> {
        self.poller.has_source(filter.source_id())?;

        // Convert the filter into a kevent.
        let event = filter.filter(kqueue::EventFlags::ADD | mode_to_flags(mode), key);

        // Modify the filter.
        self.poller.submit_changes([event])
    }
}

/// A filter that can be registered into a `kqueue`.
pub trait Filter: FilterSealed {}

unsafe impl<T: FilterSealed + ?Sized> FilterSealed for &T {
    #[inline(always)]
    fn filter(&self, flags: kqueue::EventFlags, key: usize) -> kqueue::Event {
        (**self).filter(flags, key)
    }

    #[inline(always)]
    fn source_id(&self) -> SourceId {
        (**self).source_id()
    }
}

impl<T: Filter + ?Sized> Filter for &T {}

/// A timer that wakes up after a certain amount of time.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timer {
    /// The ID of the timer.
    ///
    /// This is not the same as the key; it is used to identify the timer.
    pub id: usize,

    /// The timeout to wait for.
    pub timeout: Duration,
}

unsafe impl FilterSealed for Timer {
    fn filter(&self, flags: kqueue::EventFlags, key: usize) -> kqueue::Event {
        kqueue::Event::new(
            kqueue::EventFilter::Timer {
                ident: self.id as _,
                timer: Some(self.timeout),
            },
            flags | kqueue::EventFlags::RECEIPT,
            key as _,
        )
    }

    #[inline(always)]
    fn source_id(&self) -> SourceId {
        SourceId::Timer(self.id)
    }
}

impl Filter for Timer {}

mod __private {
    use super::super::super::kqueue::SourceId;
    use rustix::event::kqueue;

    #[doc(hidden)]
    pub unsafe trait FilterSealed {
        /// Get the filter for the given event.
        ///
        /// This filter's flags must have `EV_RECEIPT`.
        fn filter(&self, flags: kqueue::EventFlags, key: usize) -> kqueue::Event;

        /// Get the source ID for this source.
        fn source_id(&self) -> SourceId;
    }
}
