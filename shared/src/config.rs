use boml::{Toml, prelude::TomlErrorKind, table::TomlGetError, types::TomlValueType};
use core::{mem::MaybeUninit, str::Utf8Error};
use rustix::{
    fs::{Mode, OFlags},
    io::Errno,
};

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<const N: usize, T>(
        path: &[u8],
        buffer: &mut [MaybeUninit<u8>],
        keys: [&'static str; N],
        f: impl FnOnce([&str; N]) -> T,
    ) -> Result<T, ConfigParserError> {
        let toml = read_toml(path, buffer)?;

        let mut values = [""; N];

        for (key, slot) in keys.iter().zip(values.iter_mut()) {
            let value = toml.get_string(key).map_err(|err| match err {
                TomlGetError::InvalidKey => ConfigParserError::MissingKey(key),
                TomlGetError::TypeMismatch(_, found) => ConfigParserError::NotAString(key, found),
            })?;

            *slot = value;
        }

        Ok(f(values))
    }
}

fn read_toml<'a>(
    path: &[u8],
    buffer: &'a mut [MaybeUninit<u8>],
) -> Result<Toml<'a>, ConfigParserError> {
    let fd =
        rustix::fs::open(path, OFlags::RDONLY, Mode::empty()).map_err(ConfigParserError::Open)?;
    let len = buffer.len();
    let (bytes, spare) = rustix::io::read(&fd, buffer).map_err(ConfigParserError::Read)?;
    if spare.is_empty() {
        return Err(ConfigParserError::TooLarge(len));
    }
    let text = core::str::from_utf8(bytes)?;

    boml::parse(text).map_err(|err| ConfigParserError::MalformedToml(err.kind, err.src.start))
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConfigParserError {
    #[error("failed to open() config: {0:?}")]
    Open(Errno),
    #[error("failed to read() config: {0:?}")]
    Read(Errno),
    #[error("config must be smaller than {0} bytes")]
    TooLarge(usize),
    #[error("config must be valid utf-8: {0}")]
    NonUtf8(#[from] Utf8Error),
    #[error("failed to parse TOML config: {0:?} at byte {1}")]
    MalformedToml(TomlErrorKind, usize),
    #[error("key {0} is missing in toml")]
    MissingKey(&'static str),
    #[error("key {0} must be a string in toml, got {1:?}")]
    NotAString(&'static str, TomlValueType),
}
