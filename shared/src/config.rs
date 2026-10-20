use crate::{ID, NonEmptyStringError, Token, Url, UrlParseError};
use boml::{table::TomlGetError, types::TomlValueType};
use core::{mem::MaybeUninit, str::Utf8Error};
use rustix::{
    fs::{Mode, OFlags},
    io::Errno,
};
use std::path::{Path, PathBuf};

pub struct ConfigReader {
    text: Option<String>,
}

impl ConfigReader {
    const BUFFER_SIZE: usize = 1_024;

    pub fn new() -> Result<Self, ConfigReaderError> {
        for path in Self::paths() {
            log::error!("trying {}...", path.display());
            let Some(text) = read(&path)? else {
                continue;
            };

            if let Err(err) = boml::parse(&text) {
                return Err(ConfigReaderError::MalformedToml(err.src.start));
            }

            return Ok(Self { text: Some(text) });
        }

        Ok(Self { text: None })
    }

    pub fn get_url(&self) -> Result<Url, ConfigUrlError> {
        let url = self.get("MPCLIPBOARD_URL", "url")?;
        let url = Url::parse(&url)?;
        Ok(url)
    }

    pub fn get_token(&self) -> Result<Token, ConfigTokenError> {
        let token = self.get("MPCLIPBOARD_TOKEN", "token")?;
        let token = Token::new(token)?;
        Ok(token)
    }

    pub fn get_id(&self) -> Result<ID, ConfigIdError> {
        let id = self.get("MPCLIPBOARD_ID", "id")?;
        let id = ID::new(id)?;
        Ok(id)
    }

    fn get(&self, env_var_name: &str, config_key: &str) -> Result<String, LookupError> {
        if let Some(value) = std::env::var_os(env_var_name) {
            return value.into_string().map_err(|_| LookupError::NonUtf8Env);
        }

        let Some(text) = &self.text else {
            return Err(LookupError::Missing);
        };
        let toml = boml::parse(text)
            .unwrap_or_else(|_| unreachable!("config is validated in ConfigReader::new()"));

        match toml.get_string(config_key) {
            Ok(value) => Ok(value.to_string()),
            Err(TomlGetError::InvalidKey) => Err(LookupError::Missing),
            Err(TomlGetError::TypeMismatch(_, found)) => Err(LookupError::NotAString(found)),
        }
    }

    fn paths() -> impl Iterator<Item = PathBuf> {
        let xdg = std::env::var_os("XDG_CONFIG_HOME")
            .map(|dir| PathBuf::from(dir).join("mpclipboard").join("config.toml"));
        let home = std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("mpclipboard")
                .join("config.toml")
        });
        let etc = Some(PathBuf::from("/etc/mpclipboard-server/config.toml"));

        [xdg, home, etc].into_iter().flatten()
    }
}

fn read(path: &Path) -> Result<Option<String>, ConfigReaderError> {
    let fd = match rustix::fs::open(path, OFlags::RDONLY, Mode::empty()) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return Ok(None),
        Err(errno) => return Err(ConfigReaderError::Open(errno)),
    };

    let mut buffer = [MaybeUninit::uninit(); ConfigReader::BUFFER_SIZE];
    let (bytes, spare) = rustix::io::read(&fd, &mut buffer).map_err(ConfigReaderError::Read)?;
    if spare.is_empty() {
        return Err(ConfigReaderError::TooLarge(ConfigReader::BUFFER_SIZE));
    }

    let text = core::str::from_utf8(bytes)?;
    Ok(Some(text.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigReaderError {
    #[error("failed to open() config: {0:?}")]
    Open(Errno),
    #[error("failed to read() config: {0:?}")]
    Read(Errno),
    #[error("config must be smaller than {0} bytes")]
    TooLarge(usize),
    #[error("config must be valid utf-8: {0}")]
    NonUtf8(#[from] Utf8Error),
    #[error("failed to parse TOML config at byte {0}")]
    MalformedToml(usize),
}

#[derive(Debug, Clone, Copy)]
enum LookupError {
    Missing,
    NonUtf8Env,
    NotAString(TomlValueType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigUrlError {
    #[error("url is missing: set $MPCLIPBOARD_URL or add it to the config file")]
    Missing,
    #[error("$MPCLIPBOARD_URL must be valid utf-8")]
    NonUtf8Env,
    #[error("url must be a string in toml, got {0:?}")]
    NotAString(TomlValueType),
    #[error("malformed url: {0}")]
    Parse(#[from] UrlParseError),
}

impl From<LookupError> for ConfigUrlError {
    fn from(err: LookupError) -> Self {
        match err {
            LookupError::Missing => Self::Missing,
            LookupError::NonUtf8Env => Self::NonUtf8Env,
            LookupError::NotAString(found) => Self::NotAString(found),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigTokenError {
    #[error("token is missing: set $MPCLIPBOARD_TOKEN or add it to the config file")]
    Missing,
    #[error("$MPCLIPBOARD_TOKEN must be valid utf-8")]
    NonUtf8Env,
    #[error("token must be a string in toml, got {0:?}")]
    NotAString(TomlValueType),
    #[error("malformed token: {0}")]
    Invalid(#[from] NonEmptyStringError),
}

impl From<LookupError> for ConfigTokenError {
    fn from(err: LookupError) -> Self {
        match err {
            LookupError::Missing => Self::Missing,
            LookupError::NonUtf8Env => Self::NonUtf8Env,
            LookupError::NotAString(found) => Self::NotAString(found),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigIdError {
    #[error("id is missing: set $MPCLIPBOARD_ID or add it to the config file")]
    Missing,
    #[error("$MPCLIPBOARD_ID must be valid utf-8")]
    NonUtf8Env,
    #[error("id must be a string in toml, got {0:?}")]
    NotAString(TomlValueType),
    #[error("malformed id: {0}")]
    Invalid(#[from] NonEmptyStringError),
}

impl From<LookupError> for ConfigIdError {
    fn from(err: LookupError) -> Self {
        match err {
            LookupError::Missing => Self::Missing,
            LookupError::NonUtf8Env => Self::NonUtf8Env,
            LookupError::NotAString(found) => Self::NotAString(found),
        }
    }
}
