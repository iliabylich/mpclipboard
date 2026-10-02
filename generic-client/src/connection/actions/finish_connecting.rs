use rustix::{fd::AsFd, io::Errno};

pub fn finish_connecting(fd: impl AsFd) -> Result<(), Errno> {
    rustix::net::sockopt::socket_error(fd)?
}
