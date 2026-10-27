# client-tls-oidc wave 2: launch tickets, listeners and sign-in routes

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 2
> **Last updated:** 2026-10-24
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract](contract.md), [wave 1](packets-wave1.md)

Seven packets, one PR from `client-tls-oidc/wave2` (worktree `cto-w2`), based on main after wave 1 merged. Merge order: CT-20, CT-21, CT-22, CT-23, CT-24, CT-25, CT-26. Start CT-20 and CT-23 at once from main. CT-21 branches at CT-20's reviewed head, CT-22 at CT-21's (all three touch `crates/auth`). CT-24 branches at the wave head after CT-22 and CT-23 merged in; CT-25 at CT-24's reviewed head; CT-26 at CT-25's. Before the wave PR opens, one read-only `network-security-auth` pass reviews the integration head (see the dispatch rules). Every packet carries R1-R10.

## CT-20 Launch-ticket store

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-20-ticket-store` · **Worktree:** `cto-20`
- **Commit:** `feat(auth): per-launch ticket store (CT-20)`
- **Rules:** R1-R9. Contract: [C1](contract.md#c1-launch-ticket-server-cratesauthsrcauthlaunch_ticketrs), C2 (`LoginOutcome`).

Files:

1. New `crates/auth/src/auth/launch_ticket.rs`: C1 exactly. `mint` draws the ticket with `rand::rng()` and `rand::distr::Alphanumeric` (workspace `rand` 0.10), stores `Sha256(sha1_upper_hex(ticket))`, and returns the plain ticket only in `MintedTicket`. Add a private `fn sha1_upper_hex(&str) -> String` (the same as `credentials.rs` l.229; do not make that one public). `verify` uppercases the presented hex, hashes it, compares with `subtle::ConstantTimeEq`, checks the name ASCII case-insensitively and both lifetimes; on `Valid` it sets `last_used = now`; an expired entry is removed and gives `NoMatch`. `heartbeat` and `end` need the matching `launch_id`. `reap` removes expired entries and returns the count. `Entry` derives no `Debug` that prints the hash; give `LaunchTicketStore` a manual `Debug` that prints `live_count` only.
2. `crates/auth/src/auth/mod.rs`: `pub mod launch_ticket;` and `pub use launch_ticket::{LaunchTicketStore, MintedTicket, TicketCheck};`.
3. `crates/auth/src/audit.rs` `LoginOutcome` (l.28) and its `as_str` (l.49-61): `LaunchTicket` = `"launch_ticket"`, `TicketRequiresTls` = `"ticket_requires_tls"`. Extend any test that lists every outcome.
4. `crates/auth/Cargo.toml`: `subtle = "2"` (add to root `[workspace.dependencies]` first if the workspace pins it there; follow how `sha2` is declared), `sha2`, `rand` if missing. Then S4.

Tests (`launch_ticket.rs` `mod tests`, with `Instant` arithmetic, no sleeps):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `minted_ticket_shape` | 32 chars, all ASCII alphanumeric; two mints differ | C1 |
| `verify_accepts_the_sha1_of_the_ticket_any_case` | uppercase and lowercase hex both `Valid`; name `PLAYER_1234` matches `Player_1234` | the client sends uppercase hex; names are case-insensitive in `account` |
| `verify_refuses_wrong_name_wrong_hash_and_plain_ticket` | each `NoMatch` | new |
| `ticket_is_reusable_within_its_life` | three `verify` calls inside 30 min idle each `Valid` (Change Server, F14) | a one-time ticket breaks Change Server |
| `idle_and_absolute_expiry` | 31 min idle: `NoMatch`; heartbeats every 20 min keep it alive until 12 h, then `NoMatch` | C1 lifetimes |
| `mint_replaces_the_previous_ticket` | old ticket `NoMatch` after a second mint | one ticket per account |
| `heartbeat_and_end_need_the_launch_id` | wrong id: `false` and the ticket survives | new |
| `store_never_holds_the_ticket` | `format!("{:?}", store)` lacks the ticket and its SHA-1 | R7 |
| `reap_counts_expired` | 2 expired, 1 live: `reap` returns 2, `live_count` 1 | new |

Checks: S1 `cimmeria-auth`, S2 `cimmeria-auth`, S4.

## CT-21 UserAuth accepts launch tickets; password check API; name fix

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-21-userauth-ticket` · **Worktree:** `cto-21` (at CT-20's reviewed head)
- **Commit:** `feat(auth): accept launch tickets over TLS in UserAuth (CT-21)`
- **Rules:** R1-R9. Contract: C1, C2, D-CT7.

Files:

1. `crates/auth/src/auth/service.rs`: field `launch_tickets: Arc<LaunchTicketStore>` (built in `new`), `pub fn launch_tickets(&self) -> Arc<LaunchTicketStore>`, passed into `HandlerState` (l.166). In the reaper task (l.203-221) also call `launch_tickets.reap(now)` and add the count to the debug line. Keep `service.rs` at or under its current 565 lines plus 12; CT-22 splits its tests.
2. `HandlerState` (find its definition under `crates/auth/src/auth/`): field `launch_tickets: Arc<LaunchTicketStore>`.
3. New `crates/auth/src/auth/ticket_login.rs`: `pub(super) async fn ticket_account(db: Option<&PgPool>, developer_mode: bool, db_configured: bool, account_id: u32) -> Result<(u32, u32, bool), AuthCredError>` reading `enabled, accesslevel, test_account` for the id (disabled gives `AccountDisabled`; missing row `InvalidCredentials`; the developer no-DB case gives `(account_id, 99, false)` only when `developer_mode && !db_configured`).
4. `handlers.rs` `handle_user_auth`, between the protocol-digest check (ends l.161) and the credential check (l.163), when `credential` is the legacy SHA-1 variant: `match state.launch_tickets.verify(&req.account_name, &req.password, Instant::now())`:
   - `Valid { account_id }` and `!over_tls`: `state.launch_tickets.revoke_account(account_id)`, `audit!(LoginOutcome::TicketRequiresTls, id = account_id)`, `tracing::warn!(account_id, reason = "ticket_requires_tls", ...)`, return `login_error(4, "The account name or password is incorrect.")`.
   - `Valid { account_id }` over TLS: `ticket_account(..)`; on `Ok` use its triple in place of `validate_credentials`'s, `audit!(LoginOutcome::LaunchTicket, id = account_id)`, record span `result` later as today; on `AccountDisabled` revoke and return code 5 as today; on `DbError` code 10.
   - `NoMatch`: fall through to the existing check unchanged (an account's real password hash still works).
   Restructure only as much as needed: compute `(account_id, access_level, test_account)` from either path into the same `let`. `handlers.rs` is 453 lines; stay under 500.
5. Name check (l.122): `c.is_ascii_alphanumeric()` in place of `c.is_alphanumeric()`. Comment: the client and `account_name` allow ASCII only; a Unicode letter reached the audit row before.
6. New `crates/auth/src/auth/password_check.rs`: `pub enum PasswordCheck { Ok { account_id: u32 }, Invalid, Disabled, DbError }` and `pub async fn verify_account_password(db: &PgPool, account_name: &str, plaintext: &str) -> PasswordCheck`, which reads the row's `password_algo` first: algo 1 (legacy) calls `validate_credentials` with `ClientCredential::LegacySha1Hex(&<uppercase SHA-1 hex of plaintext>)` (`credentials.rs` `sha1_upper_hex`, made `pub(super)`), so the row is **not** migrated; algo 2 calls it with `ClientCredential::Plaintext`. Reason: it is a pure check. When the link that follows is refused (`not_linkable`, a staff or test account), a migration to algo 2 (argon2id of the plaintext) would lock that account out of the unmodified client, which only sends `SHA1HEX` (D-CT7). When the link succeeds, CT-26 clears the password anyway (C15 a). CT-54 later moves the remaining non-OIDC rows to argon2id(SHA1HEX). Re-export from `mod.rs`. CT-26's `link/existing` uses it.
7. New `crates/auth/src/auth/handlers_ticket_tests.rs`, registered `#[cfg(test)] mod handlers_ticket_tests;` in `mod.rs`. Build requests the way `handlers_tests.rs` does (read it first; reuse its helpers through `super::handlers_tests::` if they are `pub(super)`, else copy the minimum).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `ticket_over_tls_logs_in` (live-DB, sentinel `2_000_401`) | a minted ticket's SHA-1 with `TlsConn` gives the success XML with a SID; `login_audit` gets `launch_ticket` | step 4 |
| `ticket_over_plain_http_is_refused_and_revoked` (live-DB) | error 4; a second try over TLS also fails | the replay guard |
| `ticket_for_a_disabled_account_is_code_5` (live-DB) | error 5 and the ticket revoked | disabled accounts |
| `unmodified_client_sha1_over_http_still_logs_in` (live-DB) | a legacy account's real SHA-1 hex over plain HTTP gives the success XML | D-CT7: the legacy path stays |
| `passwordless_account_refuses_every_password` (live-DB) | an account with `password` and `password_hash_v2` NULL and algo 1: SHA-1 of `""` and of `"x"` both give error 4, no panic | OIDC-only accounts (CT-15 creates them) |
| `unicode_account_name_is_code_1` | `"Jos\u{e9}"` gives error 1 before any DB access | step 5 |
| `verify_account_password_keeps_legacy_rows` (live-DB) | `Ok` for the right plaintext; the row still has algo 1 and the same `password`; the unmodified client's SHA-1 hex still logs in afterwards; `Invalid` for a wrong one | step 6; D-CT7 |

Name every live-DB test with `live_db` in it. Checks: S1 `cimmeria-auth`, S2 `cimmeria-auth`, S3 `handlers_ticket_tests`, S3 `credentials` (the existing credential tests still pass).

## CT-22 Proxy listener, identity route hook, `require_tls`

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-22-proxy-hook` · **Worktree:** `cto-22` (at CT-21's reviewed head)
- **Commit:** `feat(auth): reverse-proxy listener and /auth/v1 route hook (CT-22)`
- **Rules:** R1-R9. Contract: C2.

Files:

1. Split first, as its own step in the same commit: move `service.rs`'s `#[cfg(test)] mod tests` (l.440 to the end) into new `crates/auth/src/auth/service_tests.rs` (`#[cfg(test)] #[path = "service_tests.rs"] mod tests;` in `service.rs`). No test changes.
2. `crates/common/src/config.rs`: `pub auth_proxy_addr: Option<SocketAddr>` next to `auth_tls_cert_path` (l.35), default `None`, documented. Fix every struct literal the compiler finds.
3. `crates/server/src/main.rs` `config_from_env` (l.424): read `AUTH_PROXY_ADDR` (parse failure: log `WARN reason=bad_auth_proxy_addr` and leave `None`). On `2019922fe` nothing reads `AUTH_TLS_CERT_PATH` or `AUTH_TLS_KEY_PATH` from the environment (grep to confirm), so read both here too, into `auth_tls_cert_path` and `auth_tls_key_path`, and add the three rows to the env table in `main.rs`'s module doc (next to `AUTH_TLS_RELOAD_INTERVAL_SECS`, l.16). `main.rs` is 495 lines: move the auth block of `config_from_env` into a new `crates/server/src/config_env_auth.rs` (`pub(crate) fn apply(cfg: &mut ServerConfig)`) so `main.rs` does not grow past 500.
4. New `crates/auth/src/auth/proxy.rs`: `pub(super) async fn proxy_layer(req, next)` per C2 steps 1-3 (rightmost `X-Forwarded-For` entry, trimmed; IPv6 accepted; replaces the `ConnectInfo<SocketAddr>` extension with `SocketAddr::new(ip, 0)`; inserts `TlsConn`); `400` bodies are empty. `pub(super) async fn require_tls(req, next)`: `403` JSON `{"error":"https_required"}` without `TlsConn`.
5. New `crates/auth/src/auth/identity_hook.rs`: `AuthRouteDeps` and `IdentityRoutes` (C2), `impl AuthService { pub fn set_identity_routes(&mut self, routes: IdentityRoutes) }` (a field `identity_routes: Option<IdentityRoutes>` in `service.rs`). Re-export both types from `mod.rs` and `lib.rs`.
6. `service.rs` `start()`: after building `app` (l.181-189), when `identity_routes` is set, call it once with `AuthRouteDeps { db: self.db_pool.clone(), launch_tickets, developer_mode }`, wrap the router with `require_tls`, and `nest("/auth/v1", ..)` into `app` before the HSTS layer. Then, when `auth_proxy_addr` is set, bind a `TcpListener` on it and serve `app.clone().layer(from_fn(proxy_layer))` with `into_make_service_with_connect_info::<SocketAddr>()`, logging `INFO "Auth proxy listener bound"` with the address. The plain and TLS listeners are unchanged, so `X-Forwarded-*` means nothing on them. `start()` may need to move into a helper function in a new `service_listeners.rs` to keep `service.rs` under 500 lines; do so when it passes 500.
7. `docker/.env.example` (or the file the compose stack reads env from; grep `AUTH_TLS_CERT_PATH` under `docker/`): add a commented `# AUTH_PROXY_ADDR=0.0.0.0:13480` line with one comment line saying it is for a TLS-terminating reverse proxy only and must never be published to the internet.

Tests (`proxy.rs` `mod tests` with `tower::ServiceExt::oneshot`; `service_tests.rs` for the listener):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `proxy_requires_https_proto` | no header or `http`: 400 | step 4 |
| `proxy_takes_the_rightmost_forwarded_for` | `203.0.113.9, 198.51.100.7` gives `198.51.100.7` in `ConnectInfo` and `TlsConn` present | spoofing the leftmost entry |
| `proxy_refuses_a_bad_forwarded_for` | `nonsense` and missing: 400 | new |
| `require_tls_refuses_plain` | without `TlsConn`: 403 `https_required` | C2 |
| `identity_routes_mount_under_auth_v1` | a test router with `GET /ping`: plain listener 403, proxy listener with headers 200 (bind both on port 0) | the hook is new |
| `forwarded_headers_mean_nothing_on_the_plain_listener` | plain listener, `X-Forwarded-Proto: https` on `/auth/v1/ping`: still 403 | trust comes from the listener |
| `userauth_over_proxy_counts_as_tls` | a plaintext password via the proxy listener is classified as TLS (dev-mode no-DB service: success XML, not error 2) | the proxy must carry `TlsConn` to the SOAP routes |

Checks: S1 `cimmeria-auth`, S2 `cimmeria-auth`, S1 `cimmeria-common`, S2 `cimmeria-server` (filter `config`).

## CT-23 In-memory sign-in stores and link codes

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-23-stores` · **Worktree:** `cto-23`
- **Commit:** `feat(identity): pending sign-in, handoff, access-token and link-code stores (CT-23)`
- **Rules:** R1-R9. Contract: C6 (pending records, handoff code, tokens, `link_grant`, `link_code`), C10, C12.

Files:

1. `crates/identity/src/stores.rs`, every map behind one `Mutex`, every method taking `now: Instant`:
   - `PendingStore`: `insert(rec: PendingAuth, now) -> String server_state` (server state = 32 random bytes base64url); `take(server_state, now) -> Option<PendingAuth>` (10 min life; single use); cap 10,000, oldest dropped. `PendingAuth { provider: String, redirect_uri: String, launcher_state: String, launcher_challenge: String, pkce_verifier: String, nonce: String, intent: Option<LinkIntent> }`; `LinkIntent { account_id: u32 }`.
   - `HandoffStore`: `issue(outcome: HandoffOutcome, challenge, redirect_uri, now) -> String code`; `redeem(code, verifier, redirect_uri, now) -> Option<HandoffOutcome>` (60 s, single use, `BASE64URL(SHA256(verifier)) == challenge` compared in constant time, the same `redirect_uri`; a failed check still consumes the code). `HandoffOutcome { identity: VerifiedIdentity, intent: Option<LinkIntent> }`.
   - `AccessTokens`: `mint(account_id, family_id, via_code: bool, now) -> String`; `check(token, now) -> Option<AccessGrant { account_id, family_id, via_code, issued_at }>` (15 min); `revoke_family(family_id)`; `revoke_account(account_id)`. Stored as SHA-256 keys.
   - `LinkGrants`: `issue(identity, now) -> String`; `take(grant, now) -> Option<VerifiedIdentity>` (10 min, single use).
   - `pub struct Stores { pending, handoff, access, grants }` with `Default` and `reap(now)`.
   - `pub fn pkce_challenge(verifier: &str) -> String` and `pub fn random_b64url(bytes: usize) -> String`.
2. `crates/identity/src/link_code.rs`: `LinkCodeStore` with `issue(account_id, now) -> String` (10 chars from `ABCDEFGHJKLMNPQRSTUVWXYZ23456789`, 15 min, one live code per account, replacing the old one), `redeem(code, now) -> Option<u32>` (case-insensitive, single use, constant-time compare per entry), `reap(now)`; `pub fn shared() -> &'static LinkCodeStore` (`std::sync::LazyLock`).
3. `lib.rs`: `pub mod stores; pub mod link_code;`. Cargo: `subtle`, `sha2`, `base64` (workspace).

Tests (pure, `Instant` arithmetic):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `pending_is_single_use_and_expires` | second take `None`; a take after 10 min `None` | C6 |
| `pending_cap_drops_the_oldest` | 10,001 inserts: the first is gone | memory bound |
| `handoff_needs_the_right_verifier_and_redirect` | wrong verifier: `None` and the code is spent; wrong redirect: `None` | PKCE binding |
| `handoff_expires_after_60_s` | redeem at 61 s gives `None` | C6 |
| `access_token_expiry_and_family_revoke` | 15 min; `revoke_family` ends it | new |
| `link_grant_single_use` | second `take` gives `None` | C6 |
| `link_code_alphabet_and_reuse` | 10 chars from the alphabet; lowercase input redeems; second redeem `None`; a second issue invalidates the first | C6 |
| `pkce_rfc7636_vector` | verifier `dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk` gives `E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM` | the S256 formula |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`.

## CT-24 Routes: providers, authorize, callback, dev-mock

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-24-routes-signin` · **Worktree:** `cto-24` (wave head after CT-22 and CT-23)
- **Commit:** `feat(identity): provider list, authorize and callback routes (CT-24)`
- **Rules:** R1-R9. Contract: C4, C6 (`providers`, `authorize`, `callback`, validation), C7 limits, C11, C12.

Files:

1. `crates/identity/src/routes/error.rs`: `pub enum ApiError` (one variant per C6 code: `InvalidRequest`, `UnknownProvider`, `Disabled`, `InvalidGrant`, `AccountDisabled`, `SignupClosed`, `RateLimited { retry_after: u64 }`, `InvalidCredentials`, `NotLinkable`, `InvalidToken`, `ReauthRequired`, `NotFound`, `LastMethod`, `Unavailable`) and `impl IntoResponse` giving the C6 status and JSON body.
2. `routes/ctx.rs`: `pub struct IdentityCtx { pub cfg: Arc<IdentityConfig>, pub providers: Arc<Providers>, pub stores: Arc<Stores>, pub limiter: Arc<RateLimiter>, pub link_codes: &'static LinkCodeStore }` and `pub struct RouteState { pub deps: AuthRouteDeps, pub ctx: IdentityCtx }` (`Clone` through `Arc`s; `AuthRouteDeps` is not `Clone`, so wrap it: `Arc<AuthRouteDeps>`). `fn client_ip(ConnectInfo) -> String`. `fn limit(&self, bucket, key, now) -> Result<(), ApiError>`.
3. `routes/mod.rs`: `pub fn router(deps: AuthRouteDeps, ctx: IdentityCtx) -> axum::Router`, every route behind the `AnyRoute` per-IP limit (a `from_fn_with_state` layer) and an 8 KiB body limit (`DefaultBodyLimit::max(8 * 1024)`); when `cfg.kill.idp_disabled`, every route but `providers` and `launch-ticket*` answers `Disabled`. The `dev-mock` route is mounted only when `deps.developer_mode`.
4. `routes/signin.rs`:
   - `GET /providers`: C6 shape; `soap_relay = !kill.soap_relay_disabled`; `links` only the configured ones.
   - `GET /authorize`: validate per C6 (write `fn valid_redirect_uri(&str) -> bool`, `fn valid_state`, `fn valid_challenge`); `Authorize` limit; unknown provider 404; `provider.prepare().await` (CT-13's trait method) and map its error to `Unavailable`; mint the server PKCE pair and nonce; `stores.pending.insert`; `302` to `authorize_url(server_state, server_challenge, nonce, "{public_base_url}/auth/v1/callback/{id}")`. `intent` query: an opaque token minted by `link/start` (CT-30); in this packet accept only its absence; a present `intent` gives `InvalidRequest` until CT-30 replaces that line.
   - `GET /callback/{provider}`: `stores.pending.take(state)`; missing gives the C6 HTML 400 page (static text, `text/html; charset=utf-8`, no reflected input); provider `error` query param: `302 {redirect_uri}?error=access_denied&state={launcher_state}`; else `exchange(code, verifier, nonce, redirect)`; error: same `access_denied` redirect with `error=server_error` when not `Denied`; success: `stores.handoff.issue(..)` and `302 {redirect_uri}?code=..&state=..`. Build the redirect with `url::Url` query pairs, never string concatenation.
   - `GET /dev-mock/authorize` and `POST /dev-mock/authorize`: the form (C4) and its post, which calls `providers.dev_mock().issue(identity)` and redirects to `/auth/v1/callback/{dev_mock id}?code=..&state=..`. Every form value passes `sanitized_display_name` or the email rule; HTML-escape what is echoed.
   - Spans per C11 (`auth.idp.authorize`, `auth.idp.callback`) with `provider`, `outcome`, `reason`, `latency_ms`; write them inline for now; CT-31 moves them into `telemetry.rs`.
5. `lib.rs`: `pub mod routes;`. Cargo: `axum` (workspace version and features of `crates/auth`), `tower` dev.

Tests (`routes/signin_tests.rs`; `oneshot` against `router(..)` with a test ctx: a `DevMockProvider` and a `wiremock`-backed Discord provider via `with_base`; `TlsConn` inserted by a test layer, since `require_tls` lives in the auth crate):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `providers_lists_configured_providers_and_links` | JSON shape exactly as C6 | the launcher parses it |
| `authorize_refuses_open_redirects` | `http://example.org/cb`, `http://127.0.0.1:80/cb`, `http://127.0.0.1:5000/cb?x=1`, `http://user@127.0.0.1:5000/cb`, `https://127.0.0.1:5000/cb`: 400, no `Location` | open-redirect defence |
| `authorize_refuses_bad_state_and_challenge` | 15-char state, 42-char challenge, method `plain`: 400 | new |
| `authorize_redirects_to_the_provider` | 302 to the stub with the server state, not the launcher's | the two PKCE legs stay separate |
| `callback_round_trip_gives_a_handoff_code` | authorize, then callback with the stub code: 302 to `http://127.0.0.1:<p>/cb?code=..&state=<launcher state>`; `stores.handoff.redeem` with the launcher verifier succeeds | new |
| `callback_with_unknown_state_is_the_expired_page` | 400 `text/html`, body lacks the presented state | no reflection |
| `provider_denial_redirects_access_denied` | `?error=access_denied` from the provider | new |
| `authorize_is_rate_limited` | 21st call in 10 min from one IP: 429 with `retry_after` | C7 |
| `idp_disabled_answers_503_except_providers` | `authorize` 503 `disabled`; `providers` 200 | C4 kill switch |
| `dev_mock_absent_without_developer_mode` | 404 | dev-only route |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`.

## CT-25 Routes: token and logout

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-25-routes-token` · **Worktree:** `cto-25` (at CT-24's reviewed head)
- **Commit:** `feat(identity): token and logout routes (CT-25)`
- **Rules:** R1-R9. Contract: C5, C6 (`token`, `logout`, Tokens, `signed_in`, `needs_link`), C7, C11, D-CT9.

Files:

1. `crates/identity/src/routes/token.rs`:
   - `POST /token` `authorization_code`: `Token` limit per IP; `stores.handoff.redeem(code, code_verifier, redirect_uri)` or `InvalidGrant`; DB `None` gives `Unavailable`. Then `resolve` (CT-14) with facts from `repo::identity::find_bound` and `email_candidates` (only when the provider is `trusted_email` and the identity has a verified email):
     - `Existing(id)`: `touch`, then `signed_in` (below).
     - `AutoLink(id)`: `repo::identity::insert(id, identity, store_email = true)`, event `auto_link`, then `signed_in`.
     - `NeedsLink { hint }`: `stores.grants.issue(identity)` and the `needs_link` body with `signup_open = cfg.signup_open`.
     - `Refused(AccountDisabled)`: 403 `account_disabled`.
     - With `intent = Some(LinkIntent { account_id })` (from `link/start`, CT-30): the identity is attached to that account instead (`Conflict` when bound elsewhere gives 400 `invalid_grant`), event `link`, `signed_in` for that account.
   - `refresh_token` grant: `repo::refresh::rotate`; `Reused`: event `refresh_reuse`, revoke the family's access tokens, 400 `invalid_grant`; `Expired`/`Unknown`: 400 `invalid_grant`; `Rotated`: account facts (disabled: revoke the account's families and access tokens, 403 `account_disabled`), then `signed_in` with `via_code = false`.
   - `pub(super) async fn signed_in(state, account_id, family: Option<&str>, via_code, now) -> Result<Json, ApiError>`: `repo::refresh::issue` (a new family for code grants and links, the same family for rotation), `stores.access.mint(.., via_code)`, body per C6. CT-26 and CT-30 reuse it.
   - `POST /logout`: `repo::refresh::revoke_token`, revoke the family's access tokens; always 204.
   - Span `auth.idp.token` with `outcome` (`signed_in`, `needs_link`, `auto_link`, `refused`, `reuse`) and `account_id` when known.
2. `routes/mod.rs`: mount `token` and `logout`.

Tests (`routes/token_tests_live_db.rs`, every test `live_db`; sentinel accounts `2_000_501..=2_000_515`; identities use provider `dev_mock` and subjects `ct25-<n>`; cleanup by sentinel ids):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_bound_identity_signs_in` | handoff for a bound subject gives `signed_in` with that account and a working refresh token | new |
| `live_db_unknown_identity_needs_link` | `needs_link` with a grant and `signup_open` | new |
| `live_db_trusted_verified_unique_email_auto_links` | a second provider identity with the same verified email auto-links; event `auto_link` | the owner's linking rule |
| `live_db_untrusted_provider_never_auto_links` | same email via an untrusted provider: `needs_link` with hint `none` | the owner's linking rule |
| `live_db_staff_account_is_never_auto_linked` | accesslevel 1 with the matching email: `needs_link` | D-CT6 |
| `live_db_refresh_reuse_revokes_the_family` | rotate, reuse the old token: 400, and the new token now fails | C6 |
| `live_db_disabled_account_refresh_is_403` | disable the account, refresh: 403 and the family revoked | C6 |
| `live_db_logout_revokes` | after logout the refresh token gives 400 | new |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`, S3 `token_tests_live_db`.

## CT-26 Routes: link/new and link/existing

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-26-routes-link` · **Worktree:** `cto-26` (at CT-25's reviewed head)
- **Commit:** `feat(identity): sign-up and link-to-existing-account routes (CT-26)`
- **Rules:** R1-R9. Contract: C5, C6 (`link/new`, `link/existing`), C7, C10, C11, C15 (a, b), D-CT5, D-CT6, D-CT9, D-CT11.

Files:

1. New `crates/identity/src/routes/link.rs`:
   - `POST /link/new`: `stores.grants.take` or `InvalidGrant`; `SignupClosed` unless `cfg.signup_open`; `SignupIp` then `SignupGlobal` limits; `old_enough(identity.provider_account_created_ms, now_ms, cfg.limits.min_provider_account_age_days)` else `SignupClosed`; `repo::account::create(&names::candidates(&names::base_from_display(&identity.display_name), &mut rng))`; insert the identity (store the email only when the provider is `trusted_email` and `email_verified`, D-CT9); event `signup`; `token::signed_in`.
   - `POST /link/existing`: `password_removal_ack` must be JSON `true`, else 400 `invalid_request` before anything else (no limit consumed, grant untouched). Then `LinkExistingIp` (IP) and, for the password form, `LinkExistingAccount` (lowercased name) limits. The grant is checked with `stores.grants.peek` and taken only after the credential check passes, so a wrong password does not burn it (add `peek(grant, now) -> Option<VerifiedIdentity>` to `LinkGrants` in `stores.rs`, with a test). Password form: `cimmeria_auth::verify_account_password` (CT-21): `Invalid` gives 401 `invalid_credentials`, `Disabled` 403 `account_disabled`, `DbError` `Unavailable`. Link-code form: `ctx.link_codes.redeem(code, now)` gives the account id or 401 `invalid_credentials`. Then `repo::account::facts` and `linkable` else 403 `not_linkable` plus event `link_refused`; insert the identity with `repo::identity::insert` (D-CT9 email rule; it clears the account's password and records `password_cleared` in the same transaction, C15 a and b; `Conflict` gives 400 `invalid_grant`), event `link`, `token::signed_in`. Log `password_cleared` (bool) on the span; never the name.
   - Span `auth.idp.link` with `outcome` (`signup`, `link`, `refused`, `invalid_credentials`).
2. `routes/mod.rs`: mount both routes.

Tests (`routes/link_tests_live_db.rs`, every test `live_db`; sentinel accounts `2_000_516..=2_000_530`; subjects `ct26-<n>`; cleanup by sentinel ids and by the accounts `link/new` created, found through their identity subject):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_signup_creates_a_passwordless_account` | name passes `is_valid_account_name`; `password` and `password_hash_v2` NULL; event `signup` | open sign-up (D-CT5) |
| `live_db_signup_stores_email_only_when_trusted_and_verified` | trusted+verified: email stored; untrusted: NULL | D-CT9 |
| `live_db_signup_closed_and_ip_limit` | `signup_open = false`: 403; 4th sign-up from one IP in a day: 429 | C7 |
| `live_db_signup_refuses_young_provider_accounts` | `min_provider_account_age_days = 7`, created 1 day ago: 403 `signup_closed` | sign-up abuse guard |
| `live_db_link_existing_wrong_password_keeps_the_grant` | 401, then the right password succeeds with the same grant | new |
| `live_db_link_existing_is_rate_limited_per_account` | 6th try against one name from 6 IPs: 429 | brute-force guard |
| `live_db_staff_and_test_accounts_are_not_linkable` | accesslevel 1 and `test_account` with the right password: 403 `not_linkable`, event `link_refused` | D-CT6 |
| `live_db_link_code_links_once` | a code from `LinkCodeStore::issue` links; reuse 401 | C10 |
| `live_db_link_existing_clears_the_password` | a legacy (algo 1) account links with its plaintext and the ack: `password` and `password_hash_v2` NULL; events `password_cleared` and `link`; afterwards `cimmeria_auth::verify_account_password` with the old plaintext is `Invalid` | D-CT11 (a): a linked account has no password |
| `live_db_link_code_clears_the_password` | a code from `LinkCodeStore::issue` for an account with a password links; both columns NULL; event `password_cleared` | D-CT11 (b) |
| `live_db_link_existing_needs_the_removal_ack` | the right password without `password_removal_ack`, and with `false`: 400 `invalid_request`; the password is unchanged and the same grant then links with the ack | the warning cannot be skipped |
| `live_db_refused_link_keeps_the_password` | a `test_account` with the right password and the ack: 403 `not_linkable`; its password row is unchanged | a refused link changes nothing |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`, S3 `link_tests_live_db`.