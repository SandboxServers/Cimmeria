//! The sign-in routes: the provider list, `authorize`, `callback` and the
//! developer-mode mock form (contract C4 and C6).
//!
//! The launcher's PKCE pair stays with the launcher; the server runs its own
//! pair against the provider and keeps the pending record under a fresh
//! server `state` for 10 minutes. The callback mints a single-use handoff code
//! bound to the launcher's challenge and redirect URI.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Instant;

use axum::extract::rejection::QueryRejection;
use axum::extract::{ConnectInfo, Form, Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use tracing::Instrument;
use url::form_urlencoded::Serializer;
use url::{Host, Url};

use crate::limits::Bucket;
use crate::provider::{IdentityProvider, ProviderError, VerifiedIdentity};
use crate::routes::ctx::{client_ip, RouteState};
use crate::routes::error::ApiError;
use crate::stores::{pkce_challenge, random_b64url, HandoffOutcome, PendingAuth};
use crate::telemetry;

/// The query of `GET /authorize` (C6). Every field is optional here so that
/// a missing one is our `400 invalid_request`, not the extractor's.
#[derive(Deserialize)]
pub struct AuthorizeQuery {
    provider: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    intent: Option<String>,
}

/// The query of `GET /callback/{provider}`: the provider's answer.
#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// The query of the dev-mock form: the server state the authorize URL carried.
#[derive(Deserialize)]
pub struct DevMockQuery {
    state: Option<String>,
}

/// The posted dev-mock form (C4): the identity the mock will vouch for.
#[derive(Deserialize)]
pub struct DevMockForm {
    state: Option<String>,
    subject: Option<String>,
    display_name: Option<String>,
    email: Option<String>,
    verified: Option<String>,
}

/// The static page for an unknown or expired sign-in (C6). It echoes nothing.
const EXPIRED_PAGE: &str = "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
<title>Sign-in expired</title></head><body><p>This sign-in link has expired. \
Return to the launcher and try again.</p></body></html>\n";

/// `GET /providers`: the providers, the switches and the configured links (C6).
pub async fn providers(State(state): State<RouteState>) -> Response {
    let cfg = &state.ctx.cfg;
    let list: Vec<Value> = if cfg.kill.idp_disabled {
        Vec::new()
    } else {
        state
            .ctx
            .providers
            .list()
            .iter()
            .map(|provider| json!({"id": provider.id().as_str(), "display_name": provider.display_name()}))
            .collect()
    };
    let mut body = json!({
        "api": 1,
        "soap_relay": !cfg.kill.soap_relay_disabled,
        "signup_open": cfg.signup_open,
        "providers": list,
    });
    let links: Map<String, Value> = [
        ("website", &cfg.links.website),
        ("account", &cfg.links.account),
        ("forums", &cfg.links.forums),
    ]
    .into_iter()
    .filter_map(|(name, url)| url.as_ref().map(|url| (name.to_owned(), json!(url))))
    .collect();
    if !links.is_empty() {
        body["links"] = Value::Object(links);
    }
    Json(body).into_response()
}

/// `GET /authorize`: validates the launcher's request, then redirects to the
/// provider with the server's own PKCE pair (C6).
pub async fn authorize(
    State(state): State<RouteState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    query: Result<Query<AuthorizeQuery>, QueryRejection>,
) -> Response {
    let span = telemetry::idp_authorize();
    let started = Instant::now();
    let ip = client_ip(ConnectInfo(peer));
    let Ok(Query(query)) = query else {
        let response = ApiError::InvalidRequest.into_response();
        record_outcome(&span, started, &response);
        return response;
    };
    let response = match start_authorize(&state, &ip, query)
        .instrument(span.clone())
        .await
    {
        Ok(response) => response,
        Err(error) => error.into_response(),
    };
    record_outcome(&span, started, &response);
    response
}

/// The body of `authorize`. Validation comes first, then the `Authorize`
/// limit, so a refused request never mints anything.
async fn start_authorize(
    state: &RouteState,
    ip: &str,
    query: AuthorizeQuery,
) -> Result<Response, ApiError> {
    let now = Instant::now();
    let provider_id = query.provider.as_deref().ok_or(ApiError::InvalidRequest)?;
    let redirect_uri = query
        .redirect_uri
        .as_deref()
        .filter(|uri| valid_redirect_uri(uri))
        .ok_or(ApiError::InvalidRequest)?;
    let launcher_state = query
        .state
        .as_deref()
        .filter(|s| valid_state(s))
        .ok_or(ApiError::InvalidRequest)?;
    let launcher_challenge = query
        .code_challenge
        .as_deref()
        .filter(|c| valid_challenge(c))
        .ok_or(ApiError::InvalidRequest)?;
    if query.code_challenge_method.as_deref() != Some("S256") {
        return Err(ApiError::InvalidRequest);
    }
    state.ctx.limit(Bucket::Authorize, ip, now)?;
    let running = state
        .ctx
        .providers
        .get(provider_id)
        .cloned()
        .ok_or(ApiError::UnknownProvider)?;
    tracing::Span::current().record("provider", running.id().as_str());
    running.prepare().await.map_err(|error| {
        tracing::warn!(
            provider = running.id().as_str(),
            reason = error.reason(),
            "auth.idp.prepare_failed"
        );
        ApiError::Unavailable
    })?;

    // `intent` comes from `link/start`: a known one is spent here, an unknown
    // one is refused. Spent only after the provider checks, so an unknown or
    // unavailable provider does not burn the user's single-use token (W3-8).
    let intent = match query.intent.as_deref() {
        Some(token) => Some(
            state
                .ctx
                .stores
                .intents
                .take(token, now)
                .ok_or(ApiError::InvalidRequest)?,
        ),
        None => None,
    };

    let verifier = random_b64url(32);
    let server_challenge = pkce_challenge(&verifier);
    let nonce = random_b64url(16);
    let server_state = state.ctx.stores.pending.insert(
        PendingAuth {
            provider: provider_id.to_owned(),
            redirect_uri: redirect_uri.to_owned(),
            launcher_state: launcher_state.to_owned(),
            launcher_challenge: launcher_challenge.to_owned(),
            pkce_verifier: verifier,
            nonce: nonce.clone(),
            intent,
        },
        now,
    );
    let callback = callback_uri(state, running.id().as_str());
    let location = running.authorize_url(&server_state, &server_challenge, &nonce, &callback);
    found(&location)
}

/// `GET /callback/{provider}`: the provider's answer. Redirects the launcher
/// with a handoff code, or with `error` when the sign-in was refused (C6).
pub async fn callback(
    State(state): State<RouteState>,
    Path(provider): Path<String>,
    query: Result<Query<CallbackQuery>, QueryRejection>,
) -> Response {
    let span = telemetry::idp_callback();
    let started = Instant::now();
    let response = match query {
        Ok(Query(query)) => {
            complete_callback(&state, &provider, query)
                .instrument(span.clone())
                .await
        }
        Err(_) => expired_page(),
    };
    record_outcome(&span, started, &response);
    response
}

/// The body of `callback`. The pending record is spent by `take`, whatever
/// the provider answered.
async fn complete_callback(state: &RouteState, provider: &str, query: CallbackQuery) -> Response {
    let now = Instant::now();
    let pending = match query
        .state
        .as_deref()
        .and_then(|s| state.ctx.stores.pending.take(s, now))
    {
        Some(pending) if pending.provider == provider => pending,
        _ => return expired_page(),
    };
    tracing::Span::current().record("provider", provider);
    let refused = |error: &str, reason: &'static str| {
        with_reason(
            redirect_to(
                &pending.redirect_uri,
                &[("error", error), ("state", pending.launcher_state.as_str())],
            ),
            reason,
        )
    };
    if query.error.is_some() {
        return refused("access_denied", "provider_error");
    }
    let Some(code) = query.code.as_deref() else {
        return refused("access_denied", "no_code");
    };
    let Some(running) = state.ctx.providers.get(provider).cloned() else {
        return expired_page();
    };
    let callback = callback_uri(state, provider);
    match running
        .exchange(code, &pending.pkce_verifier, &pending.nonce, &callback)
        .await
    {
        Ok(identity) => {
            let handoff = state.ctx.stores.handoff.issue(
                HandoffOutcome {
                    identity,
                    intent: pending.intent,
                },
                &pending.launcher_challenge,
                &pending.redirect_uri,
                now,
            );
            redirect_to(
                &pending.redirect_uri,
                &[
                    ("code", handoff.as_str()),
                    ("state", pending.launcher_state.as_str()),
                ],
            )
        }
        Err(ProviderError::Denied) => refused("access_denied", "denied"),
        Err(error) => {
            tracing::warn!(
                provider,
                reason = error.reason(),
                "auth.idp.exchange_failed"
            );
            refused("server_error", error.reason())
        }
    }
}

