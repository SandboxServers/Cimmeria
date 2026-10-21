//! Which upload endpoint the launcher may use for game telemetry (launcher-telemetry
//! C10). A loopback endpoint handed out by a remote login server would send uploads
//! to the player's own machine, so it is replaced, never used.
use reqwest::Url;
use std::net::{Ipv4Addr, Ipv6Addr};

/// The path every Cimmeria login port serves telemetry on.
pub const UPLOAD_PATH: &str = "/api/telemetry";

/// The built-in https upload URL. `None` until LT-41 sets it (D-LT17); no plain-http
/// value is ever allowed here.
pub const DEFAULT_UPLOAD_ENDPOINT: Option<&str> = None;

/// The upload endpoint to use, and whether it replaced the advertised one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effective {
    pub url: String,
    pub overridden: bool,
}

/// True when the host of `url` is `localhost`, in 127.0.0.0/8 or `::1`. False when `url` does not parse.
pub fn is_loopback_url(url: &str) -> bool {
    let Some(host) = Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
    else {
        return false;
    };
    // Literal addresses only; no resolver is consulted. IPv6 keeps its brackets.
    host == "localhost"
        || host
            .parse::<Ipv4Addr>()
            .is_ok_and(|address| address.is_loopback())
        || host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .and_then(|host| host.parse::<Ipv6Addr>().ok())
            .is_some_and(|address| address.is_loopback())
}

/// The server handed out a loopback upload endpoint, although the login
/// server it came from is not loopback, so uploads would go to the player's own machine.
pub fn misconfigured(login_server: &str, upload_endpoint: &str) -> bool {
    is_loopback_url(upload_endpoint) && !is_loopback_url(login_server)
}

/// D-LT7 (amended by directives 1 and 5): the upload endpoint the launcher uses.
/// `advertised` wins unless `misconfigured(login_server, advertised)`. Then:
/// 1. an `https` login server gives `https://<host[:port]>` of `login_server` plus
///    UPLOAD_PATH (`overridden = true`), so `https://play.example` gives
///    `https://play.example/api/telemetry`;
/// 2. otherwise `DEFAULT_UPLOAD_ENDPOINT` (`overridden = true`) when it is `Some`;
/// 3. otherwise `None`. A plain-http login server never yields a derived URL.
///
/// Also `None` when `login_server` does not parse.
pub fn effective_upload_endpoint(login_server: &str, advertised: &str) -> Option<Effective> {
    let login = Url::parse(login_server).ok()?;
    if !misconfigured(login_server, advertised) {
        return Some(Effective {
            url: advertised.to_string(),
            overridden: false,
        });
    }
    if login.scheme() == "https" {
        let host = login.host_str()?;
        let port = login
            .port()
            .map(|port| format!(":{port}"))
            .unwrap_or_default();
        return Some(Effective {
            url: format!("https://{host}{port}{UPLOAD_PATH}"),
            overridden: true,
        });
    }
    DEFAULT_UPLOAD_ENDPOINT.map(|url| Effective {
        url: url.to_string(),
        overridden: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOOPBACK_UPLOAD: &str = "http://localhost:8443/api/telemetry";

    #[test]
    fn a_loopback_endpoint_from_a_remote_login_is_misconfigured() {
        assert!(misconfigured("http://play.example:8081", LOOPBACK_UPLOAD));
    }

    #[test]
    fn a_loopback_endpoint_from_a_loopback_login_is_not_misconfigured() {
        assert!(!misconfigured("http://127.0.0.1:8081", LOOPBACK_UPLOAD));
        assert!(!misconfigured("http://localhost:8081", LOOPBACK_UPLOAD));
    }

    #[test]
    fn a_remote_https_endpoint_is_not_misconfigured() {
        assert!(!misconfigured(
            "https://play.example",
            "https://play.example/api/telemetry"
        ));
    }

    #[test]
    fn loopback_hosts_are_recognised() {
        assert!(is_loopback_url("http://localhost:8443/x"));
        assert!(is_loopback_url("http://127.4.5.6/x"));
        assert!(is_loopback_url("http://[::1]:8443/x"));
        assert!(!is_loopback_url("http://play.example/x"));
        assert!(!is_loopback_url("http://10.0.0.1/x"));
        assert!(!is_loopback_url("not a url"));
    }

    #[test]
    fn an_https_login_replaces_a_loopback_endpoint() {
        let effective = effective_upload_endpoint("https://play.example", LOOPBACK_UPLOAD).unwrap();
        assert_eq!(
            effective,
            Effective {
                url: "https://play.example/api/telemetry".into(),
                overridden: true,
            }
        );
    }

    #[test]
    fn a_plain_http_login_never_yields_a_derived_url() {
        assert_eq!(
            effective_upload_endpoint("http://play.example:8081", LOOPBACK_UPLOAD),
            None
        );
    }

    #[test]
    fn a_non_loopback_advertised_url_is_kept() {
        let advertised = "http://upload.example:8081/api/telemetry";
        assert_eq!(
            effective_upload_endpoint("http://play.example:8081", advertised),
            Some(Effective {
                url: advertised.into(),
                overridden: false,
            })
        );
    }

    #[test]
    fn a_loopback_advertised_url_from_a_loopback_login_is_kept() {
        let advertised = "http://127.0.0.1:8081/api/telemetry";
        assert_eq!(
            effective_upload_endpoint("http://127.0.0.1:8081", advertised),
            Some(Effective {
                url: advertised.into(),
                overridden: false,
            })
        );
    }

    #[test]
    fn an_unparsable_login_server_gives_none() {
        assert_eq!(
            effective_upload_endpoint("not a url", LOOPBACK_UPLOAD),
            None
        );
        assert_eq!(
            effective_upload_endpoint("", "https://play.example/x"),
            None
        );
    }

    #[test]
    fn the_default_endpoint_is_none_or_https() {
        assert!(DEFAULT_UPLOAD_ENDPOINT.is_none_or(|url| url.starts_with("https://")));
    }
}
