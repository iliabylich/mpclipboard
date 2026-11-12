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
use core::{
    convert::Infallible,
    net::{Ipv4Addr, SocketAddr},
};
use externally_driven_async_rt::{Either3, io::FrameWriterError, net::TcpStream, select3};
use mpclipboard_shared2::{MessageReader, MessageReaderError, MessageWriter};
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

    pub(crate) async fn run(&self) -> Result<Infallible, ConnectionError> {
        let tls = self.establish().await?;
        log::info!("connected");
        self.ctx
            .dispatch(LoopEvent::Connectivity(Connectivity::Connected));
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

        let ip = self
            .ctx
            .timeout(FREEZE_TIME, resolve(&self.ctx, host))
            .await
            .ok_or(ConnectionError::DnsStuck)??;
        Ok(ip)
    }

    async fn connect(&self, ip: Ipv4Addr) -> Result<TcpStream, ConnectionError> {
        let addr = SocketAddr::from((ip, self.config.url.port()));
        log::info!("connecting to {addr}");

        let tcp = self
            .ctx
            .timeout(FREEZE_TIME, TcpStream::connect(&self.ctx, addr))
            .await
            .ok_or(ConnectionError::TcpStuck)?
            .map_err(ConnectionError::Connect)?;

        tcp.set_keepalive(KEEPALIVE_IDLE, KEEPALIVE_INTERVAL, KEEPALIVE_COUNT)
            .map_err(ConnectionError::Keepalive)?;

        Ok(tcp)
    }

    async fn tls_handshake(&self, tcp: TcpStream) -> Result<TlsStream, ConnectionError> {
        let host = self.config.url.host();
        let mut tls = TlsStream::new(tcp, Arc::clone(self.tls_config), host)?;

        self.ctx
            .timeout(FREEZE_TIME, tls.handshake(&self.ctx))
            .await
            .ok_or(ConnectionError::TlsStuck)??;
        Ok(tls)
    }

    async fn upgrade(&self, mut tls: TlsStream) -> Result<TlsStream, ConnectionError> {
        let (rx, tx) = tls.split();
        let upgrade = async move {
            UpgradeWriter::new(tx, self.config).write(&self.ctx).await?;
            UpgradeReader::new(rx).read(&self.ctx).await
        };

        self.ctx
            .timeout(FREEZE_TIME, upgrade)
            .await
            .ok_or(ConnectionError::UpgradeStuck)??;
        Ok(tls)
    }

    async fn serve(&self, mut tls: TlsStream) -> Result<Infallible, ConnectionError> {
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
                    ctx.dispatch(LoopEvent::Received(message));
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
    Connect(Errno),
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
