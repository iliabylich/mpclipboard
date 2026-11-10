use crate::{Stderr, errors::ServerError};
use core::{
    fmt::Write as _,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

macro_rules! error {
    ($($arg:tt)+) => {
        $crate::logger::Logger::log($crate::logger::Level::Error, format_args!($($arg)+))
    };
}

macro_rules! info {
    ($($arg:tt)+) => {
        $crate::logger::Logger::log($crate::logger::Level::Info, format_args!($($arg)+))
    };
}

macro_rules! trace {
    ($($arg:tt)+) => {
        $crate::logger::Logger::log($crate::logger::Level::Trace, format_args!($($arg)+))
    };
}

pub(crate) struct Logger;

static MAX_LEVEL: AtomicU8 = AtomicU8::new(LevelFilter::Off as u8);
static COLORED: AtomicBool = AtomicBool::new(false);

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const NC: &str = "\x1b[0m";

impl Logger {
    pub(crate) fn init() -> Result<(), ServerError> {
        let level = match crate::env::var(c"RUST_LOG")? {
            None => LevelFilter::Error,
            Some(value) => LevelFilter::parse(value).ok_or(ServerError::InvalidRustLog(value))?,
        };
        MAX_LEVEL.store(level as u8, Ordering::Relaxed);

        let stderr = unsafe { rustix::stdio::stderr() };
        COLORED.store(rustix::termios::isatty(stderr), Ordering::Relaxed);

        Ok(())
    }

    #[inline(never)]
    pub(crate) fn log(level: Level, args: core::fmt::Arguments<'_>) {
        if level as u8 > MAX_LEVEL.load(Ordering::Relaxed) {
            return;
        }
        let (color, level, nc) = level.pre_level_post();
        let _ = writeln!(Stderr, "{color}{level}{nc} {args}");
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub(crate) enum Level {
    Error = LevelFilter::Error as u8,
    Info = LevelFilter::Info as u8,
    Trace = LevelFilter::Trace as u8,
}

impl Level {
    fn pre_level_post(self) -> (&'static str, &'static str, &'static str) {
        let colored = COLORED.load(Ordering::Relaxed);

        let (pre, level, post) = match self {
            Self::Error => (RED, "ERROR", NC),
            Self::Info => (GREEN, "INFO", NC),
            Self::Trace => (CYAN, "TRACE", NC),
        };

        if colored {
            (pre, level, post)
        } else {
            ("", level, "")
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum LevelFilter {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LevelFilter {
    fn parse(s: &str) -> Option<Self> {
        [
            ("off", Self::Off),
            ("error", Self::Error),
            ("warn", Self::Warn),
            ("info", Self::Info),
            ("debug", Self::Debug),
            ("trace", Self::Trace),
        ]
        .into_iter()
        .find_map(|(name, level)| s.eq_ignore_ascii_case(name).then_some(level))
    }
}
