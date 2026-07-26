//! The readiness contract: how the harness knows the server is up.
//!
//! # Signal
//!
//! `GET /api/config/status` on the admin port, returning HTTP 200 with
//! `services.auth`, `services.base` and `services.cell` all `true`.
//!
//! # Why this signal
//!
//! It is the only observable that is *causally downstream of every bind*:
//!
//! - `Orchestrator::start_all` starts auth, base and cell strictly
//!   sequentially, `await`ing each and propagating errors. Each service sets
//!   `is_running = true` only after its own listener bind has returned `Ok`
//!   (`UdpSocket::bind(...).await?` for base and cell). So a `true` flag means
//!   that service's socket is genuinely bound, not merely that a task was
//!   spawned.
//! - The admin listener binds *after* `start_all` returns, so the endpoint
//!   being reachable at all already implies the three services started.
//!
//! Reading the flags rather than stopping at "the admin port accepts TCP"
//! costs one request and removes the assumption that startup ordering never
//! changes.
//!
//! # Alternatives rejected
//!
//! - **A fixed sleep.** Fails under load and wastes time when fast; the
//!   brief's explicit non-goal.
//! - **Scraping stdout for `Server ready`.** Couples tests to a log string
//!   and to the tracing format, and requires draining pipes to avoid the
//!   child blocking on a full stdout buffer.
//! - **TCP-connect to the base port.** Base is UDP; connect proves nothing.
//! - **`database: true`.** Deliberately *not* required by default — the
//!   orchestrator treats a DB connection failure as non-fatal and starts
//!   anyway, so requiring it would hang against a server that is running fine
//!   by its own definition. Callers that need a DB opt in via
//!   [`ReadinessCheck::require_database`].

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

/// Path on the admin API that reports per-service status.
pub const STATUS_PATH: &str = "/api/config/status";

/// Which services must report healthy for the server to count as ready.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReadinessCheck {
    /// Require `services.database == true`. Off by default (`false`): the
    /// orchestrator starts successfully without a database, so requiring it
    /// unconditionally would hang against a server that is running fine.
    pub require_database: bool,
}

/// Outcome of a single readiness probe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// Every required service reported healthy.
    Ready,
    /// The server answered but is not fully up yet.
    NotReady { detail: String },
    /// The server could not be reached (still starting, or dead).
    Unreachable { detail: String },
}

/// Probe the admin status endpoint once.
///
/// Never blocks longer than `timeout` — a hung server must not wedge the
/// polling loop past its own deadline.
pub fn probe(admin_addr: SocketAddr, check: ReadinessCheck, timeout: Duration) -> ProbeOutcome {
    let body = match http_get(admin_addr, STATUS_PATH, timeout) {
        Ok(body) => body,
        Err(e) => {
            return ProbeOutcome::Unreachable {
                detail: e.to_string(),
            }
        }
    };

    evaluate_status_body(&body, check)
}

/// Decide readiness from a `/api/config/status` response body.
///
/// Split out from the socket work so the contract is unit-testable without a
/// server: this is where "which flags matter" actually lives.
pub fn evaluate_status_body(body: &str, check: ReadinessCheck) -> ProbeOutcome {
    let json: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            return ProbeOutcome::NotReady {
                detail: format!("status body was not JSON: {e}"),
            }
        }
    };

    let services = match json.get("services") {
        Some(s) => s,
        None => {
            return ProbeOutcome::NotReady {
                detail: "status body had no `services` object".to_string(),
            }
        }
    };

    // A missing flag is treated as not-ready rather than ready. If the
    // endpoint's shape ever changes, the harness must time out loudly rather
    // than declare a server ready it never actually checked.
    let flag = |name: &str| services.get(name).and_then(serde_json::Value::as_bool);

    let mut missing = Vec::new();
    for name in ["auth", "base", "cell"] {
        if flag(name) != Some(true) {
            missing.push(name);
        }
    }
    if check.require_database && flag("database") != Some(true) {
        missing.push("database");
    }

    if missing.is_empty() {
        ProbeOutcome::Ready
    } else {
        ProbeOutcome::NotReady {
            detail: format!("services not healthy: {}", missing.join(", ")),
        }
    }
}

