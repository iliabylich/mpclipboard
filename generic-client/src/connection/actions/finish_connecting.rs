use core::convert::Infallible;
use mpclipboard_shared::{error, prelude::*};
use std::os::fd::AsFd;

pub fn finish_connecting(fd: impl AsFd) -> Completion<(), Infallible> {
    match rustix::net::sockopt::socket_error(fd) {
        Ok(Ok(())) => Done(()),
        Ok(Err(err)) | Err(err) => {
            error!("socket_error returned error: {err:?}");
            Failed
        }
    }
}
