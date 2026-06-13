//! HTTP client construction helpers.

use std::sync::Once;

static TLS_PROVIDER_INIT: Once = Once::new();

pub(crate) fn client_builder() -> reqwest::ClientBuilder {
    install_tls_provider();
    reqwest::Client::builder()
}

#[cfg(test)]
pub(crate) fn test_client() -> reqwest::Client {
    client_builder().build().expect("build test HTTP client")
}

fn install_tls_provider() {
    TLS_PROVIDER_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
