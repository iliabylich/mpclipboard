use crate::{connection::std_read_write_fd::StdReadWriteFd, tls::TLS};
use mpclipboard_shared::{
    Buffer, Url, Wants,
    io::{ReadError, WriteError},
    prelude::*,
};
use rustls::{
    ClientConnection,
    pki_types::{InvalidDnsNameError, ServerName},
};
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
            Self::Tls(conn) => complete_io(conn, fd),
        }
    }

    pub(crate) fn tls_wants(&self) -> Option<Wants> {
        match self {
            Self::Plain => None,
            Self::Tls(conn) => match (conn.wants_read(), conn.wants_write()) {
                (true, true) => Some(Wants::ReadWrite),
                (true, false) => Some(Wants::Read),
                (false, true) => Some(Wants::Write),
                (false, false) => None,
            },
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

    if let Err(err) = complete_io(conn, fd) {
        return Failed(err);
    }

    match len {
        Some(len) => Done(len),
        None => Pending(()),
    }
}

#[derive(Debug)]
pub enum MaybeTlsStreamError {
    ServerName(InvalidDnsNameError),
    TlsConnection(rustls::Error),
    TlsHandshake(std::io::Error),
    TlsIo(std::io::Error),
    TlsRead(std::io::Error),
    TlsReadEof,
    TlsWrite(std::io::Error),
    PlainRead(ReadError),
    PlainWrite(WriteError),
}

impl core::fmt::Display for MaybeTlsStreamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ServerName(err) => write!(f, "failed to build TLS server name: {err}"),
            Self::TlsConnection(err) => write!(f, "failed to create TLS connection: {err}"),
            Self::TlsHandshake(err) => write!(f, "TLS handshake failed: {err}"),
            Self::TlsIo(err) => write!(f, "failed to complete_io() on TLS stream: {err}"),
            Self::TlsRead(err) => write!(f, "failed to read_bytes() on TLS stream: {err}"),
            Self::TlsReadEof => write!(f, "failed to read_bytes() on TLS stream: EOF"),
            Self::TlsWrite(err) => write!(f, "failed to write_bytes() on TLS stream: {err}"),
            Self::PlainRead(err) => write!(f, "failed to read_bytes() on plain stream: {err}"),
            Self::PlainWrite(err) => write!(f, "failed to write_bytes() on plain stream: {err}"),
        }
    }
}

impl core::error::Error for MaybeTlsStreamError {}
