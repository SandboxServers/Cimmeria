//! Which server addresses telemetry may talk to, and the HTTP client
//! it uses.
//!
//! The launcher's shared client is `https_only`, which is right for the
//! manifest, seed and patch downloads. Telemetry went through it too,
//! while its default server is the local admin API,
//! `http://localhost:8443/api`, so every session failed before sending
//! a byte ("builder error"). Telemetry now has its own client, and this
//! policy instead: `https://` anywhere; plain `http://` only to this
//! machine. The rule applies to the configured auth URL and to every
//! upload endpoint the server hands back, so a server can't send the
//! uploads anywhere else unencrypted either.
//!
//! The owner withdrew the 2026-09-29 exception on 2026-10-24
//! (launcher-linux-install-telemetry D-LI11). Game telemetry reaches an
//! http login server on a DNS name over https on 443, under the same
//! name ([`https_upgrade_origin`]). An IP or single-word login server gets no
//! game telemetry. A dotted name, a LAN name such as `nas.lan` included, is
//! tried over https, and a failed connection simply sends nothing.

use std::net::{IpAddr, Ipv4Addr};

use reqwest::Url;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EndpointError {
    #[error("telemetry server address {0:?} is not a valid URL")]
    Invalid(String),
    #[error(
        "telemetry needs an https:// server address; {0} is plain http to another machine, \
         so nothing was sent"
    )]
    InsecureRemote(String),
    #[error("telemetry server address {0} is not http or https")]
    Scheme(String),
}

/// D-LI8: an `http://` login server on a public DNS name, as the https origin of the
/// same name on 443 (`"https://play.example"`). `None` for https and other schemes,
/// IP literals, `localhost`, `*.localhost`, single-label names, a trailing dot and
/// credentials. Never an http URL.
pub fn https_upgrade_origin(login_server: &str) -> Option<String> {
    let url = Url::parse(login_server).ok()?;
    if url.scheme() != "http" || !url.username().is_empty() || url.password().is_some() {
        return None;
    }
    let name = url.host_str()?;
    // `Url` normalises IPv4 literals and wraps IPv6 in brackets.
    if name.parse::<Ipv4Addr>().is_ok() || name.starts_with('[') || !upgradable(name) {
        return None;
    }
    Some(format!("https://{name}"))
}

/// A dotted DNS name that is not a local name. The caller has already refused IP literals.
fn upgradable(name: &str) -> bool {
    name.contains('.')
        && !name.ends_with('.')
        && name != "localhost"
        && !name.ends_with(".localhost")
}

/// Where telemetry may send. Built once per session and kept for the session, so a
/// token refresh that hands back a new upload endpoint is checked by the same rule.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndpointPolicy;

impl EndpointPolicy {
    /// Login servers no longer widen the rule (D-LI11): an http login server is
    /// reached through [`https_upgrade_origin`], never over plain http. Every session
    /// still builds its policy here.
    pub fn from_login_servers<'a>(_login_server_urls: impl IntoIterator<Item = &'a str>) -> Self {
        Self
    }

    /// Accept `url` if telemetry may send to it.
    pub fn check(&self, url: &str) -> Result<(), EndpointError> {
        let parsed = Url::parse(url).map_err(|_| EndpointError::Invalid(url.to_string()))?;
        match parsed.scheme() {
            "https" => Ok(()),
            "http" if is_loopback(&parsed) => Ok(()),
            "http" => Err(EndpointError::InsecureRemote(url.to_string())),
            _ => Err(EndpointError::Scheme(url.to_string())),
        }
    }
}

