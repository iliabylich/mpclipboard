use anyhow::{Context, Result, ensure};
use mpclipboard_shared::{ConfigParser, Token, Url};

#[derive(Clone, Copy)]
pub struct Config {
    pub(crate) url: Url,
    pub(crate) token: Token,
}

const PATH: &[u8] = if cfg!(debug_assertions) {
    b"config.toml"
} else {
    b"/etc/mpclipboard-server/config.toml"
};

impl Config {
    pub(crate) fn read() -> Result<Self> {
        ConfigParser::parse(PATH, &mut [0; 1_024], ["url", "token"], |[url, token]| {
            let url = Url::parse(url).context("malformed url")?;
            ensure!(!url.is_tls(), "url must have http scheme");

            let token = Token::new(token).context("token is too long")?;

            Ok(Self { url, token })
        })
        .context("failed to parse config")?
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
