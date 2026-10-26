//! Auth-service settings read from the environment by `config_from_env`.
//!
//! Split out of `main.rs` to keep that file under its size cap. The variables
//! are listed in the env table of the `main.rs` module doc.

use cimmeria_common::{PeerNet, ServerConfig};

/// Apply the `AUTH_*` and `LOGON_PORT` environment variables to `cfg`.
///
/// An unparsable value keeps the default, except `AUTH_PROXY_ADDR` which is
/// logged (`reason=bad_auth_proxy_addr`) and left unset. A bad
/// `AUTH_PROXY_TRUSTED_PEERS` entry trusts no peer (see
/// [`apply_trusted_peers`]).
pub(crate) fn apply(cfg: &mut ServerConfig) {
    if let Ok(v) = std::env::var("AUTH_HOST") {
        cfg.auth_host = v;
    }
    if let Ok(v) = std::env::var("AUTH_PORT") {
        if let Ok(p) = v.parse() {
            cfg.auth_port = p;
        }
    }
    if let Ok(v) = std::env::var("LOGON_PORT") {
        if let Ok(p) = v.parse() {
            cfg.logon_port = p;
        }
    }
    if let Ok(v) = std::env::var("AUTH_TLS_CERT_PATH") {
        if !v.trim().is_empty() {
            cfg.auth_tls_cert_path = Some(v.into());
        }
    }
    if let Ok(v) = std::env::var("AUTH_TLS_KEY_PATH") {
        if !v.trim().is_empty() {
            cfg.auth_tls_key_path = Some(v.into());
        }
    }
    if let Ok(v) = std::env::var("AUTH_PROXY_ADDR") {
        // An empty value (compose `${AUTH_PROXY_ADDR:-}`) means unset.
        let v = v.trim();
        if !v.is_empty() {
            match v.parse() {
                Ok(addr) => cfg.auth_proxy_addr = Some(addr),
                Err(_) => tracing::warn!(
                    reason = "bad_auth_proxy_addr",
                    "AUTH_PROXY_ADDR is not a socket address; the proxy listener stays off"
                ),
            }
        }
    }
    let trusted_given = apply_trusted_peers(
        cfg,
        std::env::var("AUTH_PROXY_TRUSTED_PEERS").ok().as_deref(),
    );
    if cfg.auth_proxy_addr.is_some() && !trusted_given {
        tracing::warn!(
            reason = "auth_proxy_default_peers",
            "AUTH_PROXY_ADDR is set but AUTH_PROXY_TRUSTED_PEERS is not; trusting the built-in default list"
        );
    }
    if let Ok(v) = std::env::var("AUTH_TLS_RELOAD_INTERVAL_SECS") {
        if let Ok(secs) = v.parse() {
            cfg.auth_tls_reload_interval_secs = secs;
        } else {
            tracing::warn!(
                value = %v,
                "AUTH_TLS_RELOAD_INTERVAL_SECS is not a number; keeping default"
            );
        }
    }
}

/// Apply the `AUTH_PROXY_TRUSTED_PEERS` value `raw` to `cfg`. Returns whether
/// a non-blank value was given.
///
/// An unset or blank value keeps the built-in default list. A value whose
/// entries all parse (blank entries skipped) sets the list. Any other value
/// empties the list, so the proxy listener trusts no peer; only the reason is
/// logged, never the value.
fn apply_trusted_peers(cfg: &mut ServerConfig, raw: Option<&str>) -> bool {
    let v = match raw {
        Some(v) if !v.trim().is_empty() => v,
        _ => return false,
    };
    let parsed: Option<Vec<PeerNet>> = v
        .split(',')
        .filter(|entry| !entry.trim().is_empty())
        .map(PeerNet::parse)
        .collect();
    match parsed {
        Some(peers) if !peers.is_empty() => cfg.auth_proxy_trusted_peers = peers,
        _ => {
            cfg.auth_proxy_trusted_peers = Vec::new();
            tracing::error!(
                reason = "bad_auth_proxy_trusted_peers",
                "AUTH_PROXY_TRUSTED_PEERS has an entry that is not a CIDR network; the proxy listener trusts no peer"
            );
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bad entry in `AUTH_PROXY_TRUSTED_PEERS` empties the trusted list
    /// rather than keeping the default. Reverting that fails the assertion.
    #[test]
    fn trusted_peers_env_with_a_bad_entry_trusts_no_peer() {
        let mut cfg = ServerConfig::default();
        let given = apply_trusted_peers(&mut cfg, Some("127.0.0.0/8,not-a-net"));
        assert!(given);
        assert!(cfg.auth_proxy_trusted_peers.is_empty());
    }

    /// A good list replaces the default. Reverting the parse fails it.
    #[test]
    fn trusted_peers_env_parses_a_list() {
        let mut cfg = ServerConfig::default();
        let given = apply_trusted_peers(&mut cfg, Some("10.0.0.1/32, ::1"));
        assert!(given);
        let contains = |ip: &str| {
            cfg.auth_proxy_trusted_peers
                .iter()
                .any(|p| p.contains(ip.parse().unwrap()))
        };
        assert!(contains("10.0.0.1"));
        assert!(contains("::1"));
        assert!(!contains("172.17.0.1"));
    }

    /// An unset or blank value keeps the default list (loopback only) and
    /// reports no value.
    #[test]
    fn trusted_peers_env_blank_keeps_the_default() {
        for raw in [None, Some("  ")] {
            let mut cfg = ServerConfig::default();
            let given = apply_trusted_peers(&mut cfg, raw);
            assert!(!given);
            let contains = |ip: &str| {
                cfg.auth_proxy_trusted_peers
                    .iter()
                    .any(|p| p.contains(ip.parse().unwrap()))
            };
            assert!(contains("127.0.0.1"));
            assert!(!contains("172.17.0.1"));
        }
    }

    /// `AUTH_PROXY_ADDR` parses a socket address; a bad or empty value leaves
    /// the listener off. Reverting the parse makes the first assertion fail.
    #[test]
    fn proxy_addr_env_parses_or_stays_off() {
        std::env::set_var("AUTH_PROXY_ADDR", "203.0.113.5:13480");
        let mut cfg = ServerConfig::default();
        apply(&mut cfg);
        assert_eq!(
            cfg.auth_proxy_addr,
            Some("203.0.113.5:13480".parse().unwrap())
        );

        std::env::set_var("AUTH_PROXY_ADDR", "nonsense");
        let mut cfg = ServerConfig::default();
        apply(&mut cfg);
        assert!(cfg.auth_proxy_addr.is_none());

        std::env::set_var("AUTH_PROXY_ADDR", "  ");
        let mut cfg = ServerConfig::default();
        apply(&mut cfg);
        assert!(cfg.auth_proxy_addr.is_none());

        std::env::remove_var("AUTH_PROXY_ADDR");
    }
}
