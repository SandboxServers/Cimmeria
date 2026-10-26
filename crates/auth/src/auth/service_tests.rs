use super::*;

/// An expired SID and an expired ticket are each dropped with one INFO
/// row carrying their account pair and a redacted prefix, a young ticket
/// survives, and no captured event holds a full SID or ticket.
#[test]
fn reap_expired_logs_unconsumed_ticket_with_account() {
    use crate::test_support::LogCapture;
    use std::net::Ipv4Addr;
    use std::time::Duration;
    use tracing::Level;

    let capture = LogCapture::install();
    let sessions = Mutex::new(HashMap::new());
    let pending = Mutex::new(HashMap::new());
    // Anchor every entry to `base` and reap at `base + SESSION_TTL + 1s`,
    // so no `now - duration` subtraction can underflow the clock on Windows.
    let base = Instant::now();
    let now = base + SESSION_TTL + Duration::from_secs(1);

    let sid = "SIDQRSTUV0123456789W".to_string();
    sessions.lock().unwrap().insert(
        sid.clone(),
        SessionRecord {
            account_id: 7,
            access_level: 0,
            test_account: false,
            account_name: "reaper-sid".to_string(),
            client_ip: Ipv4Addr::LOCALHOST.into(),
            created: base,
            ticket_guard: None,
        },
    );

    let expired_ticket = "ABCDEF0123456789ABCD".to_string();
    let young_ticket = "ZYXWVU9876543210TSRQ".to_string();
    for (ticket, created) in [
        (expired_ticket.clone(), base),
        (young_ticket.clone(), base + SESSION_TTL),
    ] {
        pending.lock().unwrap().insert(
            ticket.clone(),
            PendingLogin {
                account_id: 42,
                account_name: "reaper-user".to_string(),
                access_level: 0,
                test_account: false,
                ticket,
                session_key: String::new(),
                client_ip: Ipv4Addr::LOCALHOST.into(),
                created,
                ticket_guard: None,
            },
        );
    }

    let (sessions_dropped, tickets_dropped) = reap_expired(&sessions, &pending, now);

    assert_eq!((sessions_dropped, tickets_dropped), (1, 1));
    assert!(sessions.lock().unwrap().is_empty());
    {
        let map = pending.lock().unwrap();
        assert_eq!(map.len(), 1, "only the young ticket may survive");
        assert!(map.contains_key(&young_ticket));
    }

    let sid_row = capture
        .find_event(Level::INFO, "expired before it was used", "sid_unconsumed")
        .expect("expired SID must log an INFO row with reason `sid_unconsumed`");
    assert!(sid_row.has_field("account_id", "7"));
    assert!(sid_row.has_field("sid_prefix", &CredentialPrefix(&sid).to_string()));

    let ticket_row = capture
        .find_message(Level::INFO, "expired before BaseApp consumed")
        .expect("expired ticket must log an INFO row");
    assert!(ticket_row.has_field("account_id", "42"));
    assert!(ticket_row.has_field("reason", "ticket_unconsumed"));
    assert!(ticket_row.has_field("ticket_prefix", "ABCDEF…"));

    for event in capture.all() {
        for secret in [sid.as_str(), expired_ticket.as_str()] {
            let leaked = event.message.as_deref().is_some_and(|m| m.contains(secret))
                || event.fields.values().any(|v| v.contains(secret));
            assert!(!leaked, "full secret leaked into event: {event:?}");
        }
    }
}

#[test]
fn new_service_is_not_running() {
    let config = ServerConfig::default();
    let svc = AuthService::new(&config);
    assert!(!svc.is_running);
    assert_eq!(svc.listener_addr.port(), 13001);
    assert_eq!(svc.logon_addr.port(), 8081);
}

/// The unchecked developer login needs developer mode AND no database
/// configured. Developer mode alone, with the default (configured)
/// connection string, must not enable it.
#[test]
fn dev_login_fallback_needs_dev_mode_and_no_configured_db() {
    let dev_with_db = ServerConfig {
        developer_mode: true,
        ..ServerConfig::loopback()
    };
    assert!(!AuthService::new(&dev_with_db).dev_login_fallback_active());

    let no_dev_no_db = ServerConfig {
        db_connection_string: String::new(),
        ..ServerConfig::loopback()
    };
    assert!(!AuthService::new(&no_dev_no_db).dev_login_fallback_active());

    let dev_no_db = ServerConfig {
        developer_mode: true,
        db_connection_string: String::new(),
        ..ServerConfig::loopback()
    };
    assert!(AuthService::new(&dev_no_db).dev_login_fallback_active());
}

#[tokio::test]
async fn start_sets_running() {
    let config = ServerConfig {
        logon_port: 0, // OS-assigned port to avoid conflicts in tests
        ..ServerConfig::loopback()
    };
    let mut svc = AuthService::new(&config);
    svc.start().await.unwrap();
    assert!(svc.is_running);
}

