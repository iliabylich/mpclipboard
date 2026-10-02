use alloc::sync::Arc;
use rustls::ClientConfig;
use std::sync::OnceLock;

static CLIENT_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

#[expect(clippy::upper_case_acronyms)]
pub struct TLS;

impl TLS {
    pub(crate) fn init() {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        if CLIENT_CONFIG.set(Arc::new(client_config)).is_err() {
            unreachable!("TLS::init() is only called once from MPClipboard::init_once()");
        }
        log::trace!("TLS has been configured");
    }

    pub(crate) fn client_config() -> Arc<ClientConfig> {
        let client_config = CLIENT_CONFIG
            .get()
            .unwrap_or_else(|| unreachable!("TLS::init() is always called before connecting"));
        Arc::clone(client_config)
    }
}
