use mpclipboard_shared::{
    ConfigParser, ConfigParserError, ID, NonEmptyInlineStringError, PROTOCOL_VERSION, Token,
    UpgradeRequest, Url, UrlParseError,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub struct Config {
    pub(crate) url: Url,
    pub(crate) token: Token,
    pub(crate) id: ID,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.url)
            .field("token", &"******")
            .field("id", &self.id)
            .finish()
    }
}

impl Config {
    pub(crate) fn new(url: &str, token: &str, id: &str) -> Result<Self, ConfigError> {
        let url = Url::parse(url).map_err(ConfigError::Url)?;
        let token = Token::new(token).map_err(ConfigError::Token)?;
        let id = ID::new(id).map_err(ConfigError::Id)?;

        Ok(Self { url, token, id })
    }

    fn read(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        ConfigParser::parse(
            path.as_ref().as_os_str().as_encoded_bytes(),
            &mut [0; 1_024],
            ["url", "token", "id"],
            |[url, token, id]| Self::new(url, token, id),
        )
        .map_err(ConfigError::Parse)?
    }

    pub(crate) fn read_local_file() -> Result<Self, ConfigError> {
        Self::read("config.toml")
    }

    pub(crate) fn read_in_xdg_config_dir() -> Result<Self, ConfigError> {
        let config_dir = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .ok_or(ConfigError::NoConfigDir)?;

        let path = config_dir.join("mpclipboard").join("config.toml");
        Self::read(path)
    }

    pub(crate) const fn update_request(&self) -> UpgradeRequest {
        UpgradeRequest {
            host: self.url.header(),
            token: self.token,
            id: self.id,
            version: PROTOCOL_VERSION,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    Url(UrlParseError),
    Token(NonEmptyInlineStringError),
    Id(NonEmptyInlineStringError),
    Parse(ConfigParserError),
    NoConfigDir,
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Url(err) => write!(f, "malformed url: {err}"),
            Self::Token(err) => write!(f, "malformed token: {err}"),
            Self::Id(err) => write!(f, "malformed id: {err}"),
            Self::Parse(err) => write!(f, "failed to parse config: {err}"),
            Self::NoConfigDir => write!(f, "neither $XDG_CONFIG_HOME nor $HOME is set"),
        }
    }
}

impl core::error::Error for ConfigError {}