/// `GET /dev-mock/authorize`: the developer-mode identity form (C4).
pub async fn dev_mock_form(
    State(state): State<RouteState>,
    query: Result<Query<DevMockQuery>, QueryRejection>,
) -> Response {
    if state.ctx.providers.dev_mock().is_none() {
        return ApiError::NotFound.into_response();
    }
    let Ok(Query(query)) = query else {
        return ApiError::InvalidRequest.into_response();
    };
    let Some(server_state) = query.state.as_deref().filter(|s| valid_state(s)) else {
        return ApiError::InvalidRequest.into_response();
    };
    let action = format!(
        "{}/auth/v1/dev-mock/authorize",
        state.ctx.cfg.public_base_url
    );
    Html(form_page(&action, server_state)).into_response()
}

/// `POST /dev-mock/authorize`: issues the mock's code for the posted identity
/// and redirects to the mock's callback (C4).
pub async fn dev_mock_post(
    State(state): State<RouteState>,
    Form(form): Form<DevMockForm>,
) -> Response {
    match issue_dev_mock(&state, form) {
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

fn issue_dev_mock(state: &RouteState, form: DevMockForm) -> Result<Response, ApiError> {
    let dev = state.ctx.providers.dev_mock().ok_or(ApiError::NotFound)?;
    let server_state = form
        .state
        .as_deref()
        .filter(|s| valid_state(s))
        .ok_or(ApiError::InvalidRequest)?;
    let subject = form
        .subject
        .as_deref()
        .filter(|s| valid_subject(s))
        .ok_or(ApiError::InvalidRequest)?;
    let display_name =
        VerifiedIdentity::sanitized_display_name(form.display_name.as_deref().unwrap_or_default());
    let email = match form.email.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(raw) if valid_email(raw) => Some(raw.to_owned()),
        Some(_) => return Err(ApiError::InvalidRequest),
    };
    let identity = VerifiedIdentity {
        provider: dev.id().clone(),
        issuer: state.ctx.cfg.public_base_url.clone(),
        subject: subject.to_owned(),
        display_name,
        email_verified: form.verified.is_some() && email.is_some(),
        email,
        provider_account_created_ms: None,
    };
    let code = dev.issue(identity);
    let query = Serializer::new(String::new())
        .append_pair("code", &code)
        .append_pair("state", server_state)
        .finish();
    found(&format!("/auth/v1/callback/{}?{query}", dev.id().as_str()))
}

/// The developer-mode form. Its values are escaped before they are echoed.
fn form_page(action: &str, server_state: &str) -> String {
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\
<title>Developer sign-in</title></head><body><h1>Developer sign-in</h1>\
<form method=\"post\" action=\"{action}\">\
<input type=\"hidden\" name=\"state\" value=\"{state}\">\
<p><label>Subject <input name=\"subject\" required maxlength=\"64\"></label></p>\
<p><label>Display name <input name=\"display_name\" maxlength=\"64\"></label></p>\
<p><label>Email <input name=\"email\" type=\"email\" maxlength=\"254\"></label></p>\
<p><label><input type=\"checkbox\" name=\"verified\"> Email verified</label></p>\
<p><button type=\"submit\">Sign in</button></p></form></body></html>\n",
        action = escape(action),
        state = escape(server_state),
    )
}

