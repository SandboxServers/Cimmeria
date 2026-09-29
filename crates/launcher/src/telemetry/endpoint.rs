//! Which server addresses telemetry may talk to, and the HTTP client
//! it uses.
//!
//! The launcher's shared client is `https_only`, which is right for the
//! manifest, seed and patch downloads. Telemetry went through it too,
//! while its default server is the local admin API,
//! `http://localhost:8443/api`, so every session failed before sending
//! a byte ("builder error"). Telemetry now has its own client, and this
//! policy instead: `https://` anywhere, plain `http://` only to this
//! machine. The rule applies to the configured auth URL and to every
//! upload endpoint the server hands back, so a server can't send the
//! uploads somewhere unencrypted either.

use std::net::IpAddr;

use reqwest::Url;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EndpointError {
    #[error("telemetry server address {0:?} is not a valid URL")]
    Invalid(String),
    #[error(
        "telemetry needs an https:// server address; {0} is plain http to another \
         machine, so nothing was sent"
    )]
    InsecureRemote(String),
    #[error("telemetry server address {0} is not http or https")]
    Scheme(String),
}

/// Accept `url` if telemetry may send to it.
pub fn check(url: &str) -> Result<(), EndpointError> {
    let parsed = Url::parse(url).map_err(|_| EndpointError::Invalid(url.to_string()))?;
    match parsed.scheme() {
        "https" => Ok(()),
        "http" if is_loopback(&parsed) => Ok(()),
        "http" => Err(EndpointError::InsecureRemote(url.to_string())),
        _ => Err(EndpointError::Scheme(url.to_string())),
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

    #[test]
    fn the_default_auth_url_passes() {
        assert_eq!(
            check(&crate::config::TelemetrySettings::default().auth_url),
            Ok(())
        );
    }
}
