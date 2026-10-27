# client-tls-oidc wave 3: account routes, server wiring, relay and launcher engine

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 3
> **Last updated:** 2026-10-25
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract](contract.md), [wave 2](packets-wave2.md)

Six packets, one PR from `client-tls-oidc/wave3` (worktree `cto-w3`), based on main after wave 2 merged. Merge order: CT-30, CT-31, CT-32, CT-33, CT-34, CT-35. Start CT-30 and CT-32 at once from main. CT-31 branches at CT-30's reviewed head. CT-33 branches at CT-32's reviewed head, CT-34 at CT-33's, CT-35 at CT-34's (one chain in the desktop workspace). Before the wave PR opens, one read-only `network-security-auth` pass reviews the integration head. Every packet carries R1-R10.

Desktop-workspace packets (CT-32..CT-35) use checks D1-D3, never S1-S2. The desktop workspace has its own `Cargo.lock`: after adding a dependency, run D2 once without `--locked` (`pwsh tools/build-lane/lane.ps1 cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml -p <pkg> --all-targets -- -D warnings`) so the lock file updates, commit `crates/launcher/desktop/Cargo.lock`, then run D2 and D3 with `--locked`. If `crates/launcher/desktop/deny.toml` exists, run the desktop `cargo deny` step that [pre-pr-checks.md](../../agents/pre-pr-checks.md) names.

