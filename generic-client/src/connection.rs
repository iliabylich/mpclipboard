use crate::{
    Connectivity,
    config::Config,
    constants::{FREEZE_TIME, KEEPALIVE_COUNT, KEEPALIVE_IDLE, KEEPALIVE_INTERVAL},
    loop_state::LoopEvent,
    outbox::Outbox,
    resolve::{ResolveError, resolve},
    runtime::LoopContext,
    tls_stream::{TlsError, TlsStream, TlsWriteHalf},
    upgrade::{UpgradeError, UpgradeReader, UpgradeWriter},
};
use core::net::{Ipv4Addr, SocketAddr};
use externally_driven_async_rt::{
    Either3, MaybeTimedOut,
    io::FrameWriterError,
    net::{NetError, TcpStream},
    select3,
};
use mpclipboard_shared::{MessageReader, MessageReaderError, MessageWriter};
use rustix::io::Errno;
use rustls::ClientConfig;
use std::sync::Arc;

pub(crate) struct Connection {
    ctx: LoopContext<'static>,
    config: &'static Config,
    tls_config: &'static Arc<ClientConfig>,
    outbox: &'static Outbox,
}

impl Connection {
    pub(crate) const fn new(
        ctx: LoopContext<'static>,
        config: &'static Config,
        tls_config: &'static Arc<ClientConfig>,
        outbox: &'static Outbox,
    ) -> Self {
        Self {
            ctx,
            config,
            tls_config,
            outbox,
        }
    }

    pub(crate) async fn run(&self) -> Result<!, ConnectionError> {
        let tls = self.establish().await?;
        log::info!("connected");
        if let Err(event) = self
            .ctx
            .dispatch(LoopEvent::Connectivity(Connectivity::Connected))
        {
            log::warn!("events queue is full, dropping {event:?}");
        }
        self.serve(tls).await
    }

    async fn establish(&self) -> Result<TlsStream, ConnectionError> {
        let ip = self.resolve_ip_address().await?;
        let tcp = self.connect(ip).await?;
        let tls = self.tls_handshake(tcp).await?;
        self.upgrade(tls).await
    }

    async fn resolve_ip_address(&self) -> Result<Ipv4Addr, ConnectionError> {
        let host = self.config.url.host();
        if let Ok(ip) = host.parse::<Ipv4Addr>() {
            return Ok(ip);
        }

        let result = self
            .ctx
            .timeout(FREEZE_TIME, resolve(&self.ctx, host))
            .await
            .unwrap_or_else(|err| unreachable!("too many timers: {err:?}"));

        match result {
            MaybeTimedOut::Timeout => Err(ConnectionError::DnsStuck),
            MaybeTimedOut::Completed(ip) => Ok(ip?),
        }
    }

    async fn connect(&self, ip: Ipv4Addr) -> Result<TcpStream, ConnectionError> {
        let addr = SocketAddr::from((ip, self.config.url.port()));
        log::info!("connecting to {addr}");

        let result = self
            .ctx
            .timeout(FREEZE_TIME, TcpStream::connect(&self.ctx, addr))
            .await
            .unwrap_or_else(|err| unreachable!("too many timers: {err:?}"));

        let tcp = match result {
            MaybeTimedOut::Timeout => return Err(ConnectionError::TcpStuck),
            MaybeTimedOut::Completed(tcp) => tcp.map_err(ConnectionError::Connect)?,
        };

        tcp.set_keepalive(KEEPALIVE_IDLE, KEEPALIVE_INTERVAL, KEEPALIVE_COUNT)
            .map_err(ConnectionError::Keepalive)?;

        Ok(tcp)
    }

    async fn tls_handshake(&self, tcp: TcpStream) -> Result<TlsStream, ConnectionError> {
        let host = self.config.url.host();
        let mut tls = TlsStream::new(tcp, Arc::clone(self.tls_config), host)?;

        let result = self
            .ctx
            .timeout(FREEZE_TIME, tls.handshake(&self.ctx))
            .await
            .unwrap_or_else(|err| unreachable!("too many timers: {err:?}"));

        match result {
            MaybeTimedOut::Timeout => return Err(ConnectionError::TlsStuck),
            MaybeTimedOut::Completed(handshake) => handshake?,
        }

        Ok(tls)
    }

    async fn upgrade(&self, mut tls: TlsStream) -> Result<TlsStream, ConnectionError> {
        let (rx, tx) = tls.split();
        let upgrade = async move {
            UpgradeWriter::new(tx, self.config).write(&self.ctx).await?;
            UpgradeReader::new(rx).read(&self.ctx).await
        };

        let result = self
            .ctx
            .timeout(FREEZE_TIME, upgrade)
            .await
            .unwrap_or_else(|err| unreachable!("too many timers: {err:?}"));

        match result {
            MaybeTimedOut::Timeout => return Err(ConnectionError::UpgradeStuck),
            MaybeTimedOut::Completed(upgrade) => upgrade?,
        }

        Ok(tls)
    }

    async fn serve(&self, mut tls: TlsStream) -> Result<!, ConnectionError> {
        let ctx = &self.ctx;
        let (rx, tx) = tls.split();
        let mut reader = MessageReader::new(rx);
        let mut writer = MessageWriter::new(tx);

        self.move_message_from_outbox_to(&mut writer);

        loop {
            match select3(reader.read(ctx), ctx.notified(), writer.run(ctx)).await {
                Either3::First(message) => {
                    let Some(message) = message? else {
                        return Err(TlsError::Closed.into());
                    };
                    log::info!("received {message:?}");
                    if let Err(event) = ctx.dispatch(LoopEvent::Received(message)) {
                        log::warn!("events queue is full, dropping {event:?}");
                    }
                }
                Either3::Second(()) => self.move_message_from_outbox_to(&mut writer),
                Either3::Third(written) => {
                    let Err(err) = written;
                    return Err(err.into());
                }
            }
        }
    }

    fn move_message_from_outbox_to(&self, writer: &mut MessageWriter<TlsWriteHalf<'_>>) {
        if let Some(message) = self.outbox.take() {
            log::info!("sending {message:?}");
            writer.push(message);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionError {
    #[error("failed to resolve host: {0}")]
    Resolve(#[from] ResolveError),
    #[error("failed to connect: {0}")]
    Connect(NetError),
    #[error("failed to enable TCP keepalive: {0}")]
    Keepalive(Errno),
    #[error(transparent)]
    Tls(#[from] TlsError),
    #[error("upgrade failed: {0}")]
    Upgrade(#[from] UpgradeError),
    #[error("failed to read a message: {0}")]
    Read(#[from] MessageReaderError<TlsError>),
    #[error("failed to write a message: {0}")]
    Write(#[from] FrameWriterError<TlsError>),
    #[error("stuck in DNS resolution for {}s", FREEZE_TIME.as_secs())]
    DnsStuck,
    #[error("stuck in TCP connect for {}s", FREEZE_TIME.as_secs())]
    TcpStuck,
    #[error("stuck in TLS handshake for {}s", FREEZE_TIME.as_secs())]
    TlsStuck,
    #[error("stuck in HTTP upgrade for {}s", FREEZE_TIME.as_secs())]
    UpgradeStuck,
}
