use crate::{ID, NonEmptyStringError, Token, Url, UrlParseError};
use boml::{table::TomlGetError, types::TomlValueType};
use std::path::{Path, PathBuf};

pub struct ConfigReader {
    text: Option<String>,
}

impl ConfigReader {
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

    fn get(
        &self,
        env_var_name: &'static str,
        config_key: &'static str,
    ) -> Result<String, LookupError> {
        if let Some(value) = std::env::var_os(env_var_name) {
            return value
                .into_string()
                .map_err(|_| LookupError::NonUtf8Env { env_var_name });
        }

        let Some(text) = &self.text else {
            return Err(LookupError::Missing {
                env_var_name,
                config_key,
            });
        };
        let toml = boml::parse(text)
            .unwrap_or_else(|_| unreachable!("config is validated in ConfigReader::new()"));

        match toml.get_string(config_key) {
            Ok(value) => Ok(value.to_string()),
            Err(TomlGetError::InvalidKey) => Err(LookupError::Missing {
                env_var_name,
                config_key,
            }),
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
        let etc = Some(PathBuf::from("/etc/mpclipboard-server/config.toml"));

        [xdg, home, etc].into_iter().flatten()
    }
}

fn read(path: &Path) -> Result<Option<String>, ConfigReaderError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(ConfigReaderError::Read(err)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigReaderError {
    #[error("failed to read config: {0}")]
    Read(std::io::Error),
    #[error("failed to parse TOML config at byte {0}")]
    MalformedToml(usize),
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
    Invalid(#[from] NonEmptyStringError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigIdError {
    #[error(transparent)]
    Lookup(#[from] LookupError),
    #[error("malformed id: {0}")]
    Invalid(#[from] NonEmptyStringError),
}