## CT-30 Routes: account, identities, link/start, launch tickets

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-30-routes-account` · **Worktree:** `cto-30`
- **Commit:** `feat(identity): account, unlink, add-method and launch-ticket routes (CT-30)`
- **Rules:** R1-R9. Contract: C1, C6 (`link/start`, `account`, `identities`, `launch-ticket*`), C11, C12, C15 (e).

Files:

1. `crates/identity/src/routes/bearer.rs`: an axum extractor `Bearer(AccessGrant)` reading `Authorization: Bearer <t>` and `ctx.stores.access.check(t, now)`; missing or unknown gives `InvalidToken`.
2. `stores.rs`: `IntentStore` (`issue(LinkIntent, now) -> String`, `take(token, now) -> Option<LinkIntent>`, 10 min, single use) added to `Stores`.
3. `routes/signin.rs` `authorize`: replace the "`intent` present gives `InvalidRequest`" line with `stores.intents.take(intent)` (unknown gives `InvalidRequest`) stored into `PendingAuth.intent`.
4. `routes/account.rs`:
   - `POST /link/start`: Bearer; validate the body like `authorize` (reuse its validators); `intents.issue(LinkIntent { account_id })`; `200 {"authorize_url": "{public_base_url}/auth/v1/authorize?provider=..&redirect_uri=..&state=..&code_challenge=..&code_challenge_method=S256&intent=.."}` built with `url::Url`.
   - `GET /account`: Bearer; `repo::account::facts` plus name and display (display = the most recently used identity's display name, else the account name); `repo::identity::list_for_account`. C6 shape (no password field: a linked account has none, C15).
   - `DELETE /identities/{identity_id}`: Bearer with `via_code` and `issued_at` within 10 min, else 403 `reauth_required`; `count_methods <= 1` gives 409 `last_method`; `delete` false gives 404; event `unlink`; span `auth.idp.unlink`.
5. `routes/ticket.rs`:
   - `POST /launch-ticket`: Bearer; `launch_id` 16-64 chars `[A-Za-z0-9-]` else `InvalidRequest`; account facts (disabled: 403 `account_disabled`); `deps.launch_tickets.mint(account_id, &account_name, &launch_id, now)`; `200 {"account_name","ticket","expires_in"}` with `Cache-Control: no-store`. Span `auth.launch_ticket.mint` (no ticket field).
   - `POST /launch-ticket/heartbeat`: `heartbeat(account_id, launch_id)`; false gives 404 `not_found`.
   - `POST /launch-ticket/end`: `end(..)`; always 204. Span `auth.launch_ticket.end`.
6. `routes/mod.rs`: mount the six routes. `launch-ticket*` stay mounted under the `CIMMERIA_IDP_DISABLED` kill switch (C4).

Tests (`routes/account_tests_live_db.rs`, `live_db`, sentinel accounts `2_000_601..=2_000_615`, subjects `ct30-<n>`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_account_lists_identities_without_subjects` | body has the identity rows and no `subject` or `email` key | R7 |
| `live_db_unlink_needs_a_fresh_code_token` | a refresh-minted token: 403 `reauth_required`; a code token: 204 | stolen refresh tokens cannot unlink |
| `live_db_unlink_refuses_the_last_method` | one identity: 409 `last_method`; two identities: the first unlink 204, the second 409 | C15 (e): a linked account keeps one way in |
| `live_db_link_start_attaches_a_second_identity` | link/start, authorize with its intent, callback, token: the account now lists two identities | add-method flow |
| `live_db_launch_ticket_logs_in_through_userauth` | mint via the route; `deps.launch_tickets.verify(name, SHA1HEX(ticket))` is `Valid` | the route and C1 agree |
| `live_db_launch_ticket_refused_for_disabled_account` | 403 | C6 |
| `live_db_heartbeat_and_end` | heartbeat 204, wrong id 404, end 204 then heartbeat 404 | C6 |
| `launch_ticket_response_is_no_store` | header present | tickets must not be cached |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`, S3 `account_tests_live_db`.

## CT-31 Server wiring, telemetry, deployment env, end-to-end smoke

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-31-wiring` · **Worktree:** `cto-31` (at CT-30's reviewed head)
- **Commit:** `feat(server): mount the identity routes; spans and deployment env (CT-31)`
- **Rules:** R1-R10. Contract: C2, C4, C11, C12.

Files:

1. `crates/identity/src/telemetry.rs`: one function per C11 span returning a `tracing::Span` with the C11 fields declared `Empty`, plus `fn finish(span, outcome, reason, started: Instant)`. Move the inline spans from CT-24..CT-30 to these helpers. A unit test asserts the field list of every helper (`span.metadata().fields()`) contains none of `code`, `token`, `ticket`, `subject`, `email`, `ip`, `display_name`.
2. `crates/identity/src/wiring.rs`: `pub fn install(auth: &mut cimmeria_auth::auth::AuthService, developer_mode: bool) -> Result<(), ConfigError>`: path from `CIMMERIA_AUTH_PROVIDERS` or `config/auth-providers.toml`; `config::load_file`; log each warning once (`WARN reason=.. provider=..`); `providers::build(&cfg, http::provider_client())`; build `IdentityCtx` with `link_code::shared()`; `auth.set_identity_routes(Box::new(move |deps| routes::router(deps, ctx)))`; spawn a reaper (every 60 s: `stores.reap`, `link_code::shared().reap`, span `auth.launch_ticket.reap` is the auth crate's, not here). Log one `INFO` line: provider count, `signup_open`, kill-switch states; never ids or secrets.
3. `crates/server/Cargo.toml`: `cimmeria-identity`. `crates/server/src/main.rs`: `let developer_mode = config.developer_mode;` before `Orchestrator::new(config)` (l.209), then next to the `set_public_routes` block (l.219-223), in the same `write().await` guard: `if let Err(e) = cimmeria_identity::wiring::install(&mut guard.auth, developer_mode) { tracing::error!(field = e.field(), "auth provider config refused"); std::process::exit(1); }`. Put the public-routes block and this install call together in a new `crates/server/src/login_routes.rs` (`pub(crate) async fn install(orch: &Orchestrator, developer_mode: bool)`), so `main.rs` keeps one call line. Add six rows to the env table in `main.rs`'s module doc (after the `AUTH_*` rows CT-22 added): `CIMMERIA_AUTH_PROVIDERS` (default `config/auth-providers.toml`; a refused file stops the server), `CIMMERIA_IDP_DISCORD_SECRET` and `CIMMERIA_IDP_GOOGLE_SECRET` (examples of the `CIMMERIA_IDP_<ID>_SECRET` pattern), `CIMMERIA_IDP_DISABLED`, `CIMMERIA_IDP_SIGNUP_DISABLED`, `CIMMERIA_SOAP_RELAY_DISABLED` (C4 meanings, one sentence each). `main.rs` may end a few lines over 500 (soft cap); never past 700.
4. `docker/.env.example`: commented lines for `CIMMERIA_AUTH_PROVIDERS`, `CIMMERIA_IDP_DISCORD_SECRET`, `CIMMERIA_IDP_GOOGLE_SECRET`, `CIMMERIA_IDP_DISABLED`, `CIMMERIA_IDP_SIGNUP_DISABLED`, `CIMMERIA_SOAP_RELAY_DISABLED`, each with a one-line comment. `docker/compose.yml`: pass those variables through to the server service the way the file passes the Discord ones, and mount `../config/auth-providers.toml` read-only at the path `CIMMERIA_AUTH_PROVIDERS` names, only if the file exists (compose cannot do "if exists": instead document in `.env.example` that the operator sets `CIMMERIA_AUTH_PROVIDERS` to a path inside an already-mounted config folder; read how `compose.discord-file.yml` handles its file and copy that pattern into a new `docker/compose.auth.yml` overlay).
5. `crates/server/src/logging/filters.rs`: confirm the `cimmeria_identity` row from CT-11 exports the spans at INFO; adjust the server's target-scan catalog if a test requires each target to be catalogued (run S2 `cimmeria-server` and fix what it names).
6. New `crates/identity/src/wiring_smoke_live_db.rs` (`#[cfg(test)]`): one end-to-end test `live_db_dev_mock_sign_in_reaches_userauth`: a dev-mode `AuthService` with `logon_port 0`, `auth_proxy_addr 127.0.0.1:0`, a DB pool, `install` with a temp config file holding one `dev_mock` provider (`CIMMERIA_AUTH_PROVIDERS` pointed at it through a test-only `install_with(path, ..)` variant, not a process env write). Through the proxy listener with `X-Forwarded-Proto: https` and `X-Forwarded-For: 203.0.113.7`: `GET providers`, `GET authorize`, `POST dev-mock/authorize`, follow to `callback`, `POST token` (needs_link), `POST link/new` (signed_in), `POST launch-ticket`, then `POST /SGWLogin/UserAuth` with the ticket's SHA-1 hex: the success XML. Then the same UserAuth on the plain listener: error 4. Clean up the created account by its identity subject.

Tests: the smoke above and the telemetry field test (step 1). Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`, S1 `cimmeria-server`, S2 `cimmeria-server`, S3 `wiring_smoke_live_db`, S4.

## CT-32 Launcher relay crate and OAuth callback listener

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-32-relay` · **Worktree:** `cto-32`
- **Commit:** `feat(launcher): loopback SOAP relay and sign-in callback listener (CT-32)`
- **Rules:** R1-R9. Contract: [C9](contract.md#c9-launcher-relay-and-sign-in-desktop-workspace). D-MR6 (decided 2026-10-24: the launch ticket is the tunnel's session identity): the relay is the same process the Mercury forwarder will join; leave `RelayConfig.session` as the reserved `Option<Infallible>` for mercury-replacement to fill.

Files:

1. `crates/launcher/desktop/Cargo.toml`: `members` gains `"relay"`; `[workspace.dependencies]` gains `hyper = { version = "1", features = ["server", "http1"] }`, `hyper-util = { version = "0.1", features = ["tokio"] }`, `http-body-util = "0.1"`, `url = "2"` if missing.
2. `crates/launcher/desktop/relay/Cargo.toml`: package `cimmeria-launcher-relay`, `publish = false`, lints as `engine/Cargo.toml`; deps `tokio`, `hyper`, `hyper-util`, `http-body-util`, `reqwest` (workspace), `url`, `thiserror`, `tracing`, `base64`, `rand` if the workspace has it (else `getrandom`); dev `wiremock`, `tokio` with `macros`.
3. `relay/src/lib.rs`: C9 `RelayConfig`, `RelayHandle`, `RelayError`, `start`. One accept loop on `127.0.0.1:0` with `hyper::server::conn::http1`; per request apply the C9 rules in this order: path and method (404), `Host` (421), `Origin` (403), body size (413, read with `http_body_util::Limited`). Forward to `{upstream}{path}`; the upstream client is built once: `reqwest::Client::builder().use_rustls_tls().min_tls_version(Version::TLS_1_2).local_address(IpAddr::V4(UNSPECIFIED)).timeout(20 s).redirect(none)`, plus `add_root_certificate` for `extra_root_pem` only when `cfg!(debug_assertions)`. `upstream` must be `https` with no path, query or userinfo; `#[cfg(test)]` also allows `http://127.0.0.1:<port>` so tests can use `wiremock`. Response cap 256 KiB (502 past it). Counters (`AtomicU64`: requests, rejected, upstream_errors) for the `launcher.relay.stopped` event. `shutdown` stops accepting and awaits in-flight requests for at most 2 s.
4. `relay/src/callback.rs`: C9 `CallbackServer`. Serves one `GET /cb?code=..&state=..` (or `?error=..&state=..`): compares `state` in constant time (`subtle` or a manual XOR fold), answers a static HTML page ("You can close this tab and return to the Cimmeria launcher.", or "Sign-in was cancelled." for an error) with `Cache-Control: no-store`, and returns `Code`, `Denied` (any `error`) or `StateMismatch`; other paths 404 without ending the wait; `TimedOut` after `timeout`. Binds `127.0.0.1:0` only.
5. `relay/src/tests.rs` (`#[cfg(test)]`).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `relays_userauth_and_returns_status_body_and_cookie` | wiremock upstream sees the same body, `Content-Type`, `SOAPAction`; the reply's `Set-Cookie` comes back | C9 |
| `refuses_other_paths_hosts_origins_and_big_bodies` | `GET /SGWLogin/UserAuth` 404, `/x` 404, `Host: evil.example` 421, `Origin: https://example.org` 403, 65 KiB 413 | DNS-rebinding and browser-CSRF defence |
| `upstream_failure_is_502_with_empty_body` | upstream down: 502, empty body | C9 |
| `refuses_non_https_upstream_outside_tests` | `start` with `http://example.org` gives `RelayError::Upstream` (call the validator directly with the test exemption off) | TLS is the point |
| `binds_loopback_only` | `soap_url()` starts with `http://127.0.0.1:` | C9 |
| `callback_returns_the_code_for_the_right_state` | `Code("abc")` | new |
| `callback_state_mismatch_and_denied_and_timeout` | each variant | new |
| `relay_logs_no_body` | capture logs with a `tracing` test subscriber; a body containing `SECRETMARKER` never appears | R7 |

Checks: D1, D2 `cimmeria-launcher-relay`, D3 `cimmeria-launcher-relay` (no filter).

## CT-33 Engine sign-in: discovery and the browser flow

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-33-engine-discovery` · **Worktree:** `cto-33` (at CT-32's reviewed head)
- **Commit:** `feat(launcher): discover server sign-in and run the browser flow (CT-33)`
- **Rules:** R1-R9. Contract: C6 (`providers`, `authorize`), C9, D-CT21.

Files:

1. `crates/launcher/desktop/engine/Cargo.toml`: `cimmeria-launcher-relay = { path = "../relay" }`, `url`.
2. New `engine/src/sign_in/mod.rs`: module doc (what sign-in is, D-CT21: only the first login server in the list is offered sign-in, a link to the ledger); `pub mod discovery; pub mod oauth;` and `pub enum SignInError { Offline, Unsupported, Cancelled, Denied, TimedOut, StateMismatch, Server(String code), Protocol }` with `reason() -> &'static str`.
3. `sign_in/discovery.rs`: C9. `pub fn auth_base(login_url: &str) -> Option<Url>`: the login server URL's host with scheme `https` and no port, path or query (a login URL of `http://play.example.org:8081` gives `https://play.example.org`); an IP-literal host gives `None` (no certificate); the two debug env overrides only under `cfg!(debug_assertions)`. `pub async fn probe(http: &reqwest::Client, login_url: &str) -> Option<Discovery>` (4 s; `api` must be `1`; any error is `None` and one `debug!` line with a reason). `Discovery { auth_base, soap_relay, signup_open, providers: Vec<ProviderView { id, display_name }>, links: Links }` with `Links` from `cimmeria_launcher_core::client_setup::login_session` (CT-03). `DiscoveryCache` (10 min per login URL, `Instant` passed in). The one sign-in HTTP client (ruling CT-33-ruling-1 D2): `pub fn client() -> reqwest::Client` (rustls, TLS 1.2 minimum, `redirect::Policy::none()`, 10 s), `pub fn extra_root_pem() -> Option<Vec<u8>>` (debug builds only: the bytes of the PEM file `CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM` names; `client()` adds it as a root), `pub(crate) async fn read_capped(resp, cap) -> Option<Vec<u8>>` (replies capped at `MAX_BODY` = 64 KiB).
4. `sign_in/oauth.rs`: `Pkce { verifier, challenge }` and `fn new_pkce()`, `fn new_state()` per C9; `pub struct BrowserFlow { pub authorize_url: String, wait: .. }`. `pub async fn prepare() -> Result<PendingFlow, SignInError>` binds a `CallbackServer` and makes the PKCE pair and `state`; `PendingFlow` exposes `redirect_uri()`, `state()`, `code_challenge()` (for `link/start`) and finishes with `authorize(self, base, provider)` (builds `{base}auth/v1/authorize?..` with `url::Url`) or `with_intent(self, base, authorize_url)` (the `link/start` reply, after checking it starts with `{base}auth/v1/authorize?`). `pub async fn begin(base: &Url, provider: &str) -> Result<BrowserFlow, SignInError>` is `prepare` then `authorize`. `BrowserFlow::finish(self) -> Result<CodeGrant { code, verifier, redirect_uri }, SignInError>` waits up to 5 min and maps `CallbackResult`.
5. `engine/src/lib.rs`: `pub mod sign_in;`.

Tests (`sign_in/tests.rs`; `wiremock` for `/providers`; the browser is simulated by a `reqwest` GET to the callback URL):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `auth_base_from_login_urls` | `http://play.example.org:8081` gives `https://play.example.org/`; `http://203.0.113.5:8081` gives `None`; `http://localhost:8081` gives `https://localhost/` | D-CT4: auth lives on 443 of the same host |
| `probe_parses_providers_and_links` | C6 shape | new |
| `probe_refuses_unknown_api_and_errors` | `api: 2` and 500: `None` | forward compatibility |
| `pkce_challenge_matches_rfc7636` | the RFC vector | S256 |
| `browser_flow_round_trip` | `begin`; GET `{redirect_uri}?code=c&state=<state from the authorize URL>`; `finish` gives `code == "c"` and the same `redirect_uri` | new |
| `intent_url_must_point_at_the_same_server` | `with_intent` with `https://evil.example/auth/v1/authorize?..` gives `Protocol` | the launcher only opens its server's pages |
| `intent_flow_round_trip` | `prepare`; an intent URL built from its `redirect_uri`, `state`, `code_challenge`; `with_intent`; GET the redirect; `finish` gives `code == "c"` and a verifier whose challenge is `code_challenge()` | add-method through `link/start` (CT-34) |
| `client_does_not_follow_redirects` | `/providers` 302 to a 200 page: `probe_at(&client(), ..)` is `None` | a hijacked probe cannot redirect the launcher |
| `debug_env_supplies_the_extra_root_pem` | debug: the file's bytes; missing file or unset: `None` (release twin: always `None`) | local servers on a private CA |

Checks: D1, D2 `cimmeria-launcher-engine`, D3 `cimmeria-launcher-engine sign_in`.

## CT-34 Engine sign-in: credential store and session

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-34-engine-session` · **Worktree:** `cto-34` (at CT-33's reviewed head)
- **Commit:** `feat(launcher): sign-in session and OS credential store (CT-34)`
- **Rules:** R1-R9. Contract: C6 (every route), C9 (`store.rs`, `session.rs`).

Files:

1. `crates/launcher/desktop/Cargo.toml` and `engine/Cargo.toml`: `keyring = { version = "3", default-features = false, features = ["windows-native", "apple-native"] }`; on targets other than Windows and macOS the engine uses `MemoryStore` only (`#[cfg]` on the `KeyringStore` type).
2. `sign_in/api.rs`: serde types for every C6 request and response body (`SignedIn`, `NeedsLink`, `AccountView`, `IdentityView`, `TicketView`, `ApiErrorBody`), `#[serde(deny_unknown_fields)]` off (the server may add fields).
3. `sign_in/store.rs`: C9 `CredentialStore` (`get(host) -> Option<Stored>`, `set(host, &Stored)`, `delete(host)`; errors become `None`/logged reason, never the value), `KeyringStore` (service `cimmeria-launcher`, user `<auth host>`), `MemoryStore`. `Stored { refresh_token, account_name, display_name }` with a manual `Debug` that hides `refresh_token`.
4. `sign_in/session.rs`: `pub struct SignIn { base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>, state: Mutex<Inner { access: Option<(String, Instant)>, account: Option<AccountSummary> }> }` with the C9 methods:
   - `complete(CodeGrant) -> Result<Outcome, SignInError>` where `Outcome = SignedIn(AccountSummary) | NeedsLink(NeedsLinkView)`; `link_new(grant)`, `link_existing_password(grant, name, password)`, `link_existing_code(grant, code)`: both send `"password_removal_ack": true` (the player's acknowledgement is checked earlier, by CT-40's command layer; C13, C15 a); on `signed_in` save `Stored` and keep the access token in memory.
   - `access_token()`: the cached token when more than 60 s remain, else `POST /token` `refresh_token` (rotating the stored token); `invalid_grant` deletes the stored entry and gives `Server("signed_out")`.
   - `account()`, `unlink(identity_id)` (`reauth_required` surfaces as `Server("reauth_required")`), `sign_out()` (`POST /logout`, delete the stored entry, clear memory; succeeds offline too).
   - `add_method(&self, provider: &str) -> Result<BrowserFlow, SignInError>` (ruling CT-33-ruling-1 D1): `let pending = oauth::prepare().await?;` first (the listener must exist before `link/start`), then `POST {base}auth/v1/link/start` with the bearer from `access_token()` and body `{"provider", "redirect_uri": pending.redirect_uri(), "state": pending.state(), "code_challenge": pending.code_challenge(), "code_challenge_method": "S256"}`; the `200` body's `authorize_url` goes to `pending.with_intent(&self.base, url)` (its refusal is `Protocol`). Any error drops `pending`, which closes the listener. Never call `oauth::begin` here: its fresh `state` and challenge are not the ones the server echoed.
   - `launch_ticket(launch_id) -> TicketView`, `heartbeat(launch_id) -> bool`, `end_ticket(launch_id)`.
   - `pub fn restore(base, http, store) -> Option<SignIn>` from a stored entry (no network).
   - Every request: 10 s timeout, `Accept: application/json`; status mapping to `SignInError::Server(<error code>)`; a 429 keeps `retry_after` in the error text for the UI.
   - HTTP client (ruling CT-33-ruling-1 D3): `SignIn`'s `http` is always `discovery::client()` (CT-33: rustls, TLS 1.2 minimum, no redirects, 10 s, the debug-only `CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM` root). Never build a `reqwest::Client` in `session.rs`; the tests pass `discovery::client()` too, so the redirect policy under test is the shipped one. Read every reply body with `discovery::read_capped(resp, discovery::MAX_BODY)`; `None` (over 64 KiB or a broken read) is `Protocol`.
   - Events `launcher.sign_in.completed` / `.failed` with `outcome` and `reason` only, through the engine's existing launcher-event helper (find how `launcher.relay`-style events are emitted: grep `launcher_telemetry` in `engine/src`; if there is no event API in the engine, log `info!` with the same names and say so).
5. `sign_in/mod.rs`: `pub mod api; pub mod store; pub mod session;`.

Tests (`sign_in/session_tests.rs`, `wiremock` server standing in for `/auth/v1`, `MemoryStore`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `complete_signed_in_saves_the_refresh_token` | store holds it; `account()` works with the access token | new |
| `needs_link_then_link_new` | new |
| `access_token_refreshes_near_expiry_and_rotates` | `expires_in 30`: next call posts `refresh_token` and stores the new one | C6 rotation |
| `invalid_grant_signs_out` | stored entry deleted, `Server("signed_out")` | revoked families |
| `sign_out_works_offline` | server down: stored entry still deleted | the player can always sign out |
| `stored_debug_hides_the_token` | R7 |
| `launch_ticket_heartbeat_end` | three calls with the bearer and `launch_id` | new |
| `rate_limit_error_carries_retry_after` | 429 body gives an error whose text includes the seconds | feedback on the first press |
| `add_method_sends_the_flow_values_to_link_start` | wiremock `POST /auth/v1/link/start` with `header("authorization", "Bearer <access>")` and a custom `wiremock::Respond` that reads the JSON body and answers `{"authorize_url": "{server}/auth/v1/authorize?provider=..&redirect_uri=<body>&state=<body>&code_challenge=<body>&code_challenge_method=S256&intent=i1"}`; `add_method("dev")`; GET `{redirect_uri}?code=c&state=<state>` from the returned URL; `finish` gives `code == "c"` and `oauth::challenge_for(&grant.verifier)` equals the body's `code_challenge` | with `oauth::begin` instead of `prepare` + `with_intent`, `finish` gives `StateMismatch` (D1) |
| `oversized_reply_is_protocol` | `GET /account` answering 200 with a 70 KiB JSON body gives `Protocol` | the 64 KiB cap |

Fill the two short rows' columns when you write them: asserts "link_new gives SignedIn" / "Debug output lacks the token", fails-on-revert "new" / "R7". Checks: D1, D2 `cimmeria-launcher-engine`, D3 `cimmeria-launcher-engine sign_in`.

## CT-35 Play integration: relay, ticket and scrub

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-35-play` · **Worktree:** `cto-35` (at CT-34's reviewed head)
- **Commit:** `feat(launcher): Play signs in through the relay and a launch ticket (CT-35)`
- **Rules:** R1-R9. Contract: C1, C8, C9 (Play), C15 (d), D-CT8, D-CT11 (no password fallback for a signed-in player), D-CT21.

Files:

1. `engine/src/storage/mod.rs` `DesktopState` (l.218): two fields `sign_in: Option<Arc<crate::sign_in::session::SignIn>>` and `play_refusal: Option<&'static str>`, `None` in every constructor; accessors in a new `engine/src/storage/sign_in_state.rs` (`pub fn sign_in(&self) -> Option<Arc<SignIn>>`, `pub fn set_sign_in(&mut self, Option<Arc<SignIn>>)`, `pub fn play_refusal(&self) -> Option<&'static str>`, `pub fn set_play_refusal(&mut self, Option<&'static str>)`). `mod.rs` is 514 lines: add only the fields, their initializers and the `mod` line.
2. New `engine/src/storage/launch/sign_in_play.rs`:
   - `pub(super) enum ClientLogin { Plain, RelayPassword { relay: RelayHandle }, Ticket { relay: RelayHandle, login: TicketLogin, sign_in: Arc<SignIn>, launch_id: String } }`.
   - `pub(super) async fn plan(sign_in: Option<Arc<SignIn>>, plan: &Plan, servers: &[LoginServer], discovery: Option<Discovery>) -> Result<(Vec<LoginServer>, ClientLogin), TicketRefusal>` with `pub(super) enum TicketRefusal { Unavailable, SignInEnded }` and `fn reason(&self) -> &'static str` (`ticket_unavailable`, `sign_in_ended`). The caller takes `sign_in` out with `state.lock()...sign_in()` in its own statement first: never hold the `DesktopState` lock across an `.await`. D-CT21 applies to `servers[0]` only. First narrow `sign_in` to this Play: keep it only when its `base()` equals `discovery.auth_base`, or, when `discovery` is `None`, `discovery::auth_base(&servers[0].url)` (ruling CT-35-1). When `discovery` is `None` or `soap_relay` is false: a narrowed `sign_in` gives `Err(Unavailable)` (event `ticket_failed`, reason `no_discovery` or `soap_relay_disabled`), else `(servers.to_vec(), Plain)`. Else start a relay for that server (`upstream = discovery.auth_base`, `extra_root_pem: discovery::extra_root_pem()`, `session: None`; a start failure, event reason `relay_start_failed`, gives `Err(Unavailable)` when signed in, else `Plain`). When the narrowed `sign_in` is `Some`: `launch_ticket(launch_id)` (launch id = the plan's operation id as text, padded to 16 chars if shorter, `[A-Za-z0-9-]` only); success gives `Ticket` with a list holding only `servers[0]` re-pointed at `relay.soap_url()`; failure shuts the relay down and gives `Err(SignInEnded)` when the refresh answered `invalid_grant` (`Server("signed_out")`), else `Err(Unavailable)` (offline, `401`, `403`, `5xx`). It **never** falls back to `RelayPassword` (C15 d: a linked account has no password, so the fallback could only fail later inside the game). Otherwise (not signed in, or signed in to another auth base): `RelayPassword`, which keeps every server and re-points only `servers[0]`.
   - `pub(super) fn write(root: &Path, login: &ClientLogin) -> io::Result<()>`: `Ticket` calls `login_session::write_ticket` (CT-03), after `prepare` has written the plain file; the other two write nothing extra.
   - `pub(super) async fn heartbeat_loop(login: &ClientLogin, cancel)`: every 10 min `heartbeat`; three failures in a row stop the loop quietly (the game may already be in the world; a ticket only matters for a re-login).
   - `pub(super) async fn finish(root: &Path, plain: &[LoginServer], login: ClientLogin)`: `end_ticket`, `relay.shutdown()`, then rewrite `LoginInternal.lua` with `login_servers::write(root, plain)` so no ticket stays on disk (scrub); events `launcher.relay.stopped` with counters.
3. `engine/src/storage/launch/worker.rs` `execute` (l.147-233), at most 15 new lines:
   - After `servers_and_session` (l.169), `let discovery = sign_in_play::discover(&servers).await;` (a cached `discovery::probe` over `discovery::client()`; never a `reqwest::Client` of its own; `None` under `cfg!(any(test, feature = "test-support"))` so no unit or shell test probes a real server) and `let sign_in = state.lock().map_err(|_| StorageError::Io)?.sign_in();` then `let (client_servers, login) = sign_in_play::plan(sign_in, plan, &servers, discovery).await` mapped so `Err(r)` ends the launch as `NotStarted` with reason `r.reason()` before `prepare` runs (same shape as `ticket_write_failed` below) and stores `r.reason()` with `state.lock()?.set_play_refusal(Some(..))`; every Play start first calls `set_play_refusal(None)`. (C9; CT-40 reports it, CT-42 shows it).
   - Pass `&client_servers` (not `&servers`) to `prepare` (l.190); `telemetry::open_session` (l.212) keeps `&servers` (C9: telemetry derives its endpoint from the real list).
   - After `prepare` returns `Ok`, `sign_in_play::write(&root, &login)`; a write error ends the launch as `NotStarted` with reason `ticket_write_failed`.
   - Run `heartbeat_loop` alongside `run_game` (`tokio::select!` or a spawned task cancelled when `run_game` returns), then `finish(&root, &servers, login)` on every exit path after the spawn (success, error, cancel).
   - The Mac path (`wine::prepare`) receives the same `client_servers`; the relay listens on host loopback, which Wine reaches (V5 unverified; the report says so).
4. `engine/src/storage/launch/mod.rs`: `mod sign_in_play;`.

Tests (`engine/src/storage/launch/sign_in_play_tests.rs`; `wiremock` for `/auth/v1`; a temp install root with `Working/SGWGame/Content/UI/Startup/Login/`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `no_discovery_keeps_the_plain_list` | `Plain`, list unchanged | legacy servers and self-hosters |
| `relay_password_mode_repoints_only_the_first_server` | two servers: first is `http://127.0.0.1:<p>`, second unchanged | D-CT21 |
| `signed_in_play_writes_a_ticket_file_with_one_server` | the file has `CimmeriaLogin.mode = "ticket"`, one `LoginMod.servers` line pointing at the relay | C8 |
| `ticket_failure_ends_the_launch_with_an_error` | signed in, `/launch-ticket` 401 and then 503: `Err(Unavailable)`, never `RelayPassword` (the relay is shut down inside `plan`; its port is not observable from the test) | D-CT11 (d): no password fallback |
| `signed_in_without_discovery_is_refused` | discovery `None`, signed in to `auth_base(servers[0].url)`: `Err(Unavailable)`; signed in to another host: `Plain`, list unchanged | C15 (d): a failed probe is no password fallback (ruling CT-35-1) |
| `signed_in_with_the_relay_disabled_is_refused` | discovery with `soap_relay: false`, signed in to its base: `Err(Unavailable)` | C15 (d) (ruling CT-35-1) |
| `signed_in_relay_start_failure_is_refused` | discovery at `http://play.example.org/` (the relay refuses that upstream): signed in gives `Err(Unavailable)`; not signed in gives `Plain` | C15 (d) (ruling CT-35-1) |
| `signed_out_refresh_is_sign_in_ended` | the refresh answers `invalid_grant`: `Err(SignInEnded)` | the launcher tells the player to sign in again |
| `play_refusal_accessors_round_trip` | `sign_in_state` directly: set and clear | accessors |
| `a_signed_in_play_without_a_ticket_records_the_refusal` (`worker_tests.rs`) | over `execute`: signed in, no discovery: `NotStarted`, `play_refusal()` is `ticket_unavailable`, failure `LaunchPrepareFailed`, no `play_client_setup` phase | the panel can name why Play stopped |
| `a_play_start_clears_the_last_refusal` (`worker_tests.rs`) | over `execute`: a stored refusal is `None` after a Play that ends `prepare_failed` | every Play start clears it |
| `not_signed_in_uses_relay_password` | covered by `relay_password_mode_repoints_only_the_first_server` (`sign_in = None`, discovery with `soap_relay`); no separate test | seed and other non-OIDC accounts still play |
| `finish_scrubs_the_ticket_and_stops_the_relay` | after `finish`, the file equals `login_servers::render(plain)` and the relay port refuses connections | no ticket left on disk |
| `telemetry_keeps_the_real_servers` | a unit test over the `execute` seam: `telemetry::session` is called with the original list (extract the list choice into a function `lists_for(servers, &login) -> (&[LoginServer] telemetry, Vec<LoginServer> client)` and test it) | the relay must never become the telemetry endpoint |

Checks: D1, D2 `cimmeria-launcher-engine`, D3 `cimmeria-launcher-engine sign_in_play`, D3 `cimmeria-launcher-engine worker` (existing worker tests still pass).
