use core::time::Duration;
use rustix::{
    fd::AsFd,
    io::Errno,
    net::sockopt::{set_socket_keepalive, set_tcp_keepcnt, set_tcp_keepidle, set_tcp_keepintvl},
};

pub fn enable_tcp_keep_alive(fd: &impl AsFd) -> Result<(), TcpKeepAliveError> {
    set_socket_keepalive(fd, true).map_err(TcpKeepAliveError::SetSocketKeepalive)?;

    // start probing after one second of idle
    set_tcp_keepidle(fd, Duration::from_secs(1)).map_err(TcpKeepAliveError::SetTcpKeepidle)?;

    // retry once a second
    set_tcp_keepintvl(fd, Duration::from_secs(1)).map_err(TcpKeepAliveError::SetTcpKeepintvl)?;

    // die after 3 failed probes (i.e. after 3s of inactivity)
    set_tcp_keepcnt(fd, 3).map_err(TcpKeepAliveError::SetTcpKeepcnt)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpKeepAliveError {
    SetSocketKeepalive(Errno),
    SetTcpKeepidle(Errno),
    SetTcpKeepintvl(Errno),
    SetTcpKeepcnt(Errno),
}

impl core::fmt::Display for TcpKeepAliveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SetSocketKeepalive(errno) => {
                write!(f, "failed to set_socket_keepalive(): {errno:?}")
            }
            Self::SetTcpKeepidle(errno) => write!(f, "failed to set_tcp_keepidle(): {errno:?}"),
            Self::SetTcpKeepintvl(errno) => write!(f, "failed to set_tcp_keepintvl(): {errno:?}"),
            Self::SetTcpKeepcnt(errno) => write!(f, "failed to set_tcp_keepcnt(): {errno:?}"),
        }
    }
}

impl core::error::Error for TcpKeepAliveError {}
