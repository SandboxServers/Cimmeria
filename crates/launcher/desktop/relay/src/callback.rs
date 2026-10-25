//! One-shot loopback listener for the OAuth redirect. It takes the authorization code (or the
//! error) for the state the launcher expects, and answers the browser with a static page.

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use url::form_urlencoded;

/// Query strings above this size are refused without parsing.
const MAX_QUERY: usize = 4 * 1024;
/// Connections open at once. Further connections wait in the accept queue.
const MAX_CONNECTIONS: usize = 16;
/// A connection that is still open after this long is closed. Browsers finish a redirect
/// within this time, and an idle socket cannot pin a slot for the whole sign-in wait.
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_TAB_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Cimmeria</title>\
<p>You can close this tab and return to the Cimmeria launcher.</p>";
const CANCELLED_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Cimmeria</title>\
<p>Sign-in was cancelled.</p>";

/// The loopback listener that receives the OAuth redirect.
pub struct CallbackServer {
    listener: TcpListener,
    pub(crate) addr: SocketAddr,
}

/// How a sign-in redirect ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackResult {
    /// The redirect carried a code for the expected state.
    Code(String),
    /// The redirect carried an `error` parameter.
    Denied,
    /// The redirect carried a state other than the expected one.
    StateMismatch,
    /// No matching redirect arrived before the timeout.
    TimedOut,
}

impl CallbackServer {
    /// Binds `127.0.0.1` on an ephemeral port.
    pub async fn bind() -> std::io::Result<CallbackServer> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let addr = listener.local_addr()?;
        Ok(CallbackServer { listener, addr })
    }

    /// The redirect URI to register with the provider, `http://127.0.0.1:<port>/cb`.
    pub fn redirect_uri(&self) -> String {
        format!("http://{}/cb", self.addr)
    }

    /// Waits for the first `/cb` request that ends the sign-in, or for `timeout`.
    /// Requests to other paths get `404` and do not end the wait.
    pub async fn wait(self, expected_state: &str, timeout: Duration) -> CallbackResult {
        let (tx, mut rx) = mpsc::channel::<CallbackResult>(1);
        tokio::select! {
            _ = accept_loop(&self.listener, expected_state, &tx) => CallbackResult::TimedOut,
            result = rx.recv() => result.unwrap_or(CallbackResult::TimedOut),
            () = tokio::time::sleep(timeout) => CallbackResult::TimedOut,
        }
    }
}

/// Accepts connections forever. Each connection runs on its own task, holding one of
/// [`MAX_CONNECTIONS`] permits until it closes or [`CONNECTION_TIMEOUT`] passes.
async fn accept_loop(
    listener: &TcpListener,
    expected_state: &str,
    tx: &mpsc::Sender<CallbackResult>,
) {
    let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        let Ok(permit) = Arc::clone(&slots).acquire_owned().await else {
            return;
        };
        match listener.accept().await {
            Ok((stream, _)) => {
                tokio::spawn(serve_connection(
                    stream,
                    expected_state.to_owned(),
                    tx.clone(),
                    permit,
                ));
            }
            Err(_) => tokio::time::sleep(Duration::from_millis(10)).await,
        }
    }
}

async fn serve_connection(
    stream: TcpStream,
    expected_state: String,
    tx: mpsc::Sender<CallbackResult>,
    _permit: OwnedSemaphorePermit,
) {
    let service = service_fn(move |req: Request<Incoming>| {
        let expected_state = expected_state.clone();
        let tx = tx.clone();
        async move { Ok::<_, Infallible>(answer(&req, &expected_state, &tx)) }
    });
    let serve = http1::Builder::new().serve_connection(TokioIo::new(stream), service);
    let _ = tokio::time::timeout(CONNECTION_TIMEOUT, serve).await;
}

/// Decides what one request means for the sign-in and builds the reply.
fn answer(
    req: &Request<Incoming>,
    expected_state: &str,
    tx: &mpsc::Sender<CallbackResult>,
) -> Response<Full<Bytes>> {
    let query = req.uri().query().unwrap_or("");
    if req.method() != Method::GET || req.uri().path() != "/cb" || query.len() > MAX_QUERY {
        return page(StatusCode::NOT_FOUND, CANCELLED_PAGE);
    }
    let mut code = None;
    let mut state = None;
    let mut error = false;
    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
        match &*key {
            "code" if code.is_none() => code = Some(value.into_owned()),
            "state" if state.is_none() => state = Some(value.into_owned()),
            "error" => error = true,
            _ => {}
        }
    }
    let (result, status, body) = if error {
        (CallbackResult::Denied, StatusCode::OK, CANCELLED_PAGE)
    } else if !state.as_deref().is_some_and(|s| ct_eq(s, expected_state)) {
        (
            CallbackResult::StateMismatch,
            StatusCode::BAD_REQUEST,
            CANCELLED_PAGE,
        )
    } else {
        match code {
            Some(code) if !code.is_empty() => {
                (CallbackResult::Code(code), StatusCode::OK, CLOSE_TAB_PAGE)
            }
            _ => (CallbackResult::Denied, StatusCode::OK, CANCELLED_PAGE),
        }
    };
    let _ = tx.try_send(result);
    page(status, body)
}

fn page(status: StatusCode, body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}

/// Compares two values without an early exit on the first differing byte.
fn ct_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}
