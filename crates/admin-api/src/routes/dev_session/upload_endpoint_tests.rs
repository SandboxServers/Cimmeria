//! Tests for the https-only upload endpoint resolution (LT-01, owner
//! directives 1, 5 and 6).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::http::{header, HeaderMap, HeaderValue};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use super::handlers::{mint_with_headers, refresh_with_headers, Tables};
use super::tests::{ip, policy, request, secret, EnvGuard, NOW_UNIX};
use super::token::{decode_token, AuthError};
use super::upload_endpoint::{
    boot_warning, is_loopback_url, log_boot, resolve, valid_host, warn_unreachable_at,
    EndpointSource, ResolvedEndpoint, WarnThrottle, DEFAULT_UPLOAD_ENDPOINT,
};

const ENV_NAME: &str = "CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT";

fn headers(host: Option<&str>, proto: Option<&str>) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Some(host) = host {
        h.insert(header::HOST, HeaderValue::from_str(host).unwrap());
    }
    if let Some(proto) = proto {
        h.insert("x-forwarded-proto", HeaderValue::from_str(proto).unwrap());
    }
    h
}

#[derive(Clone, Default)]
struct Rows(Arc<Mutex<Vec<(Level, BTreeMap<String, String>)>>>);

struct FieldText<'a>(&'a mut BTreeMap<String, String>);

impl Visit for FieldText<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

impl<S: Subscriber> Layer<S> for Rows {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut fields = BTreeMap::new();
        event.record(&mut FieldText(&mut fields));
        // The message is a field named `message`, so it is searched too.
        self.0
            .lock()
            .unwrap()
            .push((*event.metadata().level(), fields));
    }
}

fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<(Level, BTreeMap<String, String>)>) {
    let rows = Rows::default();
    let sub = tracing_subscriber::registry().with(rows.clone());
    let out = tracing::subscriber::with_default(sub, f);
    let captured = rows.0.lock().unwrap().clone();
    (out, captured)
}

#[test]
fn env_wins_and_a_blank_env_falls_through() {
    let h = headers(Some("play.example"), Some("https"));
    let r = resolve(Some(" https://t.example/api/telemetry "), &h, false).unwrap();
    assert_eq!(r.url, "https://t.example/api/telemetry");
    assert_eq!(r.source, EndpointSource::Env);
    for blank in ["", "   "] {
        let r = resolve(Some(blank), &h, false).unwrap();
        assert_eq!(r.source, EndpointSource::RequestHost, "blank {blank:?}");
    }
}

#[test]
fn an_https_proxied_host_is_derived_and_plain_http_never_is() {
    let r = resolve(None, &headers(Some("play.example"), Some("https")), false).unwrap();
    assert_eq!(r.url, "https://play.example/api/telemetry");
    assert_eq!(r.source, EndpointSource::RequestHost);
    // The old derivation built http://{host}; a remote peer must now get None.
    assert_eq!(
        resolve(None, &headers(Some("play.example"), None), false),
        None
    );
    assert_eq!(
        resolve(None, &headers(Some("play.example"), Some("http")), false),
        None
    );
    // Even from a loopback peer a non-loopback Host is not plain-http derived.
    assert_eq!(
        resolve(None, &headers(Some("play.example"), None), true),
        None
    );
}

#[test]
fn a_loopback_host_works_only_from_a_loopback_peer() {
    let h = headers(Some("localhost:8443"), None);
    let r = resolve(None, &h, true).unwrap();
    assert_eq!(r.url, "http://localhost:8443/api/telemetry");
    assert_eq!(r.source, EndpointSource::LoopbackDev);
    assert_eq!(resolve(None, &h, false), None);
}

#[test]
fn a_missing_or_invalid_host_resolves_to_none() {
    let long = "a".repeat(300);
    for bad in ["evil/x", "a b", "user@host", long.as_str()] {
        let h = headers(Some(bad), Some("https"));
        // `a b` and friends may not even be a legal header value; skip those.
        if h.get(header::HOST).is_none() {
            continue;
        }
        assert_eq!(resolve(None, &h, true), None, "host {bad:?}");
    }
    assert_eq!(resolve(None, &headers(None, Some("https")), true), None);
}

