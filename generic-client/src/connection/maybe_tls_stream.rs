use crate::{connection::std_read_write_fd::StdReadWriteFd, tls::TLS};
use core::num::NonZeroUsize;
use mpclipboard_shared::{
    Buffer, Url, Wants,
    io::{ReadError, WriteError},
    prelude::*,
};
use rustix::fd::AsFd;
use rustls::{
    ClientConnection,
    pki_types::{InvalidDnsNameError, ServerName},
};
use std::io::{ErrorKind, Read, Write};

#[derive(Debug)]
pub enum MaybeTlsStream {
    Plain,
    Tls(Box<ClientConnection>),
}

impl MaybeTlsStream {
    pub(crate) fn new(url: &Url) -> Result<Self, MaybeTlsStreamError> {
        if url.is_tls() {
            let server_name = ServerName::try_from(url.host().to_owned())
                .map_err(MaybeTlsStreamError::ServerName)?;
            let conn = ClientConnection::new(TLS::client_config(), server_name)
                .map_err(MaybeTlsStreamError::TlsConnection)?;

            Ok(Self::Tls(Box::new(conn)))
        } else {
            Ok(Self::Plain)
        }
    }

    pub(crate) const fn is_tls(&self) -> bool {
        matches!(self, Self::Tls(_))
    }

    pub(crate) fn finish_tls_handshake(
        &mut self,
        fd: &impl AsFd,
    ) -> Completion<(), MaybeTlsStreamError, ()> {
        let conn = match self {
            Self::Tls(conn) => conn,
            Self::Plain => return Done(()),
        };

        match conn.complete_io(&mut StdReadWriteFd(fd)) {
            Ok(_) => {
                if conn.is_handshaking() {
                    Pending(())
                } else {
                    Done(())
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => Pending(()),
            Err(err) => Failed(MaybeTlsStreamError::TlsHandshake(err)),
        }
    }

    pub(crate) fn flush(&mut self, fd: &impl AsFd) -> Result<(), MaybeTlsStreamError> {
        match self {
            Self::Plain => Ok(()),
            Self::Tls(conn) => tls_flush(conn, fd),
        }
    }

    pub(crate) fn tls_wants_read(&self) -> Option<Wants> {
        match self {
            Self::Tls(conn) if conn.wants_read() => Some(Wants::Read),
            Self::Tls(_) | Self::Plain => None,
        }
    }

    pub(crate) fn tls_wants_write(&self) -> Option<Wants> {
        match self {
            Self::Tls(conn) if conn.wants_write() => Some(Wants::Write),
            Self::Tls(_) | Self::Plain => None,
        }
    }

    pub(crate) fn read_bytes<const N: usize>(
        &mut self,
        fd: &impl AsFd,
    ) -> Completion<Buffer<N>, MaybeTlsStreamError, ()> {
        match self {
            Self::Plain => mpclipboard_shared::io::read(fd).map_err(MaybeTlsStreamError::PlainRead),
            Self::Tls(conn) => tls_read(conn, fd),
        }
    }

    pub(crate) fn write_bytes(
        &mut self,
        fd: &impl AsFd,
        buf: &[u8],
    ) -> Completion<NonZeroUsize, MaybeTlsStreamError, ()> {
        assert!(!buf.is_empty(), "can't write an empty buffer");

        match self {
            Self::Plain => {
                mpclipboard_shared::io::write(fd, buf).map_err(MaybeTlsStreamError::PlainWrite)
            }
            Self::Tls(conn) => tls_write(conn, fd, buf),
        }
    }
}

fn complete_io(conn: &mut ClientConnection, fd: &impl AsFd) -> Result<(), MaybeTlsStreamError> {
    match conn.complete_io(&mut StdReadWriteFd(fd)) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(()),
        Err(err) => Err(MaybeTlsStreamError::TlsIo(err)),
    }
}

fn tls_flush(conn: &mut ClientConnection, fd: &impl AsFd) -> Result<(), MaybeTlsStreamError> {
    while conn.wants_write() {
        match conn.write_tls(&mut StdReadWriteFd(fd)) {
            Ok(_) => {}
            Err(err) if err.kind() == ErrorKind::WouldBlock => break,
            Err(err) => return Err(MaybeTlsStreamError::TlsFlush(err)),
        }
    }
    Ok(())
}

fn tls_read<const N: usize>(
    conn: &mut ClientConnection,
    fd: &impl AsFd,
) -> Completion<Buffer<N>, MaybeTlsStreamError, ()> {
    if let Err(err) = complete_io(conn, fd) {
        return Failed(err);
    }

    let mut buf = [0; N];
    match conn.reader().read(&mut buf).map(NonZeroUsize::new) {
        Ok(Some(len)) => {
            let buf = buf
                .get(..len.get())
                .and_then(Buffer::from_slice)
                .unwrap_or_else(|| unreachable!("read() can't return more than N bytes"));
            Done(buf)
        }
        Ok(None) => Failed(MaybeTlsStreamError::TlsReadEof),
        Err(err) if err.kind() == ErrorKind::WouldBlock => Pending(()),
        Err(err) => Failed(MaybeTlsStreamError::TlsRead(err)),
    }
}

fn tls_write(
    conn: &mut ClientConnection,
    fd: &impl AsFd,
    buf: &[u8],
) -> Completion<NonZeroUsize, MaybeTlsStreamError, ()> {
    let len = match conn.writer().write(buf).map(NonZeroUsize::new) {
        Ok(len) => len,
        Err(err) if err.kind() == ErrorKind::WouldBlock => return Pending(()),
        Err(err) => return Failed(MaybeTlsStreamError::TlsWrite(err)),
    };

    if let Err(err) = tls_flush(conn, fd) {
        return Failed(err);
    }

    match len {
        Some(len) => Done(len),
        None => Pending(()),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MaybeTlsStreamError {
    #[error("failed to build TLS server name: {0}")]
    ServerName(InvalidDnsNameError),
    #[error("failed to create TLS connection: {0}")]
    TlsConnection(rustls::Error),
    #[error("TLS handshake failed: {0}")]
    TlsHandshake(std::io::Error),
    #[error("failed to complete_io() on TLS stream: {0}")]
    TlsIo(std::io::Error),
    #[error("failed to write_tls() on TLS stream: {0}")]
    TlsFlush(std::io::Error),
    #[error("failed to read_bytes() on TLS stream: {0}")]
    TlsRead(std::io::Error),
    #[error("failed to read_bytes() on TLS stream: EOF")]
    TlsReadEof,
    #[error("failed to write_bytes() on TLS stream: {0}")]
    TlsWrite(std::io::Error),
    #[error("failed to read_bytes() on plain stream: {0}")]
    PlainRead(ReadError),
    #[error("failed to write_bytes() on plain stream: {0}")]
    PlainWrite(WriteError),
}
