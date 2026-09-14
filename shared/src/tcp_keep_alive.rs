use anyhow::{Context, Result};
use core::time::Duration;
use rustix::net::sockopt::{
    set_socket_keepalive, set_tcp_keepcnt, set_tcp_keepidle, set_tcp_keepintvl,
};
use std::os::fd::AsFd;

pub fn enable_tcp_keep_alive(fd: &impl AsFd) -> Result<()> {
    set_socket_keepalive(fd, true).context("failed to set_socket_keepalive()")?;

    // start probing after one second of idle
    set_tcp_keepidle(fd, Duration::from_secs(1)).context("failed to set_tcp_keepidle()")?;

    // retry once a second
    set_tcp_keepintvl(fd, Duration::from_secs(1)).context("failed to set_tcp_keepintvl()")?;

    // die after 3 failed probes (i.e. after 3s of inactivity)
    set_tcp_keepcnt(fd, 3).context("failed to set_tcp_keepcnt()")?;
    Ok(())
}
