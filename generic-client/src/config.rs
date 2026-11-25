use crate::{
    constants::MAX_CONFIG_SIZE,
    toml::{Toml, TomlError},
    url::{Url, UrlParseError},
};
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

        let toml = text.map(Toml::parse).transpose()?.unwrap_or_default();

        let url = get_url(&toml)?;
        let token = get_token(&toml)?;
        let id = get_id(&toml)?;

        Ok(Self { url, token, id })
    }
}

fn get_url(toml: &Toml<'_>) -> Result<Url, ConfigUrlError> {
    let url = get(c"MPCLIPBOARD_URL", toml.url, "url")?;
    let url = Url::parse(url)?;
    Ok(url)
}

fn get_token(toml: &Toml<'_>) -> Result<Token, ConfigTokenError> {
    let token = get(c"MPCLIPBOARD_TOKEN", toml.token, "token")?;
    let token = Token::new(token)?;
    Ok(token)
}

fn get_id(toml: &Toml<'_>) -> Result<ID, ConfigIdError> {
    let id = get(c"MPCLIPBOARD_ID", toml.id, "id")?;
    let id = ID::new(id)?;
    Ok(id)
}

fn get<'a>(
    env_var_name: &'static CStr,
    from_toml: Option<&'a str>,
    config_key: &'static str,
) -> Result<&'a str, LookupError> {
    let from_env = env::var(env_var_name)?;
    from_env.or(from_toml).ok_or(LookupError::Missing {
        env_var_name,
        config_key,
    })
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
    #[error("failed to parse TOML config: {0}")]
    MalformedToml(#[from] TomlError),
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
