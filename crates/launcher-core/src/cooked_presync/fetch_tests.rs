//! Tests for `fetch.rs`. Each one runs against a wiremock server on
//! `127.0.0.1` or a closed local port, with made-up categories and records.

use std::collections::BTreeMap;
use std::time::Duration;

use cimmeria_cooked_pak::catalog::{encode_body, CategoryBody, Index, IndexEntry};
use cimmeria_cooked_pak::digest::digest_hex;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;

const INDEX_PATH: &str = "/api/cooked/v1/index";
const BODY_PATH: &str = "/api/cooked/v1/category/7";

fn sample_index() -> Index {
    Index {
        schema: 1,
        categories: vec![IndexEntry {
            category_id: 7,
            pak: "CookedDataItems.pak".to_owned(),
            version: 3,
            entries: 2,
            changed: 1,
            body_bytes: 0,
            digest: digest_hex(&[0xAB; 32]),
        }],
    }
}

fn sample_body(version: u32) -> Vec<u8> {
    encode_body(&CategoryBody {
        category_id: 7,
        version,
        keys: vec![1, 2],
        changed: BTreeMap::from([(2, b"made-up record".to_vec())]),
        digest: [0xAB; 32],
    })
}

/// A fetcher whose login server is the mock itself.
fn fetcher(server: &MockServer) -> Fetcher {
    let url = server.uri();
    Fetcher::new(&url, std::slice::from_ref(&url)).expect("the mock is a login server")
}

#[tokio::test]
async fn index_and_body_decode() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_index()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(BODY_PATH))
        .and(query_param("version", "3"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(sample_body(3), "application/octet-stream"),
        )
        .mount(&server)
        .await;

    let fetcher = fetcher(&server);
    assert_eq!(fetcher.index().await.expect("index"), sample_index());
    let (body, bytes) = fetcher.body(7, 3).await.expect("body");
    assert_eq!(body.keys, vec![1, 2]);
    assert_eq!(bytes, sample_body(3).len() as u64);
}

#[tokio::test]
async fn status_codes_map_to_their_errors() {
    // Route, status, body, Retry-After, the error, its reason.
    let rows: Vec<(&str, u16, &str, Option<&str>, FetchError, &str)> = vec![
        (
            "index",
            404,
            "",
            None,
            FetchError::ServerUnsupported,
            "server_unsupported",
        ),
        (
            "index",
            503,
            r#"{"error":"disabled"}"#,
            None,
            FetchError::Disabled,
            "disabled",
        ),
        (
            "index",
            503,
            r#"{"error":"catalog_failed"}"#,
            None,
            FetchError::CatalogFailed,
            "catalog_failed",
        ),
        (
            "index",
            503,
            r#"{"error":"catalog_unavailable"}"#,
            Some("5"),
            FetchError::CatalogUnavailable {
                retry_after_s: Some(5),
            },
            "catalog_unavailable",
        ),
        (
            "index",
            503,
            "",
            None,
            FetchError::CatalogUnavailable {
                retry_after_s: None,
            },
            "catalog_unavailable",
        ),
        (
            "index",
            503,
            "",
            Some("Wed, 21 Oct 2015 07:28:00 GMT"),
            FetchError::CatalogUnavailable {
                retry_after_s: None,
            },
            "catalog_unavailable",
        ),
        (
            "index",
            429,
            r#"{"error":"rate_limited"}"#,
            Some("7"),
            FetchError::RateLimited {
                retry_after_s: Some(7),
            },
            "rate_limited",
        ),
        (
            "index",
            500,
            "",
            None,
            FetchError::HttpError(500),
            "http_error",
        ),
        (
            "body",
            404,
            r#"{"error":"unknown_category"}"#,
            None,
            FetchError::UnknownCategory,
            "unknown_category",
        ),
        (
            "body",
            409,
            r#"{"error":"version_changed","version":4}"#,
            None,
            FetchError::VersionChanged,
            "version_changed",
        ),
        (
            "body",
            503,
            r#"{"error":"catalog_failed"}"#,
            None,
            FetchError::CatalogFailed,
            "catalog_failed",
        ),
    ];
    for (route, status, body, retry_after, expected, reason) in rows {
        let server = MockServer::start().await;
        let mut response = ResponseTemplate::new(status).set_body_raw(body, "application/json");
        if let Some(value) = retry_after {
            response = response.insert_header("Retry-After", value);
        }
        let mock_path = if route == "index" {
            INDEX_PATH
        } else {
            BODY_PATH
        };
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(response)
            .mount(&server)
            .await;

        let fetcher = fetcher(&server);
        let error = if route == "index" {
            fetcher.index().await.map(|_| ())
        } else {
            fetcher.body(7, 3).await.map(|_| ())
        }
        .expect_err("an error status");
        assert_eq!(error, expected, "status {status} on {route}");
        assert_eq!(error.reason(), reason, "status {status} on {route}");
        assert_eq!(
            error.http_status(),
            Some(status),
            "status {status} on {route}"
        );
    }
}

