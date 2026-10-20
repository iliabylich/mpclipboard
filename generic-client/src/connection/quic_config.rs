use crate::connection::state::FREEZE_TIME_IN_SECS;
use alloc::sync::Arc;
use core::time::Duration;
use quinn_proto::{
    ClientConfig, EndpointConfig, IdleTimeout, TransportConfig, VarIntBoundsExceeded,
    crypto::rustls::{NoInitialCipherSuite, QuicClientConfig},
};
use rustls::RootCertStore;

const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(1);

pub(crate) struct QuicConfig {
    pub(crate) endpoint: Arc<EndpointConfig>,
    pub(crate) client: ClientConfig,
}

impl QuicConfig {
    pub(crate) fn new() -> Result<Self, QuicConfigError> {
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        #[cfg(debug_assertions)]
        let roots = with_dev_root_cert(roots);

        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let mut tls = rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_root_certificates(roots)
            .with_no_client_auth();
        tls.alpn_protocols = vec![b"h3".to_vec()];

        let crypto = QuicClientConfig::try_from(tls)?;

        let mut transport = TransportConfig::default();
        transport.max_idle_timeout(Some(IdleTimeout::try_from(Duration::from_secs(
            FREEZE_TIME_IN_SECS,
        ))?));
        transport.keep_alive_interval(Some(KEEP_ALIVE_INTERVAL));

        let mut client = ClientConfig::new(Arc::new(crypto));
        client.transport_config(Arc::new(transport));

        Ok(Self {
            endpoint: Arc::new(EndpointConfig::default()),
            client,
        })
    }
}

#[cfg(debug_assertions)]
fn with_dev_root_cert(mut roots: RootCertStore) -> RootCertStore {
    use rustls::pki_types::{CertificateDer, pem::PemObject};

    const PEM: &[u8] = include_bytes!("../../../dev-certs/cert.pem");

    let cert = CertificateDer::from_pem_slice(PEM)
        .unwrap_or_else(|err| unreachable!("dev certificate is malformed: {err}"));
    roots
        .add(cert)
        .unwrap_or_else(|err| unreachable!("dev certificate is rejected: {err}"));
    roots
}

#[derive(Debug, thiserror::Error)]
pub enum QuicConfigError {
    #[error("TLS error: {0}")]
    Rustls(#[from] rustls::Error),
    #[error("TLS config has no initial cipher suite: {0}")]
    NoInitialCipherSuite(#[from] NoInitialCipherSuite),
    #[error("idle timeout is out of range: {0}")]
    IdleTimeout(#[from] VarIntBoundsExceeded),
}
