//! Trimmed copy of `polling` 3.11.0 (`src/os.rs`), see `README.md`.
//!
//! Platform-specific functionality.

#[cfg(target_os = "macos")]
pub mod kqueue;

mod __private {
    #[doc(hidden)]
    #[allow(dead_code)]
    pub trait PollerSealed {}

    impl PollerSealed for super::super::Poller {}
}
