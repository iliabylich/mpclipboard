use crate::url::{Url, UrlParseError};
use boml::{Toml, table::TomlGetError, types::TomlValueType};
use mpclipboard_shared2::{FixedStrError, ID, Token};
use std::path::{Path, PathBuf};

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
        let mut text = None;
        for path in paths() {
            log::info!("trying {}...", path.display());
            if let Some(found) = read(&path)? {
                text = Some(found);
                break;
            }
        }

        let toml = text
            .as_deref()
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
    let url = get("MPCLIPBOARD_URL", toml, "url")?;
    let url = Url::parse(&url)?;
    Ok(url)
}

fn get_token(toml: Option<&Toml<'_>>) -> Result<Token, ConfigTokenError> {
    let token = get("MPCLIPBOARD_TOKEN", toml, "token")?;
    let token = Token::new(&token)?;
    Ok(token)
}

fn get_id(toml: Option<&Toml<'_>>) -> Result<ID, ConfigIdError> {
    let id = get("MPCLIPBOARD_ID", toml, "id")?;
    let id = ID::new(&id)?;
    Ok(id)
}

fn get(
    env_var_name: &'static str,
    toml: Option<&Toml<'_>>,
    config_key: &'static str,
) -> Result<String, LookupError> {
    from_env(env_var_name)
        .transpose()
        .or_else(|| toml.and_then(|toml| from_toml(toml, config_key).transpose()))
        .unwrap_or(Err(LookupError::Missing {
            env_var_name,
            config_key,
        }))
}

fn from_env(env_var_name: &'static str) -> Result<Option<String>, LookupError> {
    let Some(value) = std::env::var_os(env_var_name) else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_| LookupError::NonUtf8Env { env_var_name })?;
    Ok(Some(value))
}

fn from_toml(toml: &Toml<'_>, config_key: &'static str) -> Result<Option<String>, LookupError> {
    match toml.get_string(config_key) {
        Ok(value) => Ok(Some(value.to_string())),
        Err(TomlGetError::InvalidKey) => Ok(None),
        Err(TomlGetError::TypeMismatch(_, found)) => {
            Err(LookupError::NotAString { config_key, found })
        }
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

    [xdg, home].into_iter().flatten()
}

fn read(path: &Path) -> Result<Option<String>, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(ConfigError::Read(err)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(std::io::Error),
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
    #[error("{config_key} is missing: set ${env_var_name} or add it to the config file")]
    Missing {
        env_var_name: &'static str,
        config_key: &'static str,
    },
    #[error("${env_var_name} must be valid utf-8")]
    NonUtf8Env { env_var_name: &'static str },
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
