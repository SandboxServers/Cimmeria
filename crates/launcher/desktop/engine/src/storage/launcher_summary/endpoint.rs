//! Where summaries may be sent. The value comes only from native configuration,
//! never from a response, the webview or the environment.
use reqwest::Url;
use std::net::{Ipv4Addr, Ipv6Addr};

/// `https://…`, or `http://` to this machine for local fixtures only. A login server
/// is never used over plain http: `for_login_server` takes https, and
/// `upgraded_from_http_login_server` moves an http DNS name to https.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryEndpoint {
    base: Url,
}

/// The built-in https base (`https://<host>/api/`); LT-41 sets it. Never a plain-http value.
pub const DEFAULT_SUMMARY_BASE: Option<&str> = None;

impl SummaryEndpoint {
    /// The only constructor. A URL carrying credentials, a query or a fragment
    /// is refused, so none of them can ride along on a request.
    pub fn parse(value: &str) -> Option<Self> {
        let mut base = Url::parse(value).ok()?;
        let host = base.host_str()?;
        let allowed = match base.scheme() {
            "https" => true,
            "http" => loopback(host),
            _ => false,
        };
        if !allowed
            || !base.username().is_empty()
            || base.password().is_some()
            || base.query().is_some()
            || base.fragment().is_some()
        {
            return None;
        }
        // Route paths are appended to the base, so it must end with a slash.
        if !base.path().ends_with('/') {
            let path = format!("{}/", base.path());
            base.set_path(&path);
        }
        Some(Self { base })
    }

    /// `POST` target for a summary batch: the only URL ever requested.
    pub fn ingest_url(&self) -> Url {
        let mut url = self.base.clone();
        let joined = format!("{}telemetry/launcher-summary", url.path());
        url.set_path(&joined);
        url
    }

    /// The endpoint for a login server: its https host and port, with `/api/` as the
    /// base. The login server's own path is ignored. Plain http is refused even for
    /// loopback, because login servers are always https. An http login server is
    /// upgraded by `upgraded_from_http_login_server` instead. A login server carrying
    /// a username or password is refused too, and its value is never logged.
    pub fn for_login_server(login_server: &str) -> Option<Self> {
        let url = Url::parse(login_server).ok()?;
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return None;
        }
        let host = url.host_str()?;
        let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
        Self::parse(&format!("https://{host}{port}/api/"))
    }

    /// An http login server on a public DNS name: the same name over https on 443,
    /// with `/api/` as the base. The game keeps http; summaries never do.
    /// IP literals, localhost, `*.localhost`, single-label names and credentials are refused.
    pub fn upgraded_from_http_login_server(login_server: &str) -> Option<Self> {
        let url = Url::parse(login_server).ok()?;
        if url.scheme() != "http" || !url.username().is_empty() || url.password().is_some() {
            return None;
        }
        let name = url.host_str()?;
        // `Url` normalises IPv4 literals and wraps IPv6 in brackets.
        if name.parse::<Ipv4Addr>().is_ok() || name.starts_with('[') || !upgradable(name) {
            return None;
        }
        Self::parse(&format!("https://{name}/api/"))
    }

    /// The endpoint to use, with its source: the first login server when it is https,
    /// else that server's https upgrade when it is an http DNS name, else the built-in
    /// default, else none (`no_https_endpoint`).
    pub fn resolve(first_login_server: Option<&str>) -> (Option<Self>, &'static str) {
        if let Some(endpoint) = first_login_server.and_then(Self::for_login_server) {
            return (Some(endpoint), "login_server");
        }
        if let Some(endpoint) = first_login_server.and_then(Self::upgraded_from_http_login_server) {
            return (Some(endpoint), "login_server_https_upgrade");
        }
        match DEFAULT_SUMMARY_BASE.and_then(Self::parse) {
            Some(endpoint) => (Some(endpoint), "default"),
            None => (None, "no_https_endpoint"),
        }
    }
}

// A dotted DNS name that is not a local name. The caller has already refused IP literals.
fn upgradable(name: &str) -> bool {
    name.contains('.')
        && !name.ends_with('.')
        && name != "localhost"
        && !name.ends_with(".localhost")
}