/// Minimal blocking HTTP/1.1 GET.
///
/// Hand-rolled rather than pulling an HTTP client: the harness needs one
/// unauthenticated GET of a small JSON body from localhost, and staying
/// dependency-light keeps this crate usable from both sync `#[test]` and
/// async `#[tokio::test]` bodies without dragging in a runtime.
fn http_get(addr: SocketAddr, path: &str, timeout: Duration) -> io::Result<String> {
    let mut stream = TcpStream::connect_timeout(&addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    // `Connection: close` makes the server close the socket after the
    // response, which is what lets `read_to_end` terminate without parsing
    // Content-Length or chunked encoding.
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    // Signal end-of-request so a server waiting on more bytes responds now.
    let _ = stream.shutdown(Shutdown::Write);

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let text = String::from_utf8_lossy(&raw).into_owned();

    let (head, body) = text.split_once("\r\n\r\n").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "malformed HTTP response (no header/body separator)",
        )
    })?;

    let status_line = head.lines().next().unwrap_or_default();
    if !status_line.contains(" 200") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unexpected HTTP status: {status_line}"),
        ));
    }

    Ok(body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check() -> ReadinessCheck {
        ReadinessCheck::default()
    }

    fn body(auth: bool, base: bool, cell: bool, database: bool) -> String {
        format!(
            r#"{{"status":"ok","uptime_seconds":3,"services":{{"auth":{auth},"base":{base},"cell":{cell},"database":{database}}}}}"#
        )
    }

    #[test]
    fn all_services_healthy_is_ready() {
        assert_eq!(
            evaluate_status_body(&body(true, true, true, true), check()),
            ProbeOutcome::Ready
        );
    }

    /// The orchestrator starts successfully without a database, so a
    /// DB-less server must still count as ready by default. Requiring
    /// `database` unconditionally would hang every no-DB run.
    #[test]
    fn database_down_is_still_ready_by_default() {
        assert_eq!(
            evaluate_status_body(&body(true, true, true, false), check()),
            ProbeOutcome::Ready
        );
    }

    #[test]
    fn database_down_is_not_ready_when_required() {
        let outcome = evaluate_status_body(
            &body(true, true, true, false),
            ReadinessCheck {
                require_database: true,
            },
        );
        assert!(
            matches!(outcome, ProbeOutcome::NotReady { ref detail } if detail.contains("database")),
            "expected NotReady naming database, got {outcome:?}"
        );
    }

    /// Each service is load-bearing: a partially-started server must not be
    /// reported ready, or a test proceeds to talk to a socket nobody bound.
    #[test]
    fn any_single_service_down_is_not_ready() {
        for (auth, base, cell, expected) in [
            (false, true, true, "auth"),
            (true, false, true, "base"),
            (true, true, false, "cell"),
        ] {
            let outcome = evaluate_status_body(&body(auth, base, cell, true), check());
            assert!(
                matches!(outcome, ProbeOutcome::NotReady { ref detail } if detail.contains(expected)),
                "expected NotReady naming {expected}, got {outcome:?}"
            );
        }
    }

    /// A response shape the harness does not understand must fail closed.
    /// Reporting Ready on a missing `services` object would let an endpoint
    /// rename silently turn readiness into a no-op.
    #[test]
    fn missing_services_object_is_not_ready() {
        let outcome = evaluate_status_body(r#"{"status":"ok"}"#, check());
        assert!(
            matches!(outcome, ProbeOutcome::NotReady { .. }),
            "a body with no `services` object must fail closed, got {outcome:?}"
        );
    }

    /// A renamed/removed individual flag must also fail closed rather than
    /// being read as healthy.
    #[test]
    fn missing_individual_flag_is_not_ready() {
        let outcome = evaluate_status_body(
            r#"{"services":{"auth":true,"base":true}}"#, // no `cell`
            check(),
        );
        assert!(
            matches!(outcome, ProbeOutcome::NotReady { ref detail } if detail.contains("cell")),
            "a missing flag must be treated as unhealthy, got {outcome:?}"
        );
    }

    #[test]
    fn non_json_body_is_not_ready() {
        let outcome = evaluate_status_body("<html>502 Bad Gateway</html>", check());
        assert!(
            matches!(outcome, ProbeOutcome::NotReady { .. }),
            "non-JSON must not be read as ready, got {outcome:?}"
        );
    }

    /// A closed port must report Unreachable, not Ready. Pins that the probe
    /// distinguishes "not up yet" from "up and healthy" — collapsing the two
    /// would make the poll loop exit immediately against a dead server.
    #[test]
    fn probe_of_a_closed_port_is_unreachable() {
        // Reserve then release: nothing is listening on this port.
        let port = crate::ports::reserve_port(crate::ports::PortKind::Tcp).unwrap();
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let outcome = probe(addr, check(), Duration::from_millis(250));
        assert!(
            matches!(outcome, ProbeOutcome::Unreachable { .. }),
            "expected Unreachable for a port with no listener, got {outcome:?}"
        );
    }
}