/// The composition root mounts launcher telemetry on the login port
/// through `set_public_routes`; the extra routes must answer next to
/// the SOAP routes, with connect info available to their handlers.
#[tokio::test]
async fn public_routes_are_served_on_the_login_listener() {
    use axum::extract::ConnectInfo;
    use axum::routing::post;

    let port = {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let config = ServerConfig {
        logon_port: port,
        ..ServerConfig::loopback()
    };
    let mut svc = AuthService::new(&config);
    svc.set_public_routes(Router::new().route(
        "/api/telemetry/upload-chunk",
        post(|ConnectInfo(peer): ConnectInfo<SocketAddr>| async move { peer.ip().to_string() }),
    ));
    svc.start().await.unwrap();

    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{port}");
    let extra = client
        .post(format!("{base}/api/telemetry/upload-chunk"))
        .send()
        .await
        .unwrap();
    assert_eq!(extra.status(), 200);
    assert_eq!(extra.text().await.unwrap(), "127.0.0.1");

    // The SOAP route is still there (an empty body is a client error,
    // not a missing route).
    let soap = client
        .post(format!("{base}/SGWLogin/UserAuth"))
        .send()
        .await
        .unwrap();
    assert_ne!(soap.status(), 404);

    let absent = client
        .post(format!("{base}/api/telemetry/nope"))
        .send()
        .await
        .unwrap();
    assert_eq!(absent.status(), 404);
}

/// A port that was free a moment ago. The probe listener is dropped before
/// the server binds, so the port is reused rather than kept by the probe.
fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

/// A dev-mode service with no database, a `/ping` identity route and the
/// proxy listener on its own port. Returns the plain and proxy base URLs
/// once both listeners are up.
async fn start_with_identity_route() -> (String, String) {
    let plain_port = free_port();
    let proxy_port = free_port();
    let config = ServerConfig {
        logon_port: plain_port,
        developer_mode: true,
        db_connection_string: String::new(),
        auth_proxy_addr: Some(SocketAddr::from(([127, 0, 0, 1], proxy_port))),
        ..ServerConfig::loopback()
    };
    let mut svc = AuthService::new(&config);
    // A shard keeps a good login from failing with "no shards" after it.
    svc.register_shard(ShardInfo {
        name: "Test".into(),
        host: "127.0.0.1".into(),
        port: 1,
        protected: false,
    });
    svc.set_identity_routes(Box::new(|_deps: AuthRouteDeps| {
        Router::new().route("/ping", axum::routing::get(|| async { "pong" }))
    }));
    svc.start().await.unwrap();
    (
        format!("http://127.0.0.1:{plain_port}"),
        format!("http://127.0.0.1:{proxy_port}"),
    )
}

/// The identity routes sit under `/auth/v1` behind `require_tls`: the plain
/// listener refuses them, the proxy listener serves them once the proxy's
/// headers are present.
#[tokio::test]
async fn identity_routes_mount_under_auth_v1() {
    let (plain, proxy) = start_with_identity_route().await;
    let client = reqwest::Client::new();

    let refused = client
        .get(format!("{plain}/auth/v1/ping"))
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 403);

    let served = client
        .get(format!("{proxy}/auth/v1/ping"))
        .header("X-Forwarded-Proto", "https")
        .header("X-Forwarded-For", "203.0.113.9")
        .send()
        .await
        .unwrap();
    assert_eq!(served.status(), 200);
    assert_eq!(served.text().await.unwrap(), "pong");
}

/// `X-Forwarded-*` headers carry no trust on the plain listener: the request
/// still lacks the TLS marker and gets 403.
#[tokio::test]
async fn forwarded_headers_mean_nothing_on_the_plain_listener() {
    let (plain, _proxy) = start_with_identity_route().await;
    let resp = reqwest::Client::new()
        .get(format!("{plain}/auth/v1/ping"))
        .header("X-Forwarded-Proto", "https")
        .header("X-Forwarded-For", "203.0.113.9")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

/// A plaintext password arriving through the proxy carries the TLS marker to
/// the SOAP handler, so the dev-mode login succeeds rather than failing with
/// the plaintext-over-HTTP error.
#[tokio::test]
async fn userauth_over_proxy_counts_as_tls() {
    let (_plain, proxy) = start_with_identity_route().await;
    let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="someuser" Password="my-plaintext-password" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
    let xml = reqwest::Client::new()
        .post(format!("{proxy}/SGWLogin/UserAuth"))
        .header("Content-Type", "text/xml")
        .header("X-Forwarded-Proto", "https")
        .header("X-Forwarded-For", "203.0.113.9")
        .body(body)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
}