/// The callback URI the provider returns to: `{public_base_url}/auth/v1/callback/{id}`.
fn callback_uri(state: &RouteState, provider: &str) -> String {
    format!(
        "{}/auth/v1/callback/{provider}",
        state.ctx.cfg.public_base_url
    )
}

/// `302` to `location`. The value is built from validated parts, so the
/// header is always valid; a failure answers `503`.
fn found(location: &str) -> Result<Response, ApiError> {
    let value = HeaderValue::from_str(location).map_err(|_| ApiError::Unavailable)?;
    Ok((StatusCode::FOUND, [(header::LOCATION, value)]).into_response())
}

/// `302` to `redirect_uri` with `pairs` in its query. The query is built with
/// `url`, never by string concatenation.
fn redirect_to(redirect_uri: &str, pairs: &[(&str, &str)]) -> Response {
    let Ok(mut url) = Url::parse(redirect_uri) else {
        return ApiError::Unavailable.into_response();
    };
    url.query_pairs_mut().extend_pairs(pairs.iter().copied());
    found(url.as_str()).unwrap_or_else(IntoResponse::into_response)
}

/// The static `400` page for an unknown or expired sign-in.
fn expired_page() -> Response {
    (
        StatusCode::BAD_REQUEST,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        EXPIRED_PAGE,
    )
        .into_response()
}

