use anyhow::anyhow;
use core::convert::Infallible;
use mpclipboard_shared::prelude::*;
use std::os::fd::AsFd;

pub fn finish_connecting(fd: impl AsFd) -> Completion<(), anyhow::Error, Infallible> {
    match rustix::net::sockopt::socket_error(fd) {
        Ok(Ok(())) => Done(()),
        Ok(Err(err)) | Err(err) => Failed(anyhow!("socket_error() returned error: {err:?}")),
    }
}
