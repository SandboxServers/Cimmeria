//! Fail-closed configuration for the HTTP daemon (`cimmeria-lab --http`).
//!
//! The daemon starts only when the bind address is a loopback address and
//! `CIMMERIA_LAB_DAEMON_TOKEN` is at least [`MIN_TOKEN_LEN`] bytes. The rules
//! mirror the in-server lab endpoint (`crates/lab-mcp/src/config.rs`) with
//! one difference: the lab drives a desktop client on this machine, so there
//! is no allowed-hosts widening and no non-loopback bind, ever.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Shared-bearer-token environment variable.
pub const ENV_TOKEN: &str = "CIMMERIA_LAB_DAEMON_TOKEN";
/// Minimum accepted token length in bytes (same floor as lab-mcp).
pub const MIN_TOKEN_LEN: usize = 32;
/// The address `tools/lab/daemon.ps1` and `.mcp.json.example` use. Clear of
/// the bridge ports (8770 + one per instance, at most four) and lab-mcp (8444).
pub const DEFAULT_BIND: &str = "127.0.0.1:8779";
/// The only `Host` values the daemon accepts (rmcp's DNS-rebinding guard).
pub const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

/// Validated daemon configuration.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub bind: SocketAddr,
    pub token: String,
    /// Where the log goes (`--log-file`, default `labd.log` in [`state_dir`]).
    pub log_file: PathBuf,
}

/// Validate a `(bind, token)` pair. Pure so the refusals are testable
/// without touching process env.
pub fn evaluate(bind: &str, token: Option<String>) -> Result<(SocketAddr, String), String> {
    let addr: SocketAddr = bind
        .parse()
        .map_err(|e| format!("--http {bind:?} is not an ip:port address ({e})"))?;
    if !addr.ip().is_loopback() {
        return Err(format!(
            "--http {addr}: the lab daemon binds loopback only (127.0.0.1 or ::1); \
             it drives a desktop client and must never be reachable off this machine"
        ));
    }
    let token = token.filter(|t| !t.is_empty()).ok_or_else(|| {
        format!("{ENV_TOKEN} is unset; the HTTP daemon never runs without a token")
    })?;
    if token.len() < MIN_TOKEN_LEN {
        return Err(format!(
            "{ENV_TOKEN} is {} bytes; the lab daemon requires at least {MIN_TOKEN_LEN}",
            token.len()
        ));
    }
    Ok((addr, token))
}

/// `%LOCALAPPDATA%\cimmeria-lab` (the temp dir when `LOCALAPPDATA` is unset):
/// the daemon's log, pidfile and the UAT evidence root live here.
pub fn state_dir() -> PathBuf {
    std::env::var("LOCALAPPDATA")
        .map_or_else(|_| std::env::temp_dir(), PathBuf::from)
        .join("cimmeria-lab")
}

/// Command-line mode. Stdio stays the default so every existing `.mcp.json`
/// entry keeps working.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Stdio,
    Http {
        bind: String,
        log_file: Option<PathBuf>,
    },
}

/// Parse `cimmeria-lab [--http <addr>] [--log-file <path>]`.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Mode, String> {
    let mut http: Option<String> = None;
    let mut log_file: Option<PathBuf> = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--http" => {
                http = Some(
                    it.next()
                        .ok_or("--http needs an address, e.g. 127.0.0.1:8779")?,
                )
            }
            "--log-file" => {
                log_file = Some(it.next().ok_or("--log-file needs a path")?.into());
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    match (http, log_file) {
        (Some(bind), log_file) => Ok(Mode::Http { bind, log_file }),
        (None, Some(_)) => Err("--log-file only applies with --http".into()),
        (None, None) => Ok(Mode::Stdio),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(n: usize) -> Option<String> {
        Some("a".repeat(n))
    }

    #[test]
    fn a_short_token_is_refused() {
        let e = evaluate("127.0.0.1:8779", tok(MIN_TOKEN_LEN - 1)).unwrap_err();
        assert!(e.contains("at least 32"), "{e}");
        assert!(evaluate("127.0.0.1:8779", tok(MIN_TOKEN_LEN)).is_ok());
    }

    #[test]
    fn a_missing_or_empty_token_is_refused() {
        assert!(evaluate("127.0.0.1:8779", None).is_err());
        assert!(evaluate("127.0.0.1:8779", Some(String::new())).is_err());
    }

    /// Regression guard: the daemon must never listen off loopback.
    #[test]
    fn a_non_loopback_bind_is_refused() {
        for bind in ["0.0.0.0:8779", "10.0.0.5:8779", "[::]:8779"] {
            let e = evaluate(bind, tok(64)).unwrap_err();
            assert!(e.contains("loopback only"), "{bind}: {e}");
        }
        assert!(evaluate("[::1]:8779", tok(64)).is_ok());
        assert!(
            evaluate("localhost:8779", tok(64)).is_err(),
            "names are not addresses"
        );
    }

    #[test]
    fn stdio_stays_the_default() {
        assert_eq!(parse_args(Vec::<String>::new()).unwrap(), Mode::Stdio);
        let m = parse_args(["--http", "127.0.0.1:8779", "--log-file", "x.log"].map(String::from))
            .unwrap();
        assert_eq!(
            m,
            Mode::Http {
                bind: "127.0.0.1:8779".into(),
                log_file: Some("x.log".into())
            }
        );
        assert!(parse_args(["--http"].map(String::from)).is_err());
        assert!(parse_args(["--log-file", "x"].map(String::from)).is_err());
        assert!(parse_args(["--bogus"].map(String::from)).is_err());
    }
}
