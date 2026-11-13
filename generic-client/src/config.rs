use crate::{
    constants::MAX_CONFIG_SIZE,
    url::{Url, UrlParseError},
};
use boml::{Toml, table::TomlGetError, types::TomlValueType};
use core::{ffi::CStr, str::Utf8Error};
use mpclipboard_shared::{
    FixedStrError, ID, Token,
    env::{self, EnvVarError},
};
use rustix::{
    fd::OwnedFd,
    fs::{Mode, OFlags},
    io::Errno,
};

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) url: Url,
    pub(crate) token: Token,
    pub(crate) id: ID,
}

impl Config {
    pub(crate) fn new(url: &str, token: &str, id: &str) -> Result<Self, ConfigError> {
        let url = Url::parse(url).map_err(ConfigUrlError::Parse)?;
        let token = Token::new(token).map_err(ConfigTokenError::Invalid)?;
        let id = ID::new(id).map_err(ConfigIdError::Invalid)?;
        Ok(Self { url, token, id })
    }

    pub(crate) fn new_from_file() -> Result<Self, ConfigError> {
        let mut buf = [0; MAX_CONFIG_SIZE + 1];
        let text = match open()? {
            Some(fd) => Some(read(&fd, &mut buf)?),
            None => None,
        };

        let toml = text
            .map(boml::parse)
            .transpose()
            .map_err(|err| ConfigError::MalformedToml(err.src.start))?;

        let url = get_url(toml.as_ref())?;
        let token = get_token(toml.as_ref())?;
        let id = get_id(toml.as_ref())?;

        Ok(Self { url, token, id })
    }
}

fn get_url(toml: Option<&Toml<'_>>) -> Result<Url, ConfigUrlError> {
    let url = get(c"MPCLIPBOARD_URL", toml, "url")?;
    let url = Url::parse(url)?;
    Ok(url)
}

fn get_token(toml: Option<&Toml<'_>>) -> Result<Token, ConfigTokenError> {
    let token = get(c"MPCLIPBOARD_TOKEN", toml, "token")?;
    let token = Token::new(token)?;
    Ok(token)
}

fn get_id(toml: Option<&Toml<'_>>) -> Result<ID, ConfigIdError> {
    let id = get(c"MPCLIPBOARD_ID", toml, "id")?;
    let id = ID::new(id)?;
    Ok(id)
}

fn get<'a>(
    env_var_name: &'static CStr,
    toml: Option<&'a Toml<'_>>,
    config_key: &'static str,
) -> Result<&'a str, LookupError> {
    from_env(env_var_name)
        .transpose()
        .or_else(|| toml.and_then(|toml| from_toml(toml, config_key).transpose()))
        .unwrap_or(Err(LookupError::Missing {
            env_var_name,
            config_key,
        }))
}

fn from_env(env_var_name: &'static CStr) -> Result<Option<&'static str>, LookupError> {
    let value = env::var(env_var_name)?;
    Ok(value)
}

fn from_toml<'a>(
    toml: &'a Toml<'_>,
    config_key: &'static str,
) -> Result<Option<&'a str>, LookupError> {
    match toml.get_string(config_key) {
        Ok(value) => Ok(Some(value)),
        Err(TomlGetError::InvalidKey) => Ok(None),
        Err(TomlGetError::TypeMismatch(_, found)) => {
            Err(LookupError::NotAString { config_key, found })
        }
    }
}

fn open() -> Result<Option<OwnedFd>, ConfigError> {
    let xdg = env::var(c"XDG_CONFIG_HOME")?.map(|dir| (dir, "mpclipboard/config.toml"));
    let home = env::var(c"HOME")?.map(|home| (home, ".config/mpclipboard/config.toml"));

    for (dir, file) in [xdg, home].into_iter().flatten() {
        log::info!("trying {dir}/{file}...");
        if let Some(fd) = open_in(dir, file).map_err(ConfigError::Read)? {
            return Ok(Some(fd));
        }
    }
    Ok(None)
}

fn open_in(dir: &str, file: &str) -> Result<Option<OwnedFd>, Errno> {
    let flags = OFlags::RDONLY | OFlags::CLOEXEC;
    let dir = match rustix::fs::open(dir, flags, Mode::empty()) {
        Ok(dir) => dir,
        Err(Errno::NOENT) => return Ok(None),
        Err(err) => return Err(err),
    };
    match rustix::fs::openat(&dir, file, flags, Mode::empty()) {
        Ok(fd) => Ok(Some(fd)),
        Err(Errno::NOENT) => Ok(None),
        Err(err) => Err(err),
    }
}

fn read<'a>(fd: &OwnedFd, buf: &'a mut [u8; MAX_CONFIG_SIZE + 1]) -> Result<&'a str, ConfigError> {
    let mut len = 0;
    while len < buf.len() {
        match rustix::io::read(fd, &mut buf[len..]) {
            Ok(0) => break,
            Ok(read) => len += read,
            Err(Errno::INTR) => {}
            Err(err) => return Err(ConfigError::Read(err)),
        }
    }
    if len > MAX_CONFIG_SIZE {
        return Err(ConfigError::ConfigIsTooLong);
    }

    let text = core::str::from_utf8(&buf[..len])?;
    Ok(text)
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(transparent)]
    NonUtf8Env(#[from] EnvVarError),
    #[error("failed to read config: {0}")]
    Read(Errno),
    #[error("config is too long (max is {MAX_CONFIG_SIZE} bytes)")]
    ConfigIsTooLong,
    #[error("config is not valid utf-8: {0}")]
    NonUtf8(#[from] Utf8Error),
    #[error("failed to parse TOML config at byte {0}")]
    MalformedToml(usize),
    #[error("invalid url: {0}")]
    Url(#[from] ConfigUrlError),
    #[error("invalid token: {0}")]
    Token(#[from] ConfigTokenError),
    #[error("invalid id: {0}")]
    Id(#[from] ConfigIdError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LookupError {
    #[error(
        "{config_key} is missing: set ${} or add it to the config file",
        env_var_name.to_string_lossy()
    )]
    Missing {
        env_var_name: &'static CStr,
        config_key: &'static str,
    },
    #[error(transparent)]
    NonUtf8Env(#[from] EnvVarError),
    #[error("{config_key} must be a string in toml, got {found:?}")]
    NotAString {
        config_key: &'static str,
        found: TomlValueType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigUrlError {
    #[error(transparent)]
    Lookup(#[from] LookupError),
    #[error("malformed url: {0}")]
    Parse(#[from] UrlParseError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigTokenError {
    #[error(transparent)]
    Lookup(#[from] LookupError),
    #[error("malformed token: {0}")]
    Invalid(#[from] FixedStrError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigIdError {
    #[error(transparent)]
    Lookup(#[from] LookupError),
    #[error("malformed id: {0}")]
    Invalid(#[from] FixedStrError),
}
