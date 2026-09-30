use anyhow::{Result, bail};
use std::os::fd::AsFd;

pub fn finish_connecting(fd: impl AsFd) -> Result<()> {
    match rustix::net::sockopt::socket_error(fd) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(err)) | Err(err) => {
            bail!("socket_error() returned error: {err:?}");
        }
    }
}