fn is_loopback(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    // IPv6 hosts come back bracketed.
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    bare.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// The client every telemetry request goes through. Not `https_only`
/// (see the module docs); redirects are off so a server cannot bounce a
/// checked https request to an address [`check`] would refuse.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("build a rustls HTTP client")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule with no login servers: https anywhere, http to loopback.
    fn check(url: &str) -> Result<(), EndpointError> {
        EndpointPolicy.check(url)
    }

    fn with_login_servers(urls: &[&str]) -> EndpointPolicy {
        EndpointPolicy::from_login_servers(urls.iter().copied())
    }

    #[test]
    fn https_is_accepted_anywhere() {
        assert_eq!(check("https://telemetry.example.org/api"), Ok(()));
    }

    /// The default auth URL, and the address a local dev server hands
    /// back for uploads.
    #[test]
    fn plain_http_to_this_machine_is_accepted() {
        for url in [
            "http://localhost:8443/api",
            "http://LOCALHOST:8443/api",
            "http://127.0.0.1:8443/api",
            "http://127.1.2.3/api",
            "http://[::1]:8443/api",
        ] {
            assert_eq!(check(url), Ok(()), "{url}");
        }
    }

    #[test]
    fn plain_http_to_another_machine_is_refused() {
        for url in [
            "http://play.cimmeria.app:8443/api",
            "http://192.168.1.10:8443/api",
            "http://localhost.evil.example/api",
        ] {
            assert!(
                matches!(check(url), Err(EndpointError::InsecureRemote(_))),
                "{url}"
            );
        }
    }

    #[test]
    fn junk_and_other_schemes_are_refused() {
        assert!(matches!(check("not a url"), Err(EndpointError::Invalid(_))));
        assert!(matches!(
            check("ftp://localhost/api"),
            Err(EndpointError::Scheme(_))
        ));
    }

    /// Revert test for the policy (D-LI11): an http login server no longer vouches
    /// for plain http to its host and port, on any path or port, nor a single-word name.
    #[test]
    fn a_login_server_never_vouches_for_plain_http() {
        let policy = with_login_servers(&["http://play.cimmeria.app:8081", "http://lan-server"]);
        for url in [
            "http://play.cimmeria.app:8081/api",
            "http://PLAY.cimmeria.app:8081/api/telemetry",
            "http://play.cimmeria.app:8081/api/telemetry/upload-chunk",
            "http://lan-server/api",
            "http://lan-server:80/api",
        ] {
            assert!(
                matches!(policy.check(url), Err(EndpointError::InsecureRemote(_))),
                "{url}"
            );
        }
        assert_eq!(policy.check("http://localhost:8443/api"), Ok(()));
        assert_eq!(policy.check("https://play.cimmeria.app/api"), Ok(()));
    }

    /// Only the exact host and port of an http login server: not another
    /// port on it (the private admin API), not a look-alike host, not an
    /// https-only login server's host.
    #[test]
    fn plain_http_outside_the_login_servers_is_refused() {
        let policy = with_login_servers(&[
            "http://play.cimmeria.app:8081",
            "https://secure.example.org",
            "not a url",
        ]);
        for url in [
            "http://play.cimmeria.app:8443/api",
            "http://play.cimmeria.app/api",
            "http://play.cimmeria.app.evil.example:8081/api",
            "http://evil.example:8081/api",
            "http://secure.example.org/api",
            "http://secure.example.org:443/api",
        ] {
            assert!(
                matches!(policy.check(url), Err(EndpointError::InsecureRemote(_))),
                "{url}"
            );
        }
    }

    #[test]
    fn https_upgrade_origin_keeps_the_name_and_drops_port_and_path() {
        for login_server in [
            "http://play.example:8081",
            "http://Play.Example:8081/login",
            "http://play.example",
        ] {
            assert_eq!(
                https_upgrade_origin(login_server).as_deref(),
                Some("https://play.example"),
                "{login_server}"
            );
        }
    }

    #[test]
    fn https_upgrade_origin_refuses_ips_local_names_and_credentials() {
        for login_server in [
            "http://127.0.0.1:8081",
            "http://192.168.1.10:8081",
            "http://[2001:db8::1]:8081",
            "http://localhost:8081",
            "http://game.localhost:8081",
            "http://sgwserver:8081",
            "http://play.example.:8081",
            "http://user@play.example:8081",
            "http://u:p@play.example",
            "https://play.example",
            "ftp://play.example",
        ] {
            assert_eq!(https_upgrade_origin(login_server), None, "{login_server}");
        }
    }

    /// Every upgraded origin is accepted by the policy, so the https upgrade
    /// is the one path a DNS-named http login server takes.
    #[test]
    fn an_upgraded_origin_passes_the_policy() {
        for login_server in [
            "http://play.example:8081",
            "http://Play.Example:8081/login",
            "http://play.example",
        ] {
            let origin = https_upgrade_origin(login_server).expect("upgradable");
            assert_eq!(
                check(&format!("{origin}/api/auth/dev-session")),
                Ok(()),
                "{origin}"
            );
        }
    }
}
