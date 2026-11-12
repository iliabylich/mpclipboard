use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

pub(crate) fn tls_config() -> Result<Arc<ClientConfig>, rustls::Error> {
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    #[cfg(debug_assertions)]
    let roots = with_dev_root_cert(roots);

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}

#[cfg(debug_assertions)]
fn with_dev_root_cert(mut roots: RootCertStore) -> RootCertStore {
    use rustls::pki_types::{CertificateDer, pem::PemObject};

    const PEM: &[u8] = include_bytes!("../../dev-certs/cert.pem");

    let cert = CertificateDer::from_pem_slice(PEM)
        .unwrap_or_else(|err| unreachable!("dev certificate is malformed: {err}"));
    roots
        .add(cert)
        .unwrap_or_else(|err| unreachable!("dev certificate is rejected: {err}"));
    roots
}
