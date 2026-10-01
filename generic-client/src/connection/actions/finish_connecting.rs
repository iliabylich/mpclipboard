use rustix::io::Errno;
use std::os::fd::AsFd;

pub fn finish_connecting(fd: impl AsFd) -> Result<(), Errno> {
    rustix::net::sockopt::socket_error(fd)?
}
