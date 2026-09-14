use crate::tls::TLS;
use anyhow::{Context, Result};
use mpclipboard_shared::{Url, Wants, error, prelude::*};
use rustls::{ClientConnection, pki_types::ServerName};
use std::{
    io::{ErrorKind, Read, Write},
    num::NonZeroUsize,
    os::fd::AsFd,
};

#[derive(Debug)]
pub enum MaybeTlsStream {
    Plain,
    Tls(Box<ClientConnection>),
}

#[derive(Debug)]
pub enum TlsHandshakeResult {
    Done,
    Pending,
    Died,
}

impl MaybeTlsStream {
    pub(crate) fn new(url: &Url) -> Result<Self> {
        if url.is_tls() {
            let server_name = ServerName::try_from(url.host().to_owned())
                .context("failed to build TLS server name")?;
            let conn = ClientConnection::new(TLS::client_config()?, server_name)
                .context("failed to create TLS connection")?;

            Ok(Self::Tls(Box::new(conn)))
        } else {
            Ok(Self::Plain)
        }
    }

    pub(crate) const fn is_tls(&self) -> bool {
        matches!(self, Self::Tls(_))
    }

    pub(crate) fn finish_tls_handshake(&mut self, fd: &impl AsFd) -> TlsHandshakeResult {
        let conn = match self {
            Self::Tls(conn) => conn,
            Self::Plain => return TlsHandshakeResult::Done,
        };

        match conn.complete_io(&mut StdReadWriteFd(fd)) {
            Ok(_) => {
                if conn.is_handshaking() {
                    TlsHandshakeResult::Pending
                } else {
                    TlsHandshakeResult::Done
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => TlsHandshakeResult::Pending,
            Err(err) => {
                error!("TLS handshake failed: {err:?}");
                TlsHandshakeResult::Died
            }
        }
    }

    pub(crate) fn flush(&mut self, fd: &impl AsFd) -> Result<()> {
        let conn = match self {
            Self::Tls(conn) => conn,
            Self::Plain => return Ok(()),
        };

        match conn.complete_io(&mut StdReadWriteFd(fd)) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub(crate) fn tls_wants(&self) -> Option<Wants> {
        match self {
            Self::Plain => None,
            Self::Tls(conn) => match (conn.wants_read(), conn.wants_write()) {
                (true, true) => Some(Wants::ReadWrite),
                (true, false) => Some(Wants::Read),
                (false, true | false) => Some(Wants::Write),
            },
        }
    }

    pub(crate) fn read_bytes(
        &mut self,
        fd: &impl AsFd,
        buf: &mut [u8],
    ) -> Completion<NonZeroUsize, ()> {
        match self {
            Self::Plain => mpclipboard_shared::io::read(fd, buf)
                .map_err(|| error!("failed to read_bytes() on plain stream")),
            Self::Tls(conn) => {
                match conn.complete_io(&mut StdReadWriteFd(fd)) {
                    Ok(_) => {}
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {}
                    Err(err) => {
                        error!("failed to complete_io() on TLS stream: {err:?}");
                        return Failed;
                    }
                }

                match conn.reader().read(buf).map(NonZeroUsize::new) {
                    Ok(Some(len)) => Done(len),
                    Ok(None) => {
                        error!("failed to read_bytes() on TLS stream: EOF");
                        Failed
                    }
                    Err(err) if err.kind() == ErrorKind::WouldBlock => Pending(()),
                    Err(err) => {
                        error!("failed to read_bytes() on TLS stream: {err:?}");
                        Failed
                    }
                }
            }
        }
    }

    pub(crate) fn write_bytes(
        &mut self,
        fd: &impl AsFd,
        buf: &[u8],
    ) -> Completion<NonZeroUsize, ()> {
        match self {
            Self::Plain => mpclipboard_shared::io::write(fd, buf)
                .map_err(|| error!("failed to write_bytes() on plain stream")),
            Self::Tls(conn) => {
                let len = match conn.writer().write(buf).map(NonZeroUsize::new) {
                    Ok(Some(len)) => len,
                    Ok(None) => {
                        error!("failed to write_bytes() on TLS stream: EOF");
                        return Failed;
                    }
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {
                        return Pending(());
                    }
                    Err(err) => {
                        error!("failed to write_bytes() on TLS stream: {err:?}");
                        return Failed;
                    }
                };

                match conn.complete_io(&mut StdReadWriteFd(fd)) {
                    Ok(_) => Done(len),
                    Err(err) if err.kind() == ErrorKind::WouldBlock => Done(len),
                    Err(err) => {
                        error!("failed to complete_io() on TLS stream: {err:?}");
                        Failed
                    }
                }
            }
        }
    }
}

struct StdReadWriteFd<'a, F>(&'a F);

impl<F> Read for StdReadWriteFd<'_, F>
where
    F: AsFd,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = rustix::io::read(self.0, buf)?;
        Ok(len)
    }
}

impl<F> Write for StdReadWriteFd<'_, F>
where
    F: AsFd,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len = rustix::io::write(self.0, buf)?;
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
