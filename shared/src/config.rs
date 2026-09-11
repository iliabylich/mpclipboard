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
        let fd = rustix::fs::open(path, OFlags::RDONLY, Mode::empty())
            .map_err(ConfigParserError::Open)?;
        let len = rustix::io::read(&fd, &mut *buffer).map_err(ConfigParserError::Read)?;
        let bytes = buffer
            .get(..len)
            .unwrap_or_else(|| unreachable!("read() returned malformed data"));
        let text = str::from_utf8(bytes).map_err(ConfigParserError::InvalidUtf8)?;

        let toml = boml::parse(text).map_err(|_| ConfigParserError::InvalidToml)?;

        let mut values = [""; N];

        for (key, slot) in keys.iter().zip(values.iter_mut()) {
            let value = toml
                .get_string(key)
                .map_err(|_| ConfigParserError::InvalidTomlValueForKey(key))?;

            *slot = value;
        }

        Ok(f(values))
    }
}

#[derive(Debug)]
pub enum ConfigParserError {
    Open(Errno),
    Read(Errno),
    InvalidUtf8(core::str::Utf8Error),
    InvalidToml,
    InvalidTomlValueForKey(&'static str),
}

impl core::fmt::Display for ConfigParserError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Open(error) => write!(f, "failed to open config: {error}"),
            Self::Read(error) => write!(f, "failed to read config: {error}"),
            Self::InvalidUtf8(error) => write!(f, "config is not valid UTF-8: {error}"),
            Self::InvalidToml => f.write_str("config is not valid TOML"),
            Self::InvalidTomlValueForKey(key) => write!(f, "failed to get config value `{key}`"),
        }
    }
}

impl core::error::Error for ConfigParserError {}
