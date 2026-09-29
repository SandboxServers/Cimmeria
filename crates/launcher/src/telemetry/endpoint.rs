//! Which server addresses telemetry may talk to, and the HTTP client
//! it uses.
//!
//! The launcher's shared client is `https_only`, which is right for the
//! manifest, seed and patch downloads. Telemetry went through it too,
//! while its default server is the local admin API,
//! `http://localhost:8443/api`, so every session failed before sending
//! a byte ("builder error"). Telemetry now has its own client, and this
//! policy instead: `https://` anywhere; plain `http://` to this machine,
//! or to the host and port of one of the launcher's `http://` login
//! servers. The rule applies to the configured auth URL and to every
//! upload endpoint the server hands back, so a server can't send the
//! uploads anywhere else unencrypted either.
//!
//! Why a login server may take plain http: the public server serves
//! telemetry on its SOAP login port, and the player's game client
//! already sends their password there over plain http. Decision
//! (@Cadacious, 2026-09-29): telemetry to that same host and port adds
//! no exposure, and it keeps telemetry zero-config for remote players.

use std::net::IpAddr;

use reqwest::Url;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EndpointError {
    #[error("telemetry server address {0:?} is not a valid URL")]
    Invalid(String),
    #[error(
        "telemetry needs an https:// server address; {0} is plain http to a machine \
         that is neither this one nor one of your login servers, so nothing was sent"
    )]
    InsecureRemote(String),
    #[error("telemetry server address {0} is not http or https")]
    Scheme(String),
}

/// Where telemetry may send. Built once per session from the launcher's
/// login servers and kept for the session, so a token refresh that hands
/// back a new upload endpoint is checked by the same rule.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EndpointPolicy {
    /// `(lowercased host, port)` of every `http://` login server.
    http_login_origins: Vec<(String, u16)>,
}

impl EndpointPolicy {
    /// Trust plain http to the host and port of each `http://` URL in
    /// `login_server_urls`. An `https://` login server adds nothing: the
    /// player does not already talk to it in plaintext, and https is
    /// accepted anywhere anyway.
    pub fn from_login_servers<'a>(login_server_urls: impl IntoIterator<Item = &'a str>) -> Self {
        let http_login_origins = login_server_urls
            .into_iter()
            .filter_map(|u| Url::parse(u).ok())
            .filter(|u| u.scheme() == "http")
            .filter_map(|u| origin(&u))
            .collect();
        Self { http_login_origins }
    }

    /// Accept `url` if telemetry may send to it.
    pub fn check(&self, url: &str) -> Result<(), EndpointError> {
        let parsed = Url::parse(url).map_err(|_| EndpointError::Invalid(url.to_string()))?;
        match parsed.scheme() {
            "https" => Ok(()),
            "http" if is_loopback(&parsed) => Ok(()),
            "http" if self.is_http_login_server(&parsed) => Ok(()),
            "http" => Err(EndpointError::InsecureRemote(url.to_string())),
            _ => Err(EndpointError::Scheme(url.to_string())),
        }
    }

    fn is_http_login_server(&self, url: &Url) -> bool {
        origin(url).is_some_and(|o| self.http_login_origins.contains(&o))
    }
}

/// Host and port, the port defaulted from the scheme (`http://host` is
/// port 80). The host must match exactly: `play.example` does not vouch
/// for `play.example.evil`.
fn origin(url: &Url) -> Option<(String, u16)> {
    Some((
        url.host_str()?.to_ascii_lowercase(),
        url.port_or_known_default()?,
    ))
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
        EndpointPolicy::default().check(url)
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

    /// The public server serves telemetry on its login port; the player
    /// already sends their password there in plaintext.
    #[test]
    fn plain_http_to_a_login_server_host_and_port_is_accepted() {
        let policy = with_login_servers(&["http://play.cimmeria.app:8081"]);
        for url in [
            "http://play.cimmeria.app:8081/api",
            "http://PLAY.cimmeria.app:8081/api/telemetry",
            "http://play.cimmeria.app:8081/api/telemetry/upload-chunk",
        ] {
            assert_eq!(policy.check(url), Ok(()), "{url}");
        }
        // Loopback and https still pass with a login-server policy.
        assert_eq!(policy.check("http://localhost:8443/api"), Ok(()));
        assert_eq!(policy.check("https://elsewhere.example/api"), Ok(()));
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

    /// A login server written without a port is port 80, like the URL.
    #[test]
    fn a_login_server_without_a_port_means_port_80() {
        let policy = with_login_servers(&["http://lan-server"]);
        assert_eq!(policy.check("http://lan-server:80/api"), Ok(()));
        assert_eq!(policy.check("http://lan-server/api"), Ok(()));
        assert!(policy.check("http://lan-server:8081/api").is_err());
    }

    /// Fresh-install defaults work together: the default auth URL is on a
    /// default login server.
    #[test]
    fn the_default_auth_url_passes_with_the_default_login_servers() {
        let servers = crate::client_setup::login_servers::default_servers();
        let policy = EndpointPolicy::from_login_servers(servers.iter().map(|s| s.url.as_str()));
        assert_eq!(
            policy.check(&crate::config::TelemetrySettings::default().auth_url),
            Ok(())
        );
        // Without the login servers the same URL is refused: the login
        // server list is what vouches for it.
        assert!(check(&crate::config::TelemetrySettings::default().auth_url).is_err());
    }
}