#[test]
fn the_default_is_never_plain_http() {
    assert!(DEFAULT_UPLOAD_ENDPOINT.is_none_or(|u| u.starts_with("https://")));
}

#[test]
fn boot_warning_reasons() {
    assert_eq!(boot_warning(None), Some("upload_endpoint_unset"));
    assert_eq!(boot_warning(Some("  ")), Some("upload_endpoint_unset"));
    assert_eq!(
        boot_warning(Some("http://127.0.0.1:8081/api/telemetry")),
        Some("upload_endpoint_loopback")
    );
    assert_eq!(
        boot_warning(Some("http://play.example:8081/api/telemetry")),
        Some("upload_endpoint_plain_http")
    );
    assert_eq!(
        boot_warning(Some("https://play.example/api/telemetry")),
        None
    );
}

#[test]
fn valid_host_cases() {
    for ok in [
        "play.example",
        "play.example:8081",
        "10.0.0.1:1",
        "[::1]:8081",
        "[::1]",
        "a-b.c",
    ] {
        assert!(valid_host(ok), "{ok}");
    }
    let long = "a".repeat(254);
    for bad in [
        "",
        ":80",
        "a b",
        "evil/x",
        "a@b",
        "host:",
        "host:123456",
        "host:8o",
        "[::1",
        "[]",
        "[::1]x",
        "[::1]:",
        "a:b:c",
        long.as_str(),
    ] {
        assert!(!valid_host(bad), "{bad:?}");
    }
    assert!(valid_host(&"a".repeat(253)));
}

#[test]
fn is_loopback_url_cases() {
    for yes in [
        "http://localhost:8443/api/telemetry",
        "http://LOCALHOST/x",
        "http://127.0.0.1:8081/api/telemetry",
        "http://[::1]:8081/api/telemetry",
    ] {
        assert!(is_loopback_url(yes), "{yes}");
    }
    for no in [
        "https://play.example/api/telemetry",
        "http://203.0.113.9/x",
        "http://localhost.example/x",
        "not a url",
        "http://127.0.0.1@evil.example/x",
    ] {
        assert!(!is_loopback_url(no), "{no}");
    }
}

#[test]
fn boot_lines_carry_the_right_level_and_reason_and_no_ip() {
    let (_, rows) = capture(|| log_boot(None));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, Level::WARN);
    assert_eq!(rows[0].1["reason"], "upload_endpoint_unset");

    let (_, rows) = capture(|| log_boot(Some("http://127.0.0.1:8081/api/telemetry")));
    assert_eq!(rows[0].0, Level::WARN);
    assert_eq!(rows[0].1["reason"], "upload_endpoint_loopback");
    assert!(
        !format!("{rows:?}").contains("127.0.0.1"),
        "an IP literal must not be printed: {rows:?}"
    );

    let (_, rows) = capture(|| log_boot(Some("http://play.example:8081/api/telemetry")));
    assert_eq!(rows[0].0, Level::WARN);
    assert_eq!(rows[0].1["reason"], "upload_endpoint_plain_http");
    assert!(rows[0].1["upload_endpoint"].contains("play.example"));

    let (_, rows) = capture(|| log_boot(Some("https://play.example/api/telemetry")));
    assert_eq!(rows[0].0, Level::INFO);
    assert_eq!(rows[0].1["upload_endpoint_source"], "env");
}

