mod epoch;
pub use epoch::Epoch;

mod error;
pub use error::EventLoopError;

mod result;
pub use result::{EventLoopFdResult, EventLoopResult};

mod fd_state;
pub use fd_state::{Diff, FdState};

mod new;
pub use new::{
    EventLoopWithTimer, EventLoopWithTimerError, EventLoopWithTimerResult, RegisteredFd,
};

#[cfg(any(target_os = "linux", target_os = "android"))]
mod epoll;
#[cfg(any(target_os = "linux", target_os = "android"))]
pub use epoll::EventLoop;

#[cfg(target_os = "macos")]
mod kqueue;
#[cfg(target_os = "macos")]
pub use kqueue::EventLoop;
