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

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TcpKeepAliveError {
    #[error("failed to set_socket_keepalive(): {0:?}")]
    SetSocketKeepalive(Errno),
    #[error("failed to set_tcp_keepidle(): {0:?}")]
    SetTcpKeepidle(Errno),
    #[error("failed to set_tcp_keepintvl(): {0:?}")]
    SetTcpKeepintvl(Errno),
    #[error("failed to set_tcp_keepcnt(): {0:?}")]
    SetTcpKeepcnt(Errno),
}