#[test]
fn the_unreachable_warn_is_throttled_and_carries_no_url() {
    let throttle = WarnThrottle::new();
    let loopback = ResolvedEndpoint {
        url: "http://localhost:8081/api/telemetry".into(),
        source: EndpointSource::Env,
    };
    let (_, rows) = capture(|| {
        warn_unreachable_at(&throttle, 1_000, &loopback, false);
        warn_unreachable_at(&throttle, 1_001, &loopback, false);
    });
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].0, Level::WARN);
    assert_eq!(rows[0].1["reason"], "upload_endpoint_unreachable_for_peer");
    assert!(!format!("{rows:?}").contains("localhost"));
    // Past the window the next one fires and reports what it swallowed.
    let (_, rows) = capture(|| warn_unreachable_at(&throttle, 1_700, &loopback, false));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1["suppressed"], "1");
    // A loopback peer, or a non-loopback URL, never warns.
    let https = ResolvedEndpoint {
        url: "https://play.example/api/telemetry".into(),
        source: EndpointSource::Env,
    };
    let (_, rows) = capture(|| {
        warn_unreachable_at(&throttle, 9_000, &loopback, true);
        warn_unreachable_at(&throttle, 9_000, &https, false);
    });
    assert!(rows.is_empty(), "{rows:?}");
}

// Router-level: the handlers' own path, not just `resolve`.

#[test]
fn mint_through_an_https_proxy_advertises_the_host() {
    let _g = EnvGuard::install();
    std::env::remove_var(ENV_NAME);
    let resp = mint_with_headers(
        &Tables::new(),
        &policy(10, 10, 10),
        ip("198.51.100.7"),
        &headers(Some("play.example"), Some("https")),
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    assert_eq!(resp.upload_endpoint, "https://play.example/api/telemetry");
    decode_token(&resp.token, &secret()).unwrap();
}

#[test]
fn mint_without_a_resolvable_endpoint_is_refused_with_503_and_no_token() {
    let _g = EnvGuard::install();
    std::env::remove_var(ENV_NAME);
    let t = Tables::new();
    let err = mint_with_headers(
        &t,
        &policy(10, 10, 10),
        ip("198.51.100.7"),
        &headers(Some("play.example"), None),
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::NoUploadEndpoint), "{err:?}");
    assert!(err.to_string().starts_with("no_upload_endpoint"));
    assert_eq!(
        axum::response::IntoResponse::into_response(err).status(),
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
}

#[test]
fn refresh_without_a_resolvable_endpoint_is_refused_too() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let minted = mint_with_headers(
        &t,
        &policy(10, 10, 10),
        ip("198.51.100.7"),
        &headers(None, None),
        request("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    std::env::remove_var(ENV_NAME);
    let err = refresh_with_headers(
        &t,
        &policy(10, 10, 10),
        ip("198.51.100.7"),
        &headers(Some("play.example"), None),
        &minted.token,
        Instant::now(),
        NOW_UNIX + 10,
    )
    .unwrap_err();
    assert!(matches!(err, AuthError::NoUploadEndpoint), "{err:?}");
}

// Directive 6: neither the peer nor an IP-literal Host reaches a log row.
#[test]
fn a_mint_logs_no_peer_ip_and_no_host_ip() {
    let _g = EnvGuard::install();
    std::env::remove_var(ENV_NAME);
    let (resp, rows) = capture(|| {
        mint_with_headers(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("198.51.100.7"),
            &headers(Some("203.0.113.9:8081"), Some("https")),
            request("3f2504e0-4f89-41d3-9a0c-0305e82c3301"),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap()
    });
    // The client still gets the URL; only the logs must not.
    assert_eq!(
        resp.upload_endpoint,
        "https://203.0.113.9:8081/api/telemetry"
    );
    let minted = rows
        .iter()
        .find(|(l, f)| *l == Level::INFO && f.contains_key("upload_endpoint_source"))
        .expect("the mint INFO row");
    assert_eq!(minted.1["upload_endpoint_source"], "request_host");
    assert_eq!(minted.1["upload_endpoint_https"], "true");
    let text = format!("{rows:?}");
    assert!(!text.contains("198.51.100.7"), "peer ip in logs: {text}");
    assert!(!text.contains("203.0.113.9"), "host ip in logs: {text}");
}