#[tokio::test]
async fn a_closed_port_is_connect_failed_with_its_os_error() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind a local port");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);
    let url = format!("http://127.0.0.1:{port}");
    let fetcher = Fetcher::new(&url, std::slice::from_ref(&url)).expect("login server");
    match fetcher
        .index()
        .await
        .expect_err("nothing listens on the port")
    {
        FetchError::ConnectFailed(Some(detail)) => assert_eq!(detail.kind, "connection_refused"),
        other => panic!("expected connect_failed with an OS detail, got {other:?}"),
    }
}

#[tokio::test]
async fn a_silent_body_times_out_on_the_idle_timer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BODY_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(sample_body(3), "application/octet-stream")
                .set_delay(Duration::from_millis(500)),
        )
        .mount(&server)
        .await;
    let url = server.uri();
    let timeouts = Timeouts {
        idle: Duration::from_millis(100),
        ..Timeouts::DEFAULT
    };
    let fetcher =
        Fetcher::with_timeouts(&url, std::slice::from_ref(&url), timeouts).expect("login server");
    assert_eq!(fetcher.body(7, 3).await.err(), Some(FetchError::Timeout));
}

#[tokio::test]
async fn an_oversize_index_stops_at_the_cap() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(vec![b' '; INDEX_CAP + 1], "application/json"),
        )
        .mount(&server)
        .await;
    assert_eq!(
        fetcher(&server).index().await.err(),
        Some(FetchError::TooLarge)
    );
}

#[tokio::test]
async fn a_bad_pak_name_is_bad_index() {
    for name in ["../x.pak", "a/b.pak", "x.exe"] {
        let server = MockServer::start().await;
        let mut index = sample_index();
        index.categories[0].pak = name.to_owned();
        Mock::given(method("GET"))
            .and(path(INDEX_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(index))
            .mount(&server)
            .await;
        assert_eq!(
            fetcher(&server).index().await.err(),
            Some(FetchError::BadIndex),
            "{name}"
        );
    }
}

#[tokio::test]
async fn a_body_for_another_version_is_bad_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BODY_PATH))
        .and(query_param("version", "3"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw(sample_body(4), "application/octet-stream"),
        )
        .mount(&server)
        .await;
    assert_eq!(
        fetcher(&server).body(7, 3).await.err(),
        Some(FetchError::BadBody)
    );
}

#[test]
fn an_endpoint_outside_the_login_servers_is_refused() {
    let servers = vec!["http://127.0.0.1:8081".to_owned()];
    assert_eq!(
        Fetcher::new("http://127.0.0.1:8082", &servers).err(),
        Some(FetchError::EndpointRefused)
    );
    assert_eq!(
        Fetcher::new("https://127.0.0.1:8081", &servers).err(),
        Some(FetchError::EndpointRefused)
    );
    assert!(Fetcher::new("http://127.0.0.1:8081/", &servers).is_ok());
}

#[test]
fn server_label_names_host_and_port() {
    assert_eq!(
        server_label("http://play.example").as_deref(),
        Some("play.example:80")
    );
    assert_eq!(
        server_label("http://127.0.0.1:8081/").as_deref(),
        Some("127.0.0.1:8081")
    );
    assert_eq!(server_label("not a url"), None);
}
