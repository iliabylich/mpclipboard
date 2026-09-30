use crate::config::Config;
use anyhow::anyhow;
use core::convert::Infallible;
use mpclipboard_shared::{UpgradeRequestWriter, prelude::*};
use std::os::fd::AsFd;

pub fn finish_connecting(
    fd: impl AsFd,
    config: &Config,
) -> Completion<UpgradeRequestWriter, anyhow::Error, Infallible> {
    match rustix::net::sockopt::socket_error(fd) {
        Ok(Ok(())) => {}
        Ok(Err(err)) | Err(err) => {
            return Failed(anyhow!("socket_error() returned error: {err:?}"));
        }
    }

    Done(UpgradeRequestWriter::new(config.update_request()))
}
