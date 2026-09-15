use anyhow::{Context, Result};
use mpclipboard_shared::{ConfigParser, ID, PROTOCOL_VERSION, Token, UpgradeRequest, Url};
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
    pub(crate) fn new(url: &str, token: &str, id: &str) -> Result<Self> {
        let url = Url::parse(url).context("malformed url")?;
        let token = Token::new(token).context("token is too long")?;
        let id = ID::new(id).context("id is too long")?;

        Ok(Self { url, token, id })
    }

    fn read(path: impl AsRef<Path>) -> Result<Self> {
        ConfigParser::parse(
            path.as_ref().as_os_str().as_encoded_bytes(),
            &mut [0; 1_024],
            ["url", "token", "id"],
            |[url, token, id]| Self::new(url, token, id),
        )
        .context("failed to parse config")?
    }

    pub(crate) fn read_local_file() -> Result<Self> {
        Self::read("config.toml")
    }

    pub(crate) fn read_in_xdg_config_dir() -> Result<Self> {
        let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .context("no $XDG_CONFIG_HOME is set")
            .or_else(|_err| {
                let home = std::env::var("HOME").context("no $HOME")?;
                Result::<_, anyhow::Error>::Ok(PathBuf::from(home).join(".config"))
            })
            .context("neither $XDG_CONFIG_HOME nor $HOME is set")?;

        let path = xdg_config_home.join("mpclipboard").join("config.toml");
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
