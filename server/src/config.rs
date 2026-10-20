use anyhow::{Context, Result, ensure};
use mpclipboard_shared::{ConfigReader, Token, Url};

#[derive(Clone)]
pub struct Config {
    pub(crate) url: Url,
    pub(crate) token: Token,
}

impl Config {
    pub(crate) fn read() -> Result<Self> {
        let reader = ConfigReader::new().context("failed to read config")?;

        let url = reader.get_url()?;
        ensure!(!url.is_tls(), "url must have http scheme");

        let token = reader.get_token()?;

        Ok(Self { url, token })
    }
}

impl core::fmt::Debug for Config {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.url)
            .field("token", &"******")
            .finish()
    }
}