// Only the exact name and literal loopback addresses; no resolver is consulted.
fn loopback(host: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn https_and_loopback_http_are_accepted_with_the_exact_route_url() {
        for (base, ingest) in [
            (
                "https://summaries.example",
                "https://summaries.example/telemetry/launcher-summary",
            ),
            (
                "https://summaries.example/api/",
                "https://summaries.example/api/telemetry/launcher-summary",
            ),
            (
                "https://summaries.example/api",
                "https://summaries.example/api/telemetry/launcher-summary",
            ),
            (
                "http://localhost:8081",
                "http://localhost:8081/telemetry/launcher-summary",
            ),
            (
                "http://127.0.0.1:9/base",
                "http://127.0.0.1:9/base/telemetry/launcher-summary",
            ),
            (
                "http://[::1]:9",
                "http://[::1]:9/telemetry/launcher-summary",
            ),
            // Scheme and host are compared in their normalised, lower-case form.
            (
                "HTTP://LOCALHOST:8081",
                "http://localhost:8081/telemetry/launcher-summary",
            ),
        ] {
            let endpoint = SummaryEndpoint::parse(base).unwrap_or_else(|| panic!("{base}"));
            assert_eq!(endpoint.ingest_url().as_str(), ingest);
        }
    }

    #[test]
    fn everything_else_is_refused_at_construction() {
        // Positive controls differing only in scheme or host.
        assert!(SummaryEndpoint::parse("https://collector.example").is_some());
        assert!(SummaryEndpoint::parse("http://localhost").is_some());
        for refused in [
            "http://collector.example",
            "http://example.org",
            "http://localhost.evil.example",
            "http://LOCALHOST.EVIL.EXAMPLE",
            "http://127.0.0.1.evil.example",
            "http://localhost.",
            "http://192.168.1.10",
            "http://0.0.0.0",
            "http://[2001:db8::1]",
            "http://[::ffff:127.0.0.1]",
            // A loopback name in the userinfo is not the host.
            "http://localhost@evil.example",
            "http://127.0.0.1:80@evil.example",
            "ftp://localhost",
            "file:///tmp/summaries",
            "wss://collector.example",
            "https://user:secret@collector.example",
            "https://user@collector.example",
            "https://collector.example/?key=1",
            "https://collector.example/#part",
            "collector.example",
            "",
        ] {
            assert_eq!(SummaryEndpoint::parse(refused), None, "{refused}");
        }
    }

    #[test]
    fn an_https_login_server_gives_its_api_base() {
        let with_port = SummaryEndpoint::for_login_server("https://play.example:8443");
        let ingest = with_port.map(|e| e.ingest_url().to_string());
        assert_eq!(
            ingest.as_deref(),
            Some("https://play.example:8443/api/telemetry/launcher-summary")
        );
        let with_path = SummaryEndpoint::for_login_server("https://play.example/login");
        let ingest = with_path.map(|e| e.ingest_url().to_string());
        assert_eq!(
            ingest.as_deref(),
            Some("https://play.example/api/telemetry/launcher-summary")
        );
    }

    #[test]
    fn plain_http_login_servers_are_refused() {
        // Positive control: the same host over https is accepted.
        assert!(SummaryEndpoint::for_login_server("https://127.0.0.1:8081").is_some());
        for refused in [
            "http://play.example:8081",
            "http://127.0.0.1:8081",
            "http://localhost",
        ] {
            assert_eq!(
                SummaryEndpoint::for_login_server(refused),
                None,
                "{refused}"
            );
        }
    }

    #[test]
    fn other_schemes_and_bad_values_are_refused() {
        for refused in [
            "ftp://x",
            "play.example",
            "https://user@play.example",
            "https://user:pw@play.example",
            "https://:pw@play.example",
            "",
        ] {
            assert_eq!(
                SummaryEndpoint::for_login_server(refused),
                None,
                "{refused}"
            );
        }
    }

    #[test]
    fn an_http_login_server_upgrades_to_https_on_443() {
        for login_server in ["http://play.example:8081", "http://Play.Example:8081/login"] {
            let endpoint = SummaryEndpoint::upgraded_from_http_login_server(login_server);
            let ingest = endpoint.map(|e| e.ingest_url().to_string());
            assert_eq!(
                ingest.as_deref(),
                Some("https://play.example/api/telemetry/launcher-summary"),
                "{login_server}"
            );
        }
    }

    #[test]
    fn the_upgrade_refuses_ips_local_names_and_credentials() {
        for refused in [
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
            assert_eq!(
                SummaryEndpoint::upgraded_from_http_login_server(refused),
                None,
                "{refused}"
            );
        }
    }

    #[test]
    fn resolve_falls_through_to_no_https_endpoint() {
        // LT-41 may set `DEFAULT_SUMMARY_BASE`, so the expected fall-through follows it.
        let fallback = if DEFAULT_SUMMARY_BASE.is_some() {
            "default"
        } else {
            "no_https_endpoint"
        };
        let (endpoint, source) = SummaryEndpoint::resolve(Some("http://play.example:8081"));
        assert!(endpoint.is_some());
        assert_eq!(source, "login_server_https_upgrade");
        let (_, source) = SummaryEndpoint::resolve(Some("http://127.0.0.1:8081"));
        assert_eq!(source, fallback);
        let (_, source) = SummaryEndpoint::resolve(None);
        assert_eq!(source, fallback);
        let (endpoint, source) = SummaryEndpoint::resolve(Some("https://play.example"));
        assert!(endpoint.is_some());
        assert_eq!(source, "login_server");
    }

    #[test]
    fn resolve_never_yields_plain_http_for_a_login_server() {
        for login_server in [
            "http://play.example:8081",
            "http://127.0.0.1:8081",
            "http://localhost:8081",
            "http://192.168.1.10:8081",
            "http://[2001:db8::1]:8081",
            "http://game.localhost:8081",
            "http://sgwserver:8081",
            "http://play.example.:8081",
            "http://user@play.example:8081",
            "http://u:p@play.example",
            "https://play.example",
            "https://127.0.0.1:8081",
            "ftp://play.example",
            "play.example",
            "",
        ] {
            let (endpoint, _) = SummaryEndpoint::resolve(Some(login_server));
            assert!(
                endpoint.is_none_or(|e| e.ingest_url().scheme() == "https"),
                "{login_server}"
            );
        }
    }

    #[test]
    fn the_default_base_is_none_or_https() {
        assert!(DEFAULT_SUMMARY_BASE.is_none_or(|b| b.starts_with("https://")));
    }
}
