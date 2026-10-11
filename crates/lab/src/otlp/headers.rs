//! Keeps environment-named headers off labd's OTLP requests (TG-PIPE-17).
//!
//! opentelemetry-otlp 0.33 reads `OTEL_EXPORTER_OTLP_LOGS_HEADERS`, else
//! `OTEL_EXPORTER_OTLP_HEADERS`, at build time and merges the result into
//! whatever the builder was given; there is no switch to turn that off. Those
//! variables are often set user-wide for another tool's collector (an API
//! key, a tenant id), and labd must not send them to the colo. So each
//! transport strips them after the merge:
//!
//! - gRPC: the SDK applies its merged metadata, then calls the configured
//!   interceptor on the same request ([`clear_metadata`] empties it).
//! - HTTP: the SDK puts its merged headers on the request, then hands it to
//!   the configured client ([`NoEnvHeaders`] drops all but the body's type
//!   and encoding).

use std::sync::Arc;
use std::time::Duration;

use opentelemetry_http::{Bytes, HttpClient, HttpError, Request, Response};

/// The only headers an OTLP/HTTP request keeps: what the body is.
const KEPT_HEADERS: [&str; 2] = ["content-type", "content-encoding"];

/// The gRPC interceptor: drop every metadata entry the SDK merged in.
#[allow(clippy::result_large_err)] // tonic's Interceptor signature
pub(crate) fn clear_metadata(
    mut req: tonic::Request<()>,
) -> Result<tonic::Request<()>, tonic::Status> {
    *req.metadata_mut() = tonic::metadata::MetadataMap::new();
    Ok(req)
}

/// Drop every header but [`KEPT_HEADERS`].
pub(crate) fn strip_headers(req: &mut Request<Bytes>) {
    let dropped: Vec<_> = req
        .headers()
        .keys()
        .filter(|k| !KEPT_HEADERS.contains(&k.as_str()))
        .cloned()
        .collect();
    for k in dropped {
        req.headers_mut().remove(k);
    }
}

/// An [`HttpClient`] that sends through `inner` with [`strip_headers`]
/// applied.
#[derive(Debug)]
pub(crate) struct NoEnvHeaders {
    inner: Arc<dyn HttpClient>,
}

impl NoEnvHeaders {
    pub(crate) fn new(inner: Arc<dyn HttpClient>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl HttpClient for NoEnvHeaders {
    async fn send_bytes(&self, mut request: Request<Bytes>) -> Result<Response<Bytes>, HttpError> {
        strip_headers(&mut request);
        self.inner.send_bytes(request).await
    }
}

/// The blocking reqwest client the SDK would have built by itself. Built on
/// a plain thread, as the SDK does: reqwest's blocking client cannot be
/// created inside the tokio runtime `init` runs in.
pub(crate) fn blocking_client(timeout: Duration) -> Result<Arc<dyn HttpClient>, String> {
    let client = std::thread::Builder::new()
        .name("labd-otlp-client".into())
        .spawn(move || {
            reqwest::blocking::Client::builder()
                .timeout(timeout)
                .build()
        })
        .map_err(|e| format!("cannot spawn the HTTP client thread: {e}"))?
        .join()
        .map_err(|_| "building the HTTP client panicked".to_string())?
        .map_err(|e| format!("cannot build the HTTP client: {e}"))?;
    Ok(Arc::new(client))
}
