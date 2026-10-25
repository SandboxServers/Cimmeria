//! The CT-31 end-to-end smoke (S3). A dev-mode `AuthService` gets the identity
//! wiring from a temporary providers file and is driven over its proxy
//! listener: the dev mock signs in, `link/new` creates the account, and the
//! launch ticket is spent on `UserAuth`. The TLS (proxy) listener accepts the
//! ticket; the plain listener refuses it with error 4. Needs the account
//! database (`require_db_or_skip!`).

use std::net::SocketAddr;
use std::sync::Arc;

use cimmeria_auth::auth::{AuthService, ShardInfo};
use cimmeria_common::config::ServerConfig;
use cimmeria_test_support::require_db_or_skip;
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use url::Url;

use crate::stores::pkce_challenge;
use crate::wiring::install_with;

/// The launcher's PKCE verifier (43 characters) and its redirect.
const VERIFIER: &str = "ct31-verifier-0123456789abcdefghijklmnopqrs";
const CB: &str = "http://127.0.0.1:49152/cb";
const STATE: &str = "ct31-state-0123456789";
const SUBJECT: &str = "ct31-smoke";
/// The test-only secret variable the providers file names. Set by the test
/// before the install; no other test reads it.
const SECRET_ENV: &str = "CT31_SMOKE_IDP_SECRET";
const LAUNCH: &str = "CT31-LAUNCH-0000001";
const LAUNCH_AGAIN: &str = "CT31-LAUNCH-0000002";
const PROTOCOL_DIGEST: &str = "58AFA196AD3AC4F65CADD99BFF23B799";

/// One trusted dev-mock provider `dev`, the one the launcher signs in with.
fn providers_file() -> String {
    format!(
        r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "dev"
kind = "dev_mock"
display_name = "Developer sign-in"
client_id = "dev-client"
client_secret_env = "{SECRET_ENV}"
trusted_email = true
"#
    )
}

/// A port that was free a moment ago. The service has no accessor for the
/// port it bound, so the test picks both ports up front.
fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

/// A request that carries the headers the TLS proxy adds: the TLS marker.
fn via_proxy(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    req.header("X-Forwarded-Proto", "https")
        .header("X-Forwarded-For", "203.0.113.7")
}

