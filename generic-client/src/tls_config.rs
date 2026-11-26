use alloc::{sync::Arc, vec};
use core::time::Duration;
use rustix::time::{ClockId, clock_gettime};
use rustls::{ClientConfig, RootCertStore, pki_types::UnixTime, time_provider::TimeProvider};

pub(crate) fn tls_config() -> Result<Arc<ClientConfig>, rustls::Error> {
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    #[cfg(debug_assertions)]
    let roots = with_dev_root_cert(roots);

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = ClientConfig::builder_with_details(provider, Arc::new(RealtimeClock))
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}

#[derive(Debug)]
struct RealtimeClock;

impl TimeProvider for RealtimeClock {
    fn current_time(&self) -> Option<UnixTime> {
        let now = clock_gettime(ClockId::Realtime);
        let secs = u64::try_from(now.tv_sec).ok()?;
        let nanos = u32::try_from(now.tv_nsec).ok()?;
        Some(UnixTime::since_unix_epoch(Duration::new(secs, nanos)))
    }
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
