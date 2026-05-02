use anyhow::{Context as _, Result, bail};
use http_serde::http::Uri;
use serde::{Deserialize, Serialize};
use std::{
    net::{SocketAddr, ToSocketAddrs as _},
    path::PathBuf,
    str::FromStr,
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Instruction for the `Config::read` function how to read the config.
pub enum ConfigReadOption {
    /// Read from "./config.toml", based on your current working directory
    FromLocalFile = 0,

    /// Read from XDG Config dir (i.e. from `~/.config/mpclipboard/config.toml`)
    FromXdgConfigDir = 1,
}

impl ConfigReadOption {
    fn path(self) -> Result<String> {
        match self {
            Self::FromLocalFile => Ok("config.toml".to_string()),
            Self::FromXdgConfigDir => std::env::var("$XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .context("no $XDG_CONFIG_HOME is set")
                .or_else(|_err| {
                    let home = std::env::var("HOME").context("no $HOME")?;
                    Result::<_, anyhow::Error>::Ok(PathBuf::from(home).join(".config"))
                })
                .context("neither $XDG_CONFIG_HOME nor $HOME is set")?
                .join("mpclipboard")
                .join("config.toml")
                .to_str()
                .context("non-utf8 $XDG_CONFIG_HOME or $HOME")
                .map(ToString::to_string),
        }
    }
}

/// Representation of a runtime configuration
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    /// URI of the WebSocket server
    /// (e.g. `"ws://127.0.0.1:3000"` or `"wss://mpclipboard.me.dev"`)
    #[serde(with = "http_serde::uri")]
    pub uri: Uri,

    /// Token that is used for authentication
    pub token: String,

    /// Unique name of the client
    /// (e.g. `"macos-old-laptop"` or `"linux-dusty-minipc"`)
    pub name: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("uri", &self.uri)
            .field("token", &"******")
            .field("name", &self.name)
            .finish()
    }
}

impl Config {
    /// Constructs a new config in-place based on given fields.
    ///
    /// # Errors
    ///
    /// Return error if given `uri` is not a valid URI.
    pub fn new(uri: &str, token: String, name: String) -> Result<Self> {
        log::trace!("config: {uri} {token} {name}");
        Ok(Self {
            uri: Uri::from_str(uri).context("invalid URI")?,
            token,
            name,
        })
    }

    /// Reads the config based on the given instruction
    /// (which is either "read from XDG dir" or "read from ./config.toml")
    ///
    /// # Errors
    ///
    /// Returns an error if a file that given `option` is mapped to
    /// doesn't exist or is invalid.
    pub fn read(option: ConfigReadOption) -> Result<Self> {
        let path = option.path()?;
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("failed to read {path}"))?;
        toml::from_str(&content).context("invalid config format")
    }

    pub(crate) fn enable_tls(&self) -> Result<bool> {
        match self.uri.scheme_str() {
            Some("ws") => Ok(false),
            Some("wss") => Ok(true),
            _ => bail!("expected either ws:// or wss:// scheme"),
        }
    }

    pub(crate) fn remote_addr(&self) -> Result<SocketAddr> {
        let enable_tls = self.enable_tls()?;

        let host = self.uri.host().context("no host")?;
        let port = self
            .uri
            .port_u16()
            .unwrap_or(if enable_tls { 443 } else { 80 });

        let addrs = (host, port).to_socket_addrs()?;
        let mut ipv6 = None;
        let mut ipv4 = None;
        for addr in addrs {
            if addr.is_ipv4() {
                ipv4 = Some(addr);
            } else {
                ipv6 = Some(addr);
            }
        }
        let addr = ipv6.or(ipv4).context("failed to resolve DNS")?;
        Ok(addr)
    }
}
