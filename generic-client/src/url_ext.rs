use core::net::{SocketAddr, SocketAddrV4};
use mpclipboard_shared::Url;
use std::net::ToSocketAddrs;

pub trait UrlExt {
    fn resolve(&self) -> Result<SocketAddrV4, UrlResolveError>;
}

impl UrlExt for Url {
    fn resolve(&self) -> Result<SocketAddrV4, UrlResolveError> {
        let mut addrs = (self.host(), self.port()).to_socket_addrs()?;

        addrs
            .find_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .ok_or(UrlResolveError::NoIPv4Address)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UrlResolveError {
    #[error("failed to resolve URL: {0}")]
    Lookup(#[from] std::io::Error),
    #[error("can't resolve URL to IPv4 address")]
    NoIPv4Address,
}
