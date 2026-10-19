use mpclipboard_shared::{
    ConfigIdError, ConfigReader, ConfigReaderError, ConfigTokenError, ConfigUrlError, ID,
    PROTOCOL_VERSION, Token, UpgradeRequest, Url,
};

#[derive(Clone, Copy)]
pub(crate) struct Config {
    pub(crate) url: Url,
    pub(crate) token: Token,
    pub(crate) id: ID,
}

impl core::fmt::Debug for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.url)
            .field("token", &"******")
            .field("id", &self.id)
            .finish()
    }
}

impl Config {
    pub(crate) fn new(url: &str, token: &str, id: &str) -> Result<Self, ConfigError> {
        let url = Url::parse(url).map_err(ConfigUrlError::Parse)?;
        let token = Token::new(token).map_err(ConfigTokenError::Invalid)?;
        let id = ID::new(id).map_err(ConfigIdError::Invalid)?;

        Ok(Self { url, token, id })
    }

    pub(crate) fn new_from_file() -> Result<Self, ConfigError> {
        let reader = ConfigReader::new()?;

        let url = reader.get_url()?;
        let token = reader.get_token()?;
        let id = reader.get_id()?;

        Ok(Self { url, token, id })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] ConfigReaderError),
    #[error("invalid url: {0}")]
    Url(#[from] ConfigUrlError),
    #[error("invalid token: {0}")]
    Token(#[from] ConfigTokenError),
    #[error("invalid id: {0}")]
    Id(#[from] ConfigIdError),
}
