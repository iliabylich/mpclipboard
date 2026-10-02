use core::net::{SocketAddr, SocketAddrV4};
use mpclipboard_shared::Url;
use std::net::ToSocketAddrs;

pub trait UrlExt {
    fn resolve(&self) -> Result<SocketAddrV4, UrlResolveError>;
}

impl UrlExt for Url {
    fn resolve(&self) -> Result<SocketAddrV4, UrlResolveError> {
        let mut addrs = (self.host(), self.port())
            .to_socket_addrs()
            .map_err(UrlResolveError::Lookup)?;

        addrs
            .find_map(|addr| match addr {
                SocketAddr::V4(v4) => Some(v4),
                SocketAddr::V6(_) => None,
            })
            .ok_or(UrlResolveError::NoIPv4Address)
    }
}

#[derive(Debug)]
pub enum UrlResolveError {
    Lookup(std::io::Error),
    NoIPv4Address,
}

impl core::fmt::Display for UrlResolveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Lookup(err) => write!(f, "failed to resolve URL: {err}"),
            Self::NoIPv4Address => write!(f, "can't resolve URL to IPv4 address"),
        }
    }
}

impl core::error::Error for UrlResolveError {}
