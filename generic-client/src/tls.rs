use anyhow::{Context, Result};
use rustls::ClientConfig;
use std::sync::{Arc, OnceLock};

static CLIENT_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

#[expect(clippy::upper_case_acronyms)]
pub struct TLS;

impl TLS {
    pub(crate) fn init() -> Result<()> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
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