/// The refusal reason a route attaches to a response. It is never sent to
/// the client; `record_outcome` reads it for the span.
#[derive(Clone, Copy)]
struct Reason(&'static str);

/// Attaches the fixed refusal `reason` to `response`.
fn with_reason(mut response: Response, reason: &'static str) -> Response {
    response.extensions_mut().insert(Reason(reason));
    response
}

/// Records the span's outcome, reason and latency from the response.
pub(super) fn record_outcome(span: &tracing::Span, started: Instant, response: &Response) {
    let status = response.status();
    let reason = response.extensions().get::<Reason>().map(|r| r.0);
    let outcome = if reason.is_some() || status.is_client_error() || status.is_server_error() {
        "refused"
    } else {
        "ok"
    };
    span.record("outcome", outcome);
    span.record(
        "reason",
        reason.unwrap_or_else(|| status.canonical_reason().unwrap_or("unknown")),
    );
    span.record("latency_ms", started.elapsed().as_millis() as u64);
}

/// A `redirect_uri` the launcher may use: `http://127.0.0.1:<port>/cb` or
/// `http://[::1]:<port>/cb`, port 1024 to 65535, with no query, fragment or
/// userinfo. Anything else is refused, so the route is no open redirect.
pub(super) fn valid_redirect_uri(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    let loopback = match url.host() {
        Some(Host::Ipv4(ip)) => ip == Ipv4Addr::LOCALHOST,
        Some(Host::Ipv6(ip)) => ip == Ipv6Addr::LOCALHOST,
        _ => false,
    };
    // `url` drops a default port, so an explicit `:80` reads as no port here.
    url.scheme() == "http"
        && loopback
        && url.port().is_some_and(|port| port >= 1024)
        && url.path() == "/cb"
        && url.query().is_none()
        && url.fragment().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

/// The launcher's `state`: 16 to 128 characters of `[A-Za-z0-9_-]`.
pub(super) fn valid_state(raw: &str) -> bool {
    (16..=128).contains(&raw.len()) && raw.bytes().all(is_b64url)
}

/// A PKCE challenge: exactly 43 base64url characters (S256 of 32 bytes).
pub(super) fn valid_challenge(raw: &str) -> bool {
    raw.len() == 43 && raw.bytes().all(is_b64url)
}

/// A dev-mock subject: 1 to 64 characters of `[A-Za-z0-9_.:-]`.
fn valid_subject(raw: &str) -> bool {
    (1..=64).contains(&raw.len()) && raw.bytes().all(|b| is_b64url(b) || b == b'.' || b == b':')
}

/// A plain address check for the dev-mock form: one `@`, a dotted domain,
/// no whitespace, at most 254 characters.
fn valid_email(raw: &str) -> bool {
    let Some((local, domain)) = raw.split_once('@') else {
        return false;
    };
    raw.len() <= 254
        && !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !raw.contains(char::is_whitespace)
}

fn is_b64url(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Escapes the five HTML metacharacters.
fn escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
#[path = "signin_tests.rs"]
mod tests;
