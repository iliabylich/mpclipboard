use crate::{
    EventLoopWithTimerError,
    polling::{PollMode, Poller},
};
use alloc::rc::Rc;
use core::time::Duration;

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) use linux::Timer;
#[cfg(target_os = "macos")]
pub(crate) use macos::Timer;

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux {
    use super::{Duration, EventLoopWithTimerError, PollMode, Poller, Rc};
    use crate::polling::Event;
    use rustix::{
        fd::{AsFd, OwnedFd},
        fs::Timespec,
        time::{
            Itimerspec, TimerfdClockId, TimerfdFlags, TimerfdTimerFlags, timerfd_create,
            timerfd_settime,
        },
    };

    pub(crate) struct Timer {
        fd: OwnedFd,
        poller: Rc<Poller>,
    }

    impl Timer {
        pub(crate) fn new(poller: Rc<Poller>, key: usize) -> Result<Self, EventLoopWithTimerError> {
            let fd = timerfd_create(
                TimerfdClockId::Monotonic,
                TimerfdFlags::CLOEXEC | TimerfdFlags::NONBLOCK,
            )
            .map_err(EventLoopWithTimerError::CreateTimer)?;
            unsafe { poller.add_with_mode(&fd.as_fd(), Event::readable(key), PollMode::Level) }
                .map_err(EventLoopWithTimerError::AddTimer)?;

            let timer = Self { fd, poller };
            timer.set(Duration::ZERO)?;
            Ok(timer)
        }

        pub(crate) fn set(&self, timeout: Duration) -> Result<(), EventLoopWithTimerError> {
            let timeout = timeout.max(Duration::from_nanos(1));
            timerfd_settime(
                &self.fd,
                TimerfdTimerFlags::empty(),
                &Itimerspec {
                    it_interval: Timespec {
                        tv_sec: 0,
                        tv_nsec: 0,
                    },
                    it_value: Timespec {
                        tv_sec: i64::try_from(timeout.as_secs()).unwrap_or(i64::MAX),
                        tv_nsec: timeout.subsec_nanos().into(),
                    },
                },
            )
            .map_err(|errno| EventLoopWithTimerError::SetTimer(errno.into()))?;
            Ok(())
        }
    }

    impl Drop for Timer {
        fn drop(&mut self) {
            let _ = self.poller.delete(&self.fd);
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{Duration, EventLoopWithTimerError, PollMode, Poller, Rc};
    use crate::polling::os::kqueue::PollerKqueueExt;

    pub(crate) struct Timer {
        poller: Rc<Poller>,
        key: usize,
    }

    impl Timer {
        pub(crate) fn new(poller: Rc<Poller>, key: usize) -> Result<Self, EventLoopWithTimerError> {
            poller
                .add_filter(kqueue_timer(key, Duration::ZERO), key, PollMode::Oneshot)
                .map_err(EventLoopWithTimerError::AddTimer)?;
            Ok(Self { poller, key })
        }

        pub(crate) fn set(&self, timeout: Duration) -> Result<(), EventLoopWithTimerError> {
            self.poller
                .modify_filter(kqueue_timer(self.key, timeout), self.key, PollMode::Oneshot)
                .map_err(EventLoopWithTimerError::SetTimer)?;
            Ok(())
        }
    }

    fn kqueue_timer(key: usize, timeout: Duration) -> crate::polling::os::kqueue::Timer {
        let millis = timeout.as_nanos().div_ceil(1_000_000).max(1);
        crate::polling::os::kqueue::Timer {
            id: key,
            timeout: Duration::from_millis(u64::try_from(millis).unwrap_or(u64::MAX)),
        }
    }
}
