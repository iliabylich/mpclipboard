use boml::{Toml, prelude::TomlErrorKind, table::TomlGetError, types::TomlValueType};
use core::str::Utf8Error;
use rustix::{
    fs::{Mode, OFlags},
    io::Errno,
};

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse<const N: usize, T>(
        path: &[u8],
        buffer: &mut [u8],
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

fn read_toml<'a>(path: &[u8], buffer: &'a mut [u8]) -> Result<Toml<'a>, ConfigParserError> {
    let fd =
        rustix::fs::open(path, OFlags::RDONLY, Mode::empty()).map_err(ConfigParserError::Open)?;
    let len = rustix::io::read(&fd, &mut *buffer).map_err(ConfigParserError::Read)?;
    let Some(bytes) = buffer.get(..len) else {
        unreachable!("read() can't return more than buffer.len() bytes");
    };
    let text = core::str::from_utf8(bytes).map_err(ConfigParserError::NonUtf8)?;

    boml::parse(text).map_err(|err| ConfigParserError::MalformedToml(err.kind, err.src.start))
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigParserError {
    Open(Errno),
    Read(Errno),
    NonUtf8(Utf8Error),
    MalformedToml(TomlErrorKind, usize),
    MissingKey(&'static str),
    NotAString(&'static str, TomlValueType),
}

impl core::fmt::Display for ConfigParserError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Open(errno) => write!(f, "failed to open() config: {errno:?}"),
            Self::Read(errno) => write!(f, "failed to read() config: {errno:?}"),
            Self::NonUtf8(err) => write!(f, "config must be valid utf-8: {err}"),
            Self::MalformedToml(kind, offset) => {
                write!(f, "failed to parse TOML config: {kind:?} at byte {offset}")
            }
            Self::MissingKey(key) => write!(f, "key {key} is missing in toml"),
            Self::NotAString(key, found) => {
                write!(f, "key {key} must be a string in toml, got {found:?}")
            }
        }
    }
}

impl core::error::Error for ConfigParserError {}
