use crate::{connection::std_read_write_fd::StdReadWriteFd, tls::TLS};
use core::num::NonZeroUsize;
use generic_array::{ArrayLength, GenericArray, typenum::NonZero};
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
            let server_name = ServerName::try_from(url.host().to_owned())?;
            let conn = ClientConnection::new(TLS::client_config(), server_name)?;

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
    ) -> Result<Completion<(), ()>, MaybeTlsStreamError> {
        let conn = match self {
            Self::Tls(conn) => conn,
            Self::Plain => return Ok(Done(())),
        };

        match conn.complete_io(&mut StdReadWriteFd(fd)) {
            Ok(_) => {
                if conn.is_handshaking() {
                    Ok(Pending(()))
                } else {
                    Ok(Done(()))
                }
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(Pending(())),
            Err(err) => Err(MaybeTlsStreamError::TlsHandshake(err)),
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

    pub(crate) fn read_bytes<N: ArrayLength + NonZero>(
        &mut self,
        fd: &impl AsFd,
    ) -> Result<Completion<Buffer<N>, ()>, MaybeTlsStreamError> {
        match self {
            Self::Plain => {
                let read = mpclipboard_shared::io::read(fd)?;
                Ok(read)
            }
            Self::Tls(conn) => tls_read(conn, fd),
        }
    }

    pub(crate) fn write_bytes(
        &mut self,
        fd: &impl AsFd,
        buf: &[u8],
    ) -> Result<Completion<NonZeroUsize, ()>, MaybeTlsStreamError> {
        assert!(!buf.is_empty(), "can't write an empty buffer");

        match self {
            Self::Plain => {
                let written = mpclipboard_shared::io::write(fd, buf)?;
                Ok(written)
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

fn tls_read<N: ArrayLength + NonZero>(
    conn: &mut ClientConnection,
    fd: &impl AsFd,
) -> Result<Completion<Buffer<N>, ()>, MaybeTlsStreamError> {
    complete_io(conn, fd)?;

    let mut buf = GenericArray::<u8, N>::default();
    match conn
        .reader()
        .read(buf.as_mut_slice())
        .map(NonZeroUsize::new)
    {
        Ok(Some(len)) => {
            let buf = buf
                .get(..len.get())
                .and_then(Buffer::from_slice)
                .unwrap_or_else(|| unreachable!("read() can't return more than N bytes"));
            Ok(Done(buf))
        }
        Ok(None) => Err(MaybeTlsStreamError::TlsReadEof),
        Err(err) if err.kind() == ErrorKind::WouldBlock => Ok(Pending(())),
        Err(err) => Err(MaybeTlsStreamError::TlsRead(err)),
    }
}

fn tls_write(
    conn: &mut ClientConnection,
    fd: &impl AsFd,
    buf: &[u8],
) -> Result<Completion<NonZeroUsize, ()>, MaybeTlsStreamError> {
    let len = match conn.writer().write(buf).map(NonZeroUsize::new) {
        Ok(len) => len,
        Err(err) if err.kind() == ErrorKind::WouldBlock => return Ok(Pending(())),
        Err(err) => return Err(MaybeTlsStreamError::TlsWrite(err)),
    };

    tls_flush(conn, fd)?;

    match len {
        Some(len) => Ok(Done(len)),
        None => Ok(Pending(())),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MaybeTlsStreamError {
    #[error("failed to build TLS server name: {0}")]
    ServerName(#[from] InvalidDnsNameError),
    #[error("failed to create TLS connection: {0}")]
    TlsConnection(#[from] rustls::Error),
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
    PlainRead(#[from] ReadError),
    #[error("failed to write_bytes() on plain stream: {0}")]
    PlainWrite(#[from] WriteError),
}
