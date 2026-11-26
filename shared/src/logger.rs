use crate::env::EnvVarError;
use core::{ffi::CStr, fmt::Write as _};
use log::{Level, LevelFilter, Log, Metadata, Record};

#[cfg(not(target_os = "android"))]
use crate::{Stderr, env};
#[cfg(not(target_os = "android"))]
use core::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "android")]
use crate::FormatBuffer;
#[cfg(target_os = "android")]
use core::ffi::{c_char, c_int};

const RUST_LOG: &CStr = c"RUST_LOG";

#[cfg(target_os = "android")]
#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

static LOGGER: Logger = Logger;
#[cfg(not(target_os = "android"))]
static COLORED: AtomicBool = AtomicBool::new(false);

pub struct Logger;

impl Logger {
    pub fn init() -> Result<(), LoggerError> {
        let level = max_level()?;

        #[cfg(not(target_os = "android"))]
        {
            let stderr = unsafe { rustix::stdio::stderr() };
            COLORED.store(rustix::termios::isatty(stderr), Ordering::Relaxed);
        }

        log::set_logger(&LOGGER).map_err(|_| LoggerError::AlreadyInitialized)?;
        log::set_max_level(level);
        Ok(())
    }
}

#[cfg(not(target_os = "android"))]
fn max_level() -> Result<LevelFilter, LoggerError> {
    match env::var(RUST_LOG)? {
        None => Ok(LevelFilter::Error),
        Some(value) => value
            .parse()
            .map_err(|_| LoggerError::InvalidRustLog(value)),
    }
}

#[cfg(target_os = "android")]
fn max_level() -> Result<LevelFilter, LoggerError> {
    if cfg!(debug_assertions) {
        Ok(LevelFilter::Trace)
    } else {
        Ok(LevelFilter::Error)
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    #[cfg(not(target_os = "android"))]
    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let (color, level, nc) = pre_level_post(record.level());
        let _ = writeln!(Stderr, "{color}{level}{nc} {}", record.args());
    }

    #[cfg(target_os = "android")]
    fn log(&self, record: &Record<'_>) {
        const LOGCAT_TAG: &CStr = c"RUST";
        const LOGCAT_MESSAGE_SIZE: usize = 1024;

        if !self.enabled(record.metadata()) {
            return;
        }
        let mut message = FormatBuffer::<LOGCAT_MESSAGE_SIZE>::new();
        let _ = write!(message, "{}", record.args());
        let mut text = [0; LOGCAT_MESSAGE_SIZE + 1];
        let message = message.as_bytes();
        text[..message.len()].copy_from_slice(message);

        let prio = match record.level() {
            Level::Error => 6,
            Level::Warn => 5,
            Level::Info => 4,
            Level::Debug => 3,
            Level::Trace => 2,
        };
        unsafe {
            __android_log_write(prio, LOGCAT_TAG.as_ptr(), text.as_ptr().cast());
        }
    }

    fn flush(&self) {}
}

#[cfg(not(target_os = "android"))]
fn pre_level_post(level: Level) -> (&'static str, &'static str, &'static str) {
    const RED: &str = "\x1b[31m";
    const YELLOW: &str = "\x1b[33m";
    const GREEN: &str = "\x1b[32m";
    const BLUE: &str = "\x1b[34m";
    const CYAN: &str = "\x1b[36m";
    const NC: &str = "\x1b[0m";

    let color = match level {
        Level::Error => RED,
        Level::Warn => YELLOW,
        Level::Info => GREEN,
        Level::Debug => BLUE,
        Level::Trace => CYAN,
    };

    if COLORED.load(Ordering::Relaxed) {
        (color, level.as_str(), NC)
    } else {
        ("", level.as_str(), "")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggerError {
    NonUtf8EnvVar(EnvVarError),
    InvalidRustLog(&'static str),
    AlreadyInitialized,
}

impl From<EnvVarError> for LoggerError {
    fn from(err: EnvVarError) -> Self {
        Self::NonUtf8EnvVar(err)
    }
}

impl core::fmt::Display for LoggerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonUtf8EnvVar(err) => write!(f, "{err}"),
            Self::InvalidRustLog(value) => write!(f, "invalid {RUST_LOG:?}: {value}"),
            Self::AlreadyInitialized => write!(f, "logger is already initialized"),
        }
    }
}

impl core::error::Error for LoggerError {}