/// Sends `body` as JSON through the proxy and returns the status and body.
async fn send_json(req: reqwest::RequestBuilder, body: &Value) -> (reqwest::StatusCode, Value) {
    let resp = via_proxy(req)
        .header(CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    let status = resp.status();
    let text = resp.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

/// The `Location` header of a redirect.
fn location(resp: &reqwest::Response) -> String {
    resp.headers()[reqwest::header::LOCATION]
        .to_str()
        .unwrap()
        .to_owned()
}

/// Rebases a `Location` (absolute or relative) onto the test listener `base`,
/// keeping its path and query.
fn rebase(base: &str, location: &str) -> String {
    let url = Url::parse("https://play.example.org/")
        .unwrap()
        .join(location)
        .expect("a valid Location");
    let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
    format!("{base}{}{query}", url.path())
}

/// The value of query parameter `key` in `url`.
fn query_value(url: &str, key: &str) -> String {
    Url::parse(url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k.as_ref() == key)
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| panic!("no {key} in the redirect"))
}

/// A form body with the given fields, percent-encoded.
fn form(pairs: &[(&str, &str)]) -> String {
    Url::parse_with_params("http://form.invalid/", pairs)
        .unwrap()
        .query()
        .unwrap_or_default()
        .to_owned()
}

/// The `UserAuth` phase 1 body: the SHA-1 hex of `ticket` as the password.
fn user_auth(name: &str, ticket: &str) -> String {
    let digest: String = Sha1::digest(ticket.as_bytes())
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
    format!(
        r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="{name}" Password="{digest}" ProtocolDigest="{PROTOCOL_DIGEST}" />"#
    )
}

/// Runs the dev-mock sign-in through the proxy and returns the `signed_in`
/// body: the access token and the new account.
async fn sign_in(http: &reqwest::Client, proxy: &str) -> Value {
    let challenge = pkce_challenge(VERIFIER);
    let authorize = Url::parse_with_params(
        &format!("{proxy}/auth/v1/authorize"),
        &[
            ("provider", "dev"),
            ("redirect_uri", CB),
            ("state", STATE),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ],
    )
    .unwrap();
    let resp = via_proxy(http.get(authorize.as_str()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 302, "authorize refused");
    let dev_form = location(&resp);

    let server_state = query_value(&dev_form, "state");
    let body = form(&[
        ("state", &server_state),
        ("subject", SUBJECT),
        ("display_name", "CT31 Smoke"),
        ("email", "ct31@example.org"),
        ("verified", "true"),
    ]);
    let resp = via_proxy(
        http.post(rebase(proxy, &dev_form))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body),
    )
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 302, "dev-mock sign-in refused");
    let callback = location(&resp);

    let resp = via_proxy(http.get(rebase(proxy, &callback)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 302, "callback refused");
    let code = query_value(&location(&resp), "code");

    let (status, needs) = send_json(
        http.post(format!("{proxy}/auth/v1/token")),
        &json!({
            "grant_type": "authorization_code",
            "code": code,
            "code_verifier": VERIFIER,
            "redirect_uri": CB,
        }),
    )
    .await;
    assert_eq!(status, 200, "token refused");
    assert_eq!(needs["result"], "needs_link", "a new subject needs a link");
    let grant = needs["link_grant"]
        .as_str()
        .expect("a link_grant")
        .to_owned();

    let (status, signed) = send_json(
        http.post(format!("{proxy}/auth/v1/link/new")),
        &json!({ "link_grant": grant }),
    )
    .await;
    assert_eq!(status, 200, "link/new refused");
    assert_eq!(signed["result"], "signed_in", "link/new did not sign in");
    signed
}

/// Mints a launch ticket for `launch_id` with the bearer `access_token`.
async fn mint_ticket(
    http: &reqwest::Client,
    proxy: &str,
    access_token: &str,
    launch_id: &str,
) -> String {
    let (status, minted) = send_json(
        http.post(format!("{proxy}/auth/v1/launch-ticket"))
            .bearer_auth(access_token),
        &json!({ "launch_id": launch_id }),
    )
    .await;
    assert_eq!(status, 200, "launch-ticket refused");
    minted["ticket"].as_str().expect("a ticket").to_owned()
}

/// Sign-in through the proxy, then the launch ticket over the TLS listener
/// (success), then a fresh ticket on the plain listener (error 4). The
/// account the sign-in created is deleted at the end.
#[tokio::test]
async fn live_db_dev_mock_sign_in_reaches_userauth() {
    let pool = require_db_or_skip!();
    std::env::set_var(SECRET_ENV, "ct31-smoke-secret-value");
    let path =
        std::env::temp_dir().join(format!("ct31-auth-providers-{}.toml", std::process::id()));
    std::fs::write(&path, providers_file()).unwrap();

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
    // A login with no shard registered fails with "no shards" after the ticket.
    svc.register_shard(ShardInfo {
        name: "Test".into(),
        host: "127.0.0.1".into(),
        port: 1,
        protected: false,
    });
    svc.set_db_pool(Arc::new(pool.clone()));
    install_with(&mut svc, &path, true).expect("the providers file loads");
    let _ = std::fs::remove_file(&path);
    svc.start().await.unwrap();

    let plain = format!("http://127.0.0.1:{plain_port}");
    let proxy = format!("http://127.0.0.1:{proxy_port}");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let signed = sign_in(&http, &proxy).await;
    let access = signed["access_token"].as_str().expect("an access token");
    let name = signed["account"]["account_name"]
        .as_str()
        .expect("an account name")
        .to_owned();
    let account_id = signed["account"]["account_id"]
        .as_i64()
        .expect("an account id");

    // The ticket is spent over the proxy's TLS listener: success.
    let ticket = mint_ticket(&http, &proxy, access, LAUNCH).await;
    let resp = via_proxy(
        http.post(format!("{proxy}/SGWLogin/UserAuth"))
            .body(user_auth(&name, &ticket)),
    )
    .send()
    .await
    .unwrap();
    let xml = resp.text().await.unwrap();
    assert!(
        xml.contains("SGWLoginSuccess"),
        "proxy UserAuth refused: {xml}"
    );

    // The same kind of ticket on the plain listener is error 4: the ticket
    // crossed the wire in clear, so it is revoked and the login refused.
    let ticket = mint_ticket(&http, &proxy, access, LAUNCH_AGAIN).await;
    let resp = http
        .post(format!("{plain}/SGWLogin/UserAuth"))
        .body(user_auth(&name, &ticket))
        .send()
        .await
        .unwrap();
    let xml = resp.text().await.unwrap();
    assert!(
        xml.contains("SGWLoginError") && !xml.contains("SGWLoginSuccess"),
        "a ticket on the plain listener was accepted: {xml}"
    );
    assert!(
        xml.contains("The account name or password is incorrect."),
        "plain refusal is not error 4: {xml}"
    );

    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id as i32)
        .execute(&pool)
        .await;
}
