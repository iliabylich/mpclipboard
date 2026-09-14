use anyhow::{Context, Result};
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt;
use std::sync::{Arc, OnceLock};

static CLIENT_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

#[expect(clippy::upper_case_acronyms)]
pub struct TLS;

impl TLS {
    pub(crate) fn init() -> Result<()> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let client_config = ClientConfig::with_platform_verifier()
            .context("failed to create SSL client with platform verifier")?;
        log::trace!("TLS has been configured");

        let _ = CLIENT_CONFIG.set(Arc::new(client_config));

        Ok(())
    }

    pub(crate) fn client_config() -> Result<Arc<ClientConfig>> {
        CLIENT_CONFIG
            .get()
            .map(Arc::clone)
            .context("TLS::init() hasn't been called")
    }
}
