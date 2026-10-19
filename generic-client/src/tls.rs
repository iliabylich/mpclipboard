use alloc::sync::Arc;
use rustls::{ClientConfig, RootCertStore};
use std::sync::OnceLock;

static CLIENT_CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();

#[expect(clippy::upper_case_acronyms)]
pub(crate) struct TLS;

impl TLS {
    pub(crate) fn init() {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let root_store = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        #[cfg(debug_assertions)]
        let root_store = Self::with_dev_root_cert(root_store);

        let client_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        if CLIENT_CONFIG.set(Arc::new(client_config)).is_err() {
            unreachable!("TLS::init() is only called once from MPClipboard::init_once()");
        }
        log::trace!("TLS has been configured");
    }

    #[cfg(debug_assertions)]
    fn with_dev_root_cert(mut root_store: RootCertStore) -> RootCertStore {
        use rustls::pki_types::{CertificateDer, pem::PemObject};

        const PEM: &[u8] = include_bytes!("../../dev-certs/cert.pem");

        let cert = CertificateDer::from_pem_slice(PEM)
            .unwrap_or_else(|err| unreachable!("dev certificate is malformed: {err}"));
        root_store
            .add(cert)
            .unwrap_or_else(|err| unreachable!("dev certificate is rejected: {err}"));
        root_store
    }

    pub(crate) fn client_config() -> Arc<ClientConfig> {
        let client_config = CLIENT_CONFIG
            .get()
            .unwrap_or_else(|| unreachable!("TLS::init() is always called before connecting"));
        Arc::clone(client_config)
    }
}
