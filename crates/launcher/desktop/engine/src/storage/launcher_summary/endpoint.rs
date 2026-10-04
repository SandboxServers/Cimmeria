//! Where summaries may be sent. The value comes only from native configuration,
//! never from a response, the webview or the environment.
use reqwest::Url;
use std::net::{Ipv4Addr, Ipv6Addr};

/// `https://…`, or `http://` to this machine for local fixtures. Nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryEndpoint {
    base: Url,
}

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

    /// `POST` target for the dev-session mint.
    pub fn mint_url(&self) -> Url {
        self.route("auth/dev-session")
    }

    /// `POST` target for a summary batch.
    pub fn ingest_url(&self) -> Url {
        self.route("telemetry/launcher-summary")
    }

    fn route(&self, path: &str) -> Url {
        let mut url = self.base.clone();
        let joined = format!("{}{path}", url.path());
        url.set_path(&joined);
        url
    }
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
    fn https_and_loopback_http_are_accepted_with_exact_route_urls() {
        for (base, mint, ingest) in [
            (
                "https://summaries.example",
                "https://summaries.example/auth/dev-session",
                "https://summaries.example/telemetry/launcher-summary",
            ),
            (
                "https://summaries.example/api/",
                "https://summaries.example/api/auth/dev-session",
                "https://summaries.example/api/telemetry/launcher-summary",
            ),
            (
                "http://localhost:8081",
                "http://localhost:8081/auth/dev-session",
                "http://localhost:8081/telemetry/launcher-summary",
            ),
            (
                "http://127.0.0.1:9/base",
                "http://127.0.0.1:9/base/auth/dev-session",
                "http://127.0.0.1:9/base/telemetry/launcher-summary",
            ),
            (
                "http://[::1]:9",
                "http://[::1]:9/auth/dev-session",
                "http://[::1]:9/telemetry/launcher-summary",
            ),
            // Scheme and host are compared in their normalised, lower-case form.
            (
                "HTTP://LOCALHOST:8081",
                "http://localhost:8081/auth/dev-session",
                "http://localhost:8081/telemetry/launcher-summary",
            ),
        ] {
            let endpoint = SummaryEndpoint::parse(base).unwrap_or_else(|| panic!("{base}"));
            assert_eq!(endpoint.mint_url().as_str(), mint);
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
}
