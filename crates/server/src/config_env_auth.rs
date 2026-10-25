//! Auth-service settings read from the environment by `config_from_env`.
//!
//! Split out of `main.rs` to keep that file under its size cap. The variables
//! are listed in the env table of the `main.rs` module doc.

use cimmeria_common::{PeerNet, ServerConfig};

/// Apply the `AUTH_*` and `LOGON_PORT` environment variables to `cfg`.
///
/// An unparsable value keeps the default, except `AUTH_PROXY_ADDR` which is
/// logged (`reason=bad_auth_proxy_addr`) and left unset.
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
    if let Ok(v) = std::env::var("AUTH_PROXY_TRUSTED_PEERS") {
        // An empty value means the default list. Any entry that is not a
        // CIDR network (or bare address) keeps the default for the whole list.
        if !v.trim().is_empty() {
            let parsed: Option<Vec<PeerNet>> = v
                .split(',')
                .filter(|entry| !entry.trim().is_empty())
                .map(PeerNet::parse)
                .collect();
            match parsed {
                Some(peers) if !peers.is_empty() => cfg.auth_proxy_trusted_peers = peers,
                _ => tracing::warn!(
                    reason = "bad_auth_proxy_trusted_peers",
                    "AUTH_PROXY_TRUSTED_PEERS has an entry that is not a CIDR network; keeping the default list"
                ),
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
