use crate::{Stderr, errors::ServerError};
use core::{
    fmt::Write as _,
    sync::atomic::{AtomicU8, Ordering},
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

impl Logger {
    pub(crate) fn init() -> Result<(), ServerError> {
        let level = match crate::env::var(c"RUST_LOG")? {
            None => LevelFilter::Error,
            Some(value) => LevelFilter::parse(value).ok_or(ServerError::InvalidRustLog(value))?,
        };
        MAX_LEVEL.store(level as u8, Ordering::Relaxed);

        Ok(())
    }

    #[inline(never)]
    pub(crate) fn log(level: Level, args: core::fmt::Arguments<'_>) {
        if level as u8 > MAX_LEVEL.load(Ordering::Relaxed) {
            return;
        }
        let _ = writeln!(Stderr, "{} {args}", level.as_str());
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
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Info => "INFO",
            Self::Trace => "TRACE",
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
