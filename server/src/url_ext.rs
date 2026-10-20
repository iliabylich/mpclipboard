use anyhow::{Context as _, Result};
use core::net::{SocketAddr, SocketAddrV4};
use mpclipboard_shared::Url;
use std::net::ToSocketAddrs;

pub trait UrlExt {
    fn resolve(&self) -> Result<SocketAddrV4>;
}

impl UrlExt for Url {
    fn resolve(&self) -> Result<SocketAddrV4> {
        let mut addrs = (self.host(), self.port())
            .to_socket_addrs()
            .context("failed to resolve URL")?;

        addrs
            .find_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .context("can't resolve URL to IPv4 address")
    }
}
