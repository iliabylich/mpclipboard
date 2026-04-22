use anyhow::{Context as _, Result, bail};
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt as _;
use std::sync::{Arc, OnceLock};

static CLIENT_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

#[expect(clippy::upper_case_acronyms)]
pub(crate) struct TLS(pub(crate) tungstenite::Connector);

impl TLS {
    pub(crate) fn init() -> Result<()> {
        rustls::crypto::ring::default_provider()
            .install_default()
            .expect("Failed to install rustls crypto provider");

        let client_config = ClientConfig::with_platform_verifier()
            .context("failed to create SSL client with platform verifier")?;
        log::trace!("TLS has been configured");

        if CLIENT_CONFIG.set(Arc::new(client_config)).is_err() {
            bail!("TLS::init() has already been called");
        }

        Ok(())
    }

    pub(crate) fn new(enable_tls: bool) -> Result<Self> {
        if enable_tls {
            Ok(Self(tungstenite::Connector::Rustls(Arc::clone(
                CLIENT_CONFIG
                    .get()
                    .context("TLS::init() hasn't been called")?,
            ))))
        } else {
            log::trace!("Using plain ws:// connection");
            Ok(Self(tungstenite::Connector::Plain))
        }
    }
}

impl Clone for TLS {
    fn clone(&self) -> Self {
        match &self.0 {
            tungstenite::Connector::Plain => Self(tungstenite::Connector::Plain),
            tungstenite::Connector::Rustls(config) => {
                Self(tungstenite::Connector::Rustls(Arc::clone(config)))
            }
            _ => unreachable!("unsupported connector variant"),
        }
    }
}
