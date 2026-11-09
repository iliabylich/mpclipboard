use crate::Stderr;
use core::fmt::Write as _;
use log::{LevelFilter, Log, Metadata, Record};

pub(crate) struct Logger;

static LOGGER: Logger = Logger;

impl Logger {
    pub(crate) fn init() -> Result<(), ()> {
        if log::set_logger(&LOGGER).is_err() {
            let _ = writeln!(Stderr, "logger is already initialized");
            return Err(());
        }

        let level = match crate::env::var(c"RUST_LOG")? {
            None => LevelFilter::Error,
            Some(value) => match value.parse::<LevelFilter>() {
                Ok(level) => level,
                Err(err) => {
                    let _ = writeln!(Stderr, "invalid RUST_LOG={value:?}: {err}");
                    return Err(());
                }
            },
        };
        log::set_max_level(level);

        Ok(())
    }
}

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        let _ = writeln!(Stderr, "{} {}", record.level(), record.args());
    }

    fn flush(&self) {}
}
