//! The HTTP client every provider call shares (contract C3): a 10 s timeout,
//! no redirects, rustls, and response bodies read in capped chunks.

use std::time::Duration;

use crate::provider::ProviderError;

/// The timeout on every provider call.
pub const PROVIDER_TIMEOUT: Duration = Duration::from_secs(10);

/// The largest provider response body the server reads, in bytes.
pub const PROVIDER_BODY_CAP: usize = 64 * 1024;

/// Builds the one client the providers share: 10 s timeout, no redirects.
pub fn provider_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(PROVIDER_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("the provider client builds from static settings")
}

/// Reads a provider response body, failing with `BadResponse("too_large")`
/// once it passes `cap` bytes. The body is read chunk by chunk, so an
/// oversized answer is refused without being held whole in memory.
pub async fn read_capped(
    mut resp: reqwest::Response,
    cap: usize,
) -> Result<Vec<u8>, ProviderError> {
    if resp.content_length().is_some_and(|len| len > cap as u64) {
        return Err(ProviderError::BadResponse("too_large"));
    }
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(transport_error)? {
        if body.len() + chunk.len() > cap {
            return Err(ProviderError::BadResponse("too_large"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Maps a transport failure to the provider error the spans report.
pub(crate) fn transport_error(err: reqwest::Error) -> ProviderError {
    if err.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::BadResponse("transport")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Serves one HTTP answer with `body` on a local port and returns its URL.
    async fn serve_once(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind a local port");
        let addr = listener.local_addr().expect("local address");
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.expect("accept");
            let mut request = [0u8; 1024];
            let _ = sock.read(&mut request).await;
            // No Content-Length: the body ends when the connection closes.
            let head = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n";
            let _ = sock.write_all(head).await;
            let _ = sock.write_all(&body).await;
            let _ = sock.shutdown().await;
        });
        format!("http://{addr}/")
    }

    #[tokio::test]
    async fn read_capped_refuses_an_oversized_body() {
        let url = serve_once(vec![b'x'; PROVIDER_BODY_CAP + 1]).await;
        let resp = provider_client().get(url).send().await.expect("send");
        assert_eq!(
            read_capped(resp, PROVIDER_BODY_CAP).await,
            Err(ProviderError::BadResponse("too_large"))
        );
    }

    #[tokio::test]
    async fn read_capped_returns_a_small_body() {
        let url = serve_once(b"{}".to_vec()).await;
        let resp = provider_client().get(url).send().await.expect("send");
        assert_eq!(
            read_capped(resp, PROVIDER_BODY_CAP).await,
            Ok(b"{}".to_vec())
        );
    }
}
