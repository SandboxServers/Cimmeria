# client-tls-oidc: shared contract

> **Type:** Reference
> **Audience:** Packet coders and reviewers of the client-tls-oidc campaign
> **Last updated:** 2026-10-26
> **Companions:** [Ledger](README.md), [work packets](work-packets.md), [design](design.md)

Every packet builds against this file. A packet that needs something not written here stops and reports `blocked - contract gap: <what>`; it never invents a wire field, a column or an error code. Names below are exact.

## C1. Launch ticket (server, `crates/auth/src/auth/launch_ticket.rs`)

- A ticket is 32 characters of `[A-Za-z0-9]` drawn from a CSPRNG (`rand` 0.10, `rand::rng()` with `distr::Alphanumeric`), about 190 bits.
- The client sends `SHA1HEX(ticket)` as the password: 40 uppercase hex characters, which `classify_credential` already classifies as `LegacySha1Hex`.
- The store keeps `SHA-256(SHA1HEX uppercase)` as `[u8; 32]`, never the ticket or the SHA-1. Compare in constant time (add `subtle = "2"` to `crates/auth/Cargo.toml`; `ConstantTimeEq`).
- One live ticket per account. `mint` replaces the old one.
- Life: 12 h absolute (`TICKET_ABSOLUTE`), 30 min idle (`TICKET_IDLE`) since mint, last use or last heartbeat. Several uses inside the life are allowed (Change Server re-sends it, design F14).
- Accepted only when the request arrived over TLS (`Option<Extension<TlsConn>>` is `Some`). A matching ticket over plain HTTP is refused with audit outcome `ticket_requires_tls` and `login_error(4, ...)`; the ticket is then revoked.
- Never logged. Log fields carry `account_id` only.

```rust
pub const TICKET_ABSOLUTE: Duration = Duration::from_secs(12 * 3600);
pub const TICKET_IDLE: Duration = Duration::from_secs(30 * 60);
pub const TICKET_LEN: usize = 32;
pub struct MintedTicket { pub account_name: String, pub ticket: String, pub expires_in_secs: u64 }
pub enum TicketCheck { NoMatch, Valid { account_id: u32 } }
#[derive(Default)]
pub struct LaunchTicketStore { /* Mutex<HashMap<u32, Entry>>; Entry{name_lower, hash, launch_id, minted, last_used} */ }
impl LaunchTicketStore {
    pub fn mint(&self, account_id: u32, account_name: &str, launch_id: &str, now: Instant) -> MintedTicket;
    pub fn verify(&self, account_name: &str, sha1_hex: &str, now: Instant) -> TicketCheck; // name compare is ASCII case-insensitive; refreshes last_used on Valid
    pub fn heartbeat(&self, account_id: u32, launch_id: &str, now: Instant) -> bool;
    pub fn end(&self, account_id: u32, launch_id: &str) -> bool;
    pub fn revoke_account(&self, account_id: u32);
    pub fn reap(&self, now: Instant) -> usize;
    pub fn live_count(&self) -> usize;
}
```

## C2. Auth crate hooks (`crates/auth`)

```rust
// crates/auth/src/auth/mod.rs (re-exported from lib.rs)
pub struct AuthRouteDeps {
    pub db: Option<Arc<PgPool>>,
    pub launch_tickets: Arc<LaunchTicketStore>,
    pub developer_mode: bool,
}
pub type IdentityRoutes = Box<dyn FnOnce(AuthRouteDeps) -> axum::Router + Send + Sync>;
impl AuthService { pub fn set_identity_routes(&mut self, routes: IdentityRoutes); }
```

- `start()` calls the closure once, after the pool is known, and nests the result at `/auth/v1` behind `require_tls`: a request without `TlsConn` gets `403` with body `{"error":"https_required"}`.
- New `LoginOutcome` variants and `as_str()`: `LaunchTicket` = `"launch_ticket"`, `TicketRequiresTls` = `"ticket_requires_tls"`. The `login_audit.outcome` CHECK gains the same two strings (C5).
- The proxy listener: `ServerConfig.auth_proxy_addr: Option<SocketAddr>` in `crates/common/src/config.rs` (env `AUTH_PROXY_ADDR`, unset by default), copied into `AuthService` by `AuthService::new` like `auth_tls_cert_path`. `AuthService` also owns `launch_tickets: Arc<LaunchTicketStore>` (getter `launch_tickets()`), shared with `HandlerState` and `AuthRouteDeps`. Every request on it passes `proxy_layer`:
  0. The TCP peer (`ConnectInfo`) must lie in `ServerConfig.auth_proxy_trusted_peers` (env `AUTH_PROXY_TRUSTED_PEERS`, comma-separated CIDR list or bare addresses, default `127.0.0.0/8,::1`, loopback only; a list with a bad entry trusts no peer and logs ERROR `bad_auth_proxy_trusted_peers`). Any other peer has `X-Forwarded-*` ignored and is served as plain HTTP with no `TlsConn`, so `require_tls` answers 403 `https_required`.
  1. `X-Forwarded-Proto` must equal `https`, else `400`.
  2. The rightmost `X-Forwarded-For` entry must parse as an `IpAddr`, else `400`. It replaces the `ConnectInfo<SocketAddr>` extension (port 0), so handlers stay unchanged.
  3. Insert `TlsConn`.
- Trust comes from the listener, never from a header on the other listeners: the plain `8081` and in-process TLS listeners ignore `X-Forwarded-*`.
- Env names read by `crates/server/src/main.rs` `config_from_env`: `AUTH_TLS_CERT_PATH`, `AUTH_TLS_KEY_PATH`, `AUTH_PROXY_ADDR`, `AUTH_PROXY_TRUSTED_PEERS`.

## C3. Identity crate (`crates/identity`, package `cimmeria-identity`)

May depend on `cimmeria-auth` and `cimmeria-common`, never the reverse.

```rust
pub struct ProviderId(String);            // ^[a-z0-9_-]{1,32}$
pub enum ProviderKind { Discord, Oidc, DevMock }
pub struct VerifiedIdentity {
    pub provider: ProviderId,
    pub issuer: String,                   // Discord: "https://discord.com"
    pub subject: String,                  // provider's stable user id, 1-255 chars
    pub display_name: String,             // sanitized: printable ASCII, max 64
    pub email: Option<String>,            // lowercased; None unless the provider returned one
    pub email_verified: bool,
    pub provider_account_created_ms: Option<i64>,
}
pub enum ProviderError { Denied, Timeout, Http(u16), BadResponse(&'static str), Token(&'static str) }
#[async_trait::async_trait]
pub trait IdentityProvider: Send + Sync {
    fn id(&self) -> &ProviderId;
    fn display_name(&self) -> &str;
    fn trusted_email(&self) -> bool;
    /// `redirect_uri` is `{public_base_url}/auth/v1/callback/{id}`.
    fn authorize_url(&self, state: &str, pkce_challenge: &str, nonce: &str, redirect_uri: &str) -> String;
    async fn exchange(&self, code: &str, pkce_verifier: &str, nonce: &str, redirect_uri: &str)
        -> Result<VerifiedIdentity, ProviderError>;
}
```

- The server is a confidential OAuth client of each provider and uses its own PKCE (S256) and `state` towards the provider. The launcher-to-server leg (C6) has a separate PKCE pair.
- Provider HTTP: one shared `reqwest::Client`, 10 s timeout, no redirects, response bodies capped at 64 KiB.
- `ProviderError` values become the `reason` field of spans; they never carry a code, token, subject or email.

## C4. Provider config (`config/auth-providers.toml`)

Path from env `CIMMERIA_AUTH_PROVIDERS`, default `config/auth-providers.toml`. A missing file means no providers and an empty `providers` list (C6). The repo ships only `config/auth-providers.toml.example`; `config/auth-providers.toml` is gitignored.

```toml
public_base_url = "https://play.example.org"   # no trailing slash; https only
signup_open = true

[limits]
signups_per_ip_per_day = 3
signups_global_per_hour = 30
min_provider_account_age_days = 0

[links]                                         # optional; shown in the game's login screen
website = "https://example.org"
account = "https://example.org/account"
forums = "https://example.org/forums"

[[provider]]
id = "discord"
kind = "discord"
display_name = "Discord"
client_id = "000000000000000000"
client_secret_env = "CIMMERIA_IDP_DISCORD_SECRET"
trusted_email = true

[[provider]]
id = "google"
kind = "oidc"
display_name = "Google"
issuer = "https://accounts.google.com"
client_id = "example.apps.googleusercontent.com"
client_secret_env = "CIMMERIA_IDP_GOOGLE_SECRET"
scopes = ["openid", "email", "profile"]
trusted_email = true
```

Validation (`config::load`, pure over the parsed text plus an env lookup closure):

| Condition | Result |
|---|---|
| Unknown `kind`, duplicate `id`, bad `id` pattern, `public_base_url` not `https://` | `ConfigError` (fatal at startup, logged with the field name) |
| `client_secret_env` names an unset or empty variable | provider skipped, `WARN reason=secret_missing provider=<id>` |
| `kind = "oidc"` without `issuer`, or `issuer` not `https://` | `ConfigError` |
| `trusted_email = true` and the issuer host is `login.microsoftonline.com` or `login.live.com` | `ConfigError` `microsoft_email_untrusted` (Microsoft's `email` claim is mutable) |
| `kind = "dev_mock"` and developer mode is off | provider skipped, `WARN reason=dev_mock_refused` |
| The file has `client_secret` (a literal secret) anywhere | `ConfigError` `literal_secret` |

Kill switches (env, read once at startup): `CIMMERIA_IDP_DISABLED=1` (no providers; `/auth/v1/*` except `providers` and `launch-ticket*` answer `503 {"error":"disabled"}`), `CIMMERIA_IDP_SIGNUP_DISABLED=1` (`signup_open=false`), `CIMMERIA_SOAP_RELAY_DISABLED=1` (`soap_relay=false`).

`dev_mock`: `authorize_url` returns `{public_base_url}/auth/v1/dev-mock/authorize?...`; that route (mounted only in developer mode) shows one HTML form (subject, display name, email, verified checkbox) and redirects to the callback with a code. `exchange` returns what the form posted. Lab UAT uses it instead of Discord.

## C5. Schema (additive; persistent)

Files: `db/sgw/Accounts/Tables/account_identity.sql`, `auth_refresh_token.sql`, `account_identity_event.sql`; included from `db/database.sql` right after the `account.sql` include. Foreign keys go in `db/sgw/_foreign_keys.sql`, as `sgw_player_account_id_fkey` does. All three tables go in `db/persistent/persistent-public-tables.txt`, so a colo redeploy carries them (step 6 of `docker/db-rebuild.sh` fixes their sequences).

```sql
CREATE TABLE account_identity (
    identity_id bigserial PRIMARY KEY,
    account_id integer NOT NULL,
    provider character varying(32) NOT NULL,
    issuer text NOT NULL,
    subject text NOT NULL,
    display_name character varying(64) DEFAULT ''::character varying NOT NULL,
    email text,                       -- set only when verified by a trusted_email provider
    email_verified_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT account_identity_subject_key UNIQUE (provider, issuer, subject)
);
CREATE INDEX account_identity_account_idx ON account_identity (account_id);
CREATE INDEX account_identity_email_idx ON account_identity (lower(email)) WHERE email IS NOT NULL;

CREATE TABLE auth_refresh_token (
    token_hash bytea PRIMARY KEY,     -- SHA-256 of the token string
    family_id character varying(32) NOT NULL,
    account_id integer NOT NULL,
    issued_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL,
    family_expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    replaced_by bytea
);
CREATE INDEX auth_refresh_token_family_idx ON auth_refresh_token (family_id);
CREATE INDEX auth_refresh_token_account_idx ON auth_refresh_token (account_id);

CREATE TABLE account_identity_event (
    event_id bigserial PRIMARY KEY,
    account_id integer NOT NULL,
    identity_id bigint,
    kind character varying(24) NOT NULL CHECK (kind IN ('signup', 'link', 'auto_link', 'unlink',
        'link_refused', 'refresh_reuse', 'password_cleared', 'link_code_issued')),
    provider character varying(32),
    at timestamp with time zone DEFAULT now() NOT NULL
);
CREATE INDEX account_identity_event_account_idx ON account_identity_event (account_id);
```

```sql
-- _foreign_keys.sql
ALTER TABLE ONLY account_identity ADD CONSTRAINT account_identity_account_id_fkey FOREIGN KEY (account_id) REFERENCES account(account_id) ON UPDATE RESTRICT ON DELETE CASCADE;
ALTER TABLE ONLY auth_refresh_token ADD CONSTRAINT auth_refresh_token_account_id_fkey FOREIGN KEY (account_id) REFERENCES account(account_id) ON UPDATE RESTRICT ON DELETE CASCADE;
ALTER TABLE ONLY account_identity_event ADD CONSTRAINT account_identity_event_account_id_fkey FOREIGN KEY (account_id) REFERENCES account(account_id) ON UPDATE RESTRICT ON DELETE CASCADE;
```

`account` keeps its columns; two guard triggers (C15) enforce D-CT11:

```sql
-- _functions.sql
CREATE FUNCTION account_password_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (NEW.password IS NOT NULL OR NEW.password_hash_v2 IS NOT NULL)
       AND EXISTS (SELECT 1 FROM account_identity WHERE account_id = NEW.account_id) THEN
        RAISE EXCEPTION 'account_has_identity' USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;
CREATE FUNCTION account_identity_password_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF EXISTS (SELECT 1 FROM account WHERE account_id = NEW.account_id
               AND (password IS NOT NULL OR password_hash_v2 IS NOT NULL) FOR SHARE) THEN
        RAISE EXCEPTION 'account_has_password' USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;
-- _triggers.sql
CREATE TRIGGER account_password_guard BEFORE INSERT OR UPDATE OF password, password_hash_v2 ON account
    FOR EACH ROW EXECUTE FUNCTION account_password_guard();
CREATE TRIGGER account_identity_password_guard BEFORE INSERT ON account_identity
    FOR EACH ROW EXECUTE FUNCTION account_identity_password_guard();
```

`login_audit.outcome` CHECK gains `'launch_ticket'` and `'ticket_requires_tls'`. No seed rows. The event table holds no subject, email or IP.

## C6. HTTP API `/auth/v1` (JSON, TLS only)

All bodies are JSON (`Content-Type: application/json`), max request body 8 KiB. Errors are `{"error":"<code>"}` with the status shown. Every route is rate limited (C7). `Bearer` means `Authorization: Bearer <access_token>`.

| Method and path | Request | Success | Errors |
|---|---|---|---|
| `GET /providers` | - | `200 {"api":1,"soap_relay":bool,"signup_open":bool,"providers":[{"id","display_name"}],"links":{"website","account","forums"}?}` | - |
| `GET /authorize` | query `provider`, `redirect_uri`, `state`, `code_challenge`, `code_challenge_method=S256`, optional `intent` (from `link/start`) | `302` to the provider | `400 invalid_request`, `404 unknown_provider`, `503 disabled` |
| `GET /callback/{provider}` | provider's `code`, `state` | `302 {redirect_uri}?code=<H>&state=<S1>`; provider refusal `302 {redirect_uri}?error=access_denied&state=<S1>` | unknown or expired state: `400` HTML page "This sign-in link has expired. Return to the launcher and try again." |
| `POST /token` | `{"grant_type":"authorization_code","code","code_verifier","redirect_uri"}` or `{"grant_type":"refresh_token","refresh_token"}` | `200` `signed_in` or `needs_link` (below) | `400 invalid_grant`, `400 invalid_request`, `403 account_disabled`, `403 not_linkable`, `403 signup_closed`, `429 rate_limited` |
| `POST /link/new` | `{"link_grant"}` | `200` `signed_in` | `400 invalid_grant`, `403 signup_closed`, `429 rate_limited` |
| `POST /link/existing` | `{"link_grant","account_name","password","password_removal_ack":true}` or `{"link_grant","link_code","password_removal_ack":true}` | `200` `signed_in`; the account's password is cleared (C15 a) | `400 invalid_grant`, `400 invalid_request` (ack missing or not `true`), `401 invalid_credentials`, `403 not_linkable`, `403 account_disabled`, `429 rate_limited` |
| `POST /link/start` | Bearer, token issued by an `authorization_code` grant less than 10 min ago; `{"provider","redirect_uri","state","code_challenge","code_challenge_method":"S256"}` | `200 {"authorize_url"}` (an `/authorize` URL carrying `intent`) | `400 invalid_request`, `401 invalid_token`, `403 account_disabled`, `403 reauth_required` (the token came from a refresh, or from a code grant more than 10 min ago), `404 unknown_provider`, `429 rate_limited`, `503 unavailable` |
| `GET /account` | Bearer | `200 {"account_id","account_name","display_name","identities":[{"identity_id","provider","display_name","created_at","last_used_at"}]}`; `created_at` and `last_used_at` are integer epoch milliseconds (UTC) | `401 invalid_token`, `403 account_disabled`, `503 unavailable` |
| `DELETE /identities/{identity_id}` | Bearer, token issued by an `authorization_code` grant less than 10 min ago | `204` | `401 invalid_token`, `403 account_disabled`, `403 reauth_required`, `404 not_found`, `409 last_method` |
| `POST /launch-ticket` | Bearer; `{"launch_id"}` (16-64 chars `[A-Za-z0-9-]`) | `200 {"account_name","ticket","expires_in"}` | `400 invalid_request`, `401 invalid_token`, `403 account_disabled`, `429 rate_limited`, `503 unavailable` |
| `POST /launch-ticket/heartbeat` | Bearer; `{"launch_id"}` | `204` | `400 invalid_request`, `401 invalid_token`, `403 account_disabled`, `404 not_found`, `429 rate_limited`, `503 unavailable` |
| `POST /launch-ticket/end` | Bearer; `{"launch_id"}` | `204` (also when already gone; `403` or `503` when the account is disabled or the database cannot be read, and then the ticket lives out its 30 min idle limit) | `400 invalid_request`, `401 invalid_token`, `403 account_disabled`, `429 rate_limited`, `503 unavailable` |
| `POST /logout` | `{"refresh_token"}` | `204` (always) | - |

Every `/auth/v1` response carries `Cache-Control: no-store`: successes, errors, redirects and the kill-switch refusal (the `guard` layer in `routes/mod.rs` sets it).:"signed_in","access_token","expires_in":900,"refresh_token","account":{"account_id","account_name","display_name"}}`.

`needs_link` body: `{"result":"needs_link","link_grant","expires_in":600,"identity":{"provider","display_name"},"hint":"none"|"email_match","signup_open":bool}`.

Validation of `/authorize`:
- `redirect_uri` must parse (`url` crate) as `http://127.0.0.1:<port>/cb` or `http://[::1]:<port>/cb`, port 1024-65535, no query, fragment or userinfo. Anything else is `400 invalid_request` and **no redirect** (no open redirect).
- `state`: 16-128 chars of `[A-Za-z0-9_-]`. `code_challenge`: exactly 43 chars of base64url. Method must be `S256`.
- The pending record (provider, redirect_uri, launcher state, launcher challenge, server PKCE verifier, nonce, intent) lives 10 min, keyed by a fresh server `state`; at most 10,000 pending records (oldest dropped).

The handoff code `H`: 32 random bytes base64url, single use, 60 s, bound to the launcher's `code_challenge` and `redirect_uri`. `/token` checks `BASE64URL(SHA256(code_verifier)) == code_challenge` and the same `redirect_uri`.

Tokens:
- Access token: 32 random bytes base64url, kept only in server memory as SHA-256, 15 min, records `account_id`, `family_id`, `issued_at`, `via_code` (true when minted by an `authorization_code` grant or a link).
- Refresh token: 32 random bytes base64url, stored as SHA-256 in `auth_refresh_token`. Each use rotates it: the old row gets `replaced_by` and `revoked_at`. A presented token that is already revoked or replaced revokes the whole family and writes event `refresh_reuse`. Idle limit 30 days (`last_used_at`), family limit 180 days (`family_expires_at`), both provisional (D-CT14).
- A disabled account (`account.enabled = false`) gets `403 account_disabled` from `token`, `link/*` and every Bearer route (`account`, `identities/{id}`, `launch-ticket*`; the Bearer routes check `enabled` on each call). Only the refresh grant also revokes: it revokes the account's refresh families, access tokens and launch ticket. An access token minted before the disable stays valid for its 15 minutes but is refused by every Bearer route.

`link_grant`: 32 random bytes base64url, in memory, 10 min, single use, holds the `VerifiedIdentity`. `link_code` (GM-issued, C10): 10 chars `[A-HJ-NP-Z2-9]`, 15 min, single use, bound to one account.

Methods for `last_method`: the account's identities only (a linked account has no password, C15). Unlinking refuses when the count would reach 0; recovery is a GM link code (C15 e).

## C7. Linking, sign-up and limits (pure, `crates/identity/src/link.rs`, `names.rs`, `limits.rs`)

```rust
pub const STAFF_ACCESSLEVEL_MIN: i32 = 1;  // accesslevel >= 1 is staff
pub struct AccountFacts { pub account_id: u32, pub enabled: bool, pub accesslevel: i32, pub test_account: bool }
pub struct LinkFacts {
    pub bound_to: Option<AccountFacts>,         // account already holding this (provider, issuer, subject)
    pub email_candidates: Vec<AccountFacts>,    // distinct accounts with a trusted, verified identity of the same lower(email)
    pub provider_trusted_email: bool,
    pub email_verified: bool,
}
pub enum Resolution { Existing(u32), AutoLink(u32), NeedsLink { hint: LinkHint }, Refused(Refusal) }
pub enum LinkHint { None, EmailMatch }
pub enum Refusal { AccountDisabled }
pub fn resolve(facts: &LinkFacts) -> Resolution;
pub fn linkable(a: &AccountFacts) -> bool;       // enabled && accesslevel < STAFF_ACCESSLEVEL_MIN && !test_account
```

`resolve` rules, in order:
1. `bound_to = Some(a)`: `Existing(a)` when `a.enabled`, else `Refused(AccountDisabled)`.
2. `provider_trusted_email && email_verified && email_candidates.len() == 1 && linkable(&email_candidates[0])`: `AutoLink`.
3. Otherwise `NeedsLink`, with `hint = EmailMatch` only when `provider_trusted_email && email_verified && !email_candidates.is_empty()`, else `None`.

Account names (`names.rs`), all results match `^[A-Za-z0-9_-]{3,20}$`:
- `base` = the display name's ASCII `[A-Za-z0-9_-]` characters, first 15 kept.
- `base` becomes `player` when it is shorter than 3, or when it starts (ASCII case-insensitive) with `gm`, `admin`, `test`, `lab`, `cimmeria`, `system`, `dev`, `mod`, `staff` or `support`.
- Candidates: `{base}_{dddd}` (4 random digits), 8 tries; then `player{dddddddd}`. Uniqueness is `lower(account_name)`, checked by the repository on insert.

Rate limits (`limits.rs`, an in-memory keyed token bucket; the clock is injected):

| Key | Limit |
|---|---|
| any `/auth/v1/*` request, per IP | 60 per minute |
| `authorize`, per IP | 20 per 10 min |
| `token`, per IP | 30 per 10 min |
| `link/existing`, per IP and per target account name | 5 per 15 min each |
| `LinkCodeGlobal`: key `""`, charged on every `link_code` redeem on `link/existing` | 50 per 15 min |
| account creation, per IP | `signups_per_ip_per_day` |
| account creation, global | `signups_global_per_hour` |

IPv6 clients are keyed by their /64 prefix, not the full address.

`429` carries `{"error":"rate_limited","retry_after":<secs>}`. Sign-up also requires `signup_open`, and, when `min_provider_account_age_days > 0` and the provider gave `provider_account_created_ms`, an old enough provider account (Discord: `(id >> 22) + 1420070400000` ms).

## C8. Client overlay and `LoginInternal.lua` (`data/client-patches/017-launcher-sign-in/`)

`LoginInternal.lua` in ticket mode, rendered by `launcher-core` `client_setup::login_session::render_ticket` (golden file `data/client-patches/017-launcher-sign-in/test/LoginInternal.ticket.lua`, ASCII, CRLF, `-text`):

```lua
-- LoginInternal.lua, written by the Cimmeria launcher for one launch.
-- It is rewritten when the game exits.

LoginMod = {}

function LoginMod.loadServerSystems()
    LoginMod.servers = {}
    LoginMod.servers["Cimmeria"] = "http://127.0.0.1:50123"
    CimmeriaLogin = {}
    CimmeriaLogin.version = 1
    CimmeriaLogin.mode = "ticket"
    CimmeriaLogin.server = "Cimmeria"
    CimmeriaLogin.account = "Player_1234"
    CimmeriaLogin.ticket = "AbCdEf0123456789AbCdEf0123456789"
    CimmeriaLogin.display = "Player"
    CimmeriaLogin.auto = false
    CimmeriaLogin.links = {}
    CimmeriaLogin.errors = {}
    CimmeriaLogin.errors[4] = "Your sign-in for this launch is no longer valid. Close the game and press Play in the launcher."
    CimmeriaLogin.errors[5] = "This account is suspended. Contact the server staff."
    if LoginMod.cimmeriaOverlayVersion == nil then
        Login_AccountEdit:setText(CimmeriaLogin.account)
        Login_PasswordEdit:setText(CimmeriaLogin.ticket)
    end
end
```

- Ticket mode writes exactly one server: the signed-in one, pointing at its relay. With one entry the stock code hides the server combo.
- `links` entries are `CimmeriaLogin.links.website = "..."` lines, written only for links the server's `/providers` returned.
- Every string passes `sanitize`: printable ASCII 0x20-0x7E, no `"` and no `\`, max 32 chars for `display` (fallback: the account name), max 160 for error texts and links; links must start with `https://`.
- `auto` is `TICKET_AUTO_SUBMIT`, a `const bool = false` in `login_session.rs` until spike V2 passes (CT-57 flips it).
- Without the overlay (an install that has not applied 017), the fallback block pre-fills the stock boxes; the player presses Login.
- Relay password mode is the plain `login_servers::render` with each relayed server's URL replaced by its relay URL; no `CimmeriaLogin`. It is used only when the player is **not** signed in (a seed or other non-OIDC account typing its password). A signed-in Play whose ticket cannot be minted never falls back to it, nor to the plain list (C15 d).

Overlay `Login.lua` (whole file; the stock file plus these changes):
- Top level: `LoginMod.cimmeriaOverlayVersion = 1`.
- `LoginMod.startup()` runs the stock steps, then `LoginMod.applyCimmeria()`.
- `applyCimmeria()`: when `CimmeriaLogin` is nil or `CimmeriaLogin.version ~= 1` or `mode ~= "ticket"`, do nothing (stock behaviour). In ticket mode: hide `Login_AccountEdit`, `Login_PasswordEdit`, `Login_AccountLabel`, `Login_PasswordLabel`, `Login_RememberCheck`, `Login_ServerTypeCombo`; show `Login_StatusText` with `"Signed in as " .. display .. " (" .. account .. ")"`; set `Login_LoginButton` text to `"Enter"`; show `Login_PasswordModeButton`; when `auto` is true and `LoginMod.cimmeriaAutoTried` is nil, set it and call `LoginMod.submitTicket()`.
- `submitTicket()`: status `"Connecting..."` first (first-press feedback), then `accountLogin(LoginMod.servers[CimmeriaLogin.server], CimmeriaLogin.account, CimmeriaLogin.ticket)`. Never writes `GSavedVar_AccountName`.
- `onLoginClicked` in ticket mode calls `submitTicket()`; otherwise the stock body.
- `onLoginFailure`: in ticket mode set the button text to `"Retry"` and the status to `CimmeriaLogin.errors[errorNum]` when present, else `"Login failed (" .. errorNum .. "). Keep the launcher open, then press Retry."`; outside ticket mode the stock prompt, using `CimmeriaLogin.errors[errorNum]` in place of `errorStr` when present.
- `Login_PasswordModeButton` ("Use password"): leaves ticket mode for this session (`LoginMod.cimmeriaPasswordMode = true`), shows the stock widgets, clears both boxes, hides the status text and itself.
- Website, account and forums buttons: open `CimmeriaLogin.links.<name>` when `CimmeriaLogin` is set, hide the button when that link is nil; the stock URL otherwise.

Overlay `Login.layout`: the stock file plus `Name="Login_AccountLabel"` and `Name="Login_PasswordLabel"` on the two unnamed caption windows, a `StaticText_2` `Login_StatusText` (hidden, word-wrapped, where the account caption sits) and a `TextButton_2` `Login_PasswordModeButton` (hidden, text `Use password`, under the login button, inside the panel).

## C9. Launcher relay and sign-in (desktop workspace)

Crate `crates/launcher/desktop/relay`, package `cimmeria-launcher-relay`:

```rust
pub struct RelayConfig {
    pub upstream: url::Url,                    // https://host[:port], no path
    pub extra_root_pem: Option<Vec<u8>>,       // debug builds only
    pub session: Option<std::convert::Infallible>, // D-MR6 decided: the launch ticket; filled by mercury-replacement, always None here
}
pub struct RelayHandle { /* port, shutdown sender, join handle, counters */ }
impl RelayHandle { pub fn soap_port(&self) -> u16; pub fn soap_url(&self) -> String; pub async fn shutdown(self); }
pub async fn start(cfg: RelayConfig) -> Result<RelayHandle, RelayError>;
pub mod callback {
    pub struct CallbackServer;
    impl CallbackServer {
        pub async fn bind() -> std::io::Result<CallbackServer>;   // 127.0.0.1:0
        pub fn redirect_uri(&self) -> String;                    // http://127.0.0.1:<port>/cb
        pub async fn wait(self, expected_state: &str, timeout: Duration) -> CallbackResult;
    }
    pub enum CallbackResult { Code(String), Denied, TimedOut }
}
```

A request whose `state` is not the launcher's is answered 400 and ignored; only a matching state ends the wait.

Relay rules: bind `127.0.0.1:0`; accept only `POST /SGWLogin/UserAuth` and `POST /SGWLogin/ServerSelection` (else `404`); `Host` must equal `127.0.0.1:<port>` (else `421`); any `Origin` header gives `403`; request body capped at 64 KiB (`413`); forward body, `Content-Type`, `SOAPAction` and `Cookie`; return status, body, `Content-Type` and `Set-Cookie`. Upstream client: reqwest 0.12 with rustls, TLS 1.2 minimum, WebPKI roots, `local_address(0.0.0.0)` so the auth leg uses IPv4 like the game's UDP, 20 s timeout, no redirects, response capped at 256 KiB. Upstream failure gives `502` with an empty body. No log line carries a body, cookie or header value.

Engine module `crates/launcher/desktop/engine/src/sign_in/`:
- `discovery.rs`: `probe(login_url: &str) -> Option<Discovery>`; auth base `https://<host>` (port 443) of the login server URL; `GET {base}/auth/v1/providers`, 4 s timeout; `Discovery { auth_base, soap_relay, signup_open, providers: Vec<ProviderView>, links: Links }`; cached per server for 10 min. Debug builds only (`cfg!(debug_assertions)`): env `CIMMERIA_LAUNCHER_AUTH_BASE` overrides the base and `CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM` adds a root.
- `oauth.rs`: verifier = 48 random bytes base64url (64 chars); challenge = base64url(SHA-256(verifier)); state = 32 random bytes base64url; sign-in timeout 5 min.
- `store.rs`: `trait CredentialStore { get, set, delete }` keyed `cimmeria-launcher/<auth host>`; value JSON `{"refresh_token","account_name","display_name"}`; `KeyringStore` (crate `keyring` 3, features `windows-native`, `apple-native`) and `MemoryStore`. Never written to `launcher-config.json`, `preferences.json` or a log.
- `session.rs`: `SignIn` with `begin`, `complete`, `link_new`, `link_existing_password`, `link_existing_code`, `account`, `unlink`, `add_method`, `sign_out`, `access_token` (refreshes when under 60 s left), `launch_ticket`, `heartbeat`, `end_ticket`.

Play (`engine/src/storage/launch/sign_in_play.rs`): the telemetry mint and session keep the original `servers`; only the list written into `LoginInternal.lua` changes (`worker.rs` passes `servers` to `telemetry::session` and `open_session`, which derive their endpoint from it). Signed in, a failed ticket mint (offline, `401`, `403`, `5xx`) ends the launch as `NotStarted` with reason `ticket_unavailable` (`sign_in_ended` when the refresh gave `invalid_grant`), records that reason in `DesktopState` (cleared when the next Play starts), and the sign-in panel shows it through `SignInStatus.play_refusal` (C13); it never writes a password-mode `LoginInternal.lua` (C15 d).

## C10. GM console (`.account`)

`.account identities <name>`, `.account unlink <name> <identity_id>`, `.account linkcode <name>`: GM-gated like the other `.` commands; `linkcode` refuses staff and test accounts (`not_linkable`) and prints the code once; each writes an `account_identity_event`. Output never shows subjects or emails, only provider, display name and dates.

Transport, the `.bankdump` pattern (the cell has no DB pool): the cell forwards `CellToBaseMsg::Account(AccountCellToBase::{Identities, Unlink, LinkCode})` (new file `crates/wire/src/cell/messages/account_cell_to_base.rs`, fields, as `BankCellToBase::GmDump` carries them, `entity_id: u32` and `account_id: Option<u32>` of the GM from the cell's own session state, then `account_name: String`, and `identity_id: i64` for `Unlink`; `CellToBaseMsg::Account` sits beside `Bank` in `cell_to_base.rs` with a `"account"` row in `cell_to_base_kind.rs`); a base arm in `base-world-entry` `cell_dispatch/account_dispatch.rs` calls `cimmeria_base_session::base::account_admin::run`, which reads through the identity repositories and answers each line on the GM feedback channel. Link codes live in `cimmeria_identity::link_code::LinkCodeStore`; production code reaches the one process-wide store through `link_code::shared() -> &'static LinkCodeStore` (a `LazyLock`), tests build their own `LinkCodeStore::default()`.

## C11. Telemetry

Spans (target `cimmeria_identity` or `cimmeria_auth`). Emitted by `crates/identity`, one helper each in `src/telemetry.rs`: `auth.idp.authorize`, `auth.idp.callback`, `auth.idp.token`, `auth.idp.link`, `auth.idp.link_start`, `auth.idp.unlink`, `auth.launch_ticket.mint`, `auth.launch_ticket.heartbeat`, `auth.launch_ticket.end`. Emitted by `crates/auth`: `auth.launch_ticket.consume` (`handlers.rs` `consume_span`, opened where the ticket is spent, because identity cannot be called from auth; fields `account_id`, `outcome`, `reason`, `latency_ms`, no `provider`). `GET /account` has no span, and the reaps have none: the auth reaper writes one DEBUG line `Reaped expired auth entries` (`service.rs`) with the counts `expired_sessions`, `expired_tickets` and `expired_launch_tickets`, only when a count is above 0. Fields: `provider`, `outcome`, `reason`, `account_id`, `latency_ms`. Never a code, token, ticket, subject, email, display name or IP in a field. Launcher events: `launcher.sign_in.{started,completed,failed}`, `launcher.relay.{started,stopped}` with outcome and reason only.

## C12. Identity crate module map

Every packet puts its code where this map says, so the waves merge without conflicts.

| File (under `crates/identity/src/`) | Packet | Holds |
|---|---|---|
| `lib.rs` | CT-11 | module list and re-exports; each later packet adds its own `pub mod` line |
| `provider.rs` | CT-11 | C3 types and the `IdentityProvider` trait |
| `http.rs` | CT-11 | `provider_client() -> reqwest::Client` (10 s, no redirects) and `read_capped(resp, 64 * 1024)` |
| `config.rs` | CT-11 | `IdentityConfig`, `ProviderConfig`, `Limits`, `Links`, `KillSwitches`, `ConfigError`, `load(text, env, developer_mode)` |
| `providers/mod.rs`, `providers/dev_mock.rs` | CT-12 | `build(cfg, http) -> Providers`; `DevMockProvider` with `issue(VerifiedIdentity) -> String` |
| `providers/discord.rs` | CT-12 | `DiscordProvider` |
| `providers/oidc.rs`, `providers/oidc_keys.rs` | CT-13 | `OidcProvider`, discovery and JWKS cache |
| `link.rs`, `names.rs`, `limits.rs` | CT-14 | C7 |
| `repo/mod.rs`, `repo/identity.rs`, `repo/account.rs`, `repo/event.rs` | CT-15 | identity, account and event queries |
| `repo/refresh.rs` | CT-16 | refresh-token rotation and reuse detection |
| `stores.rs`, `link_code.rs` | CT-23 | in-memory pending, handoff, access-token and link-grant stores; `LinkCodeStore` |
| `routes/mod.rs`, `routes/error.rs`, `routes/ctx.rs`, `routes/signin.rs` | CT-24 | `router(deps, ctx)`, `ApiError`, `IdentityCtx`, providers, authorize, callback, dev-mock |
| `routes/token.rs` | CT-25 | token (both grants), the `signed_in` helper, logout |
| `routes/link.rs` | CT-26 | link/new, link/existing |
| `routes/bearer.rs`, `routes/account.rs`, `routes/ticket.rs` | CT-30 | the `Bearer` extractor; account, identities, link/start; launch-ticket, heartbeat, end |
| `telemetry.rs`, `wiring.rs` | CT-31 | span helpers; `install(auth: &mut AuthService, developer_mode)` (and the test-only `install_with(path, ..)`) called by `crates/server/src/login_routes.rs` |

Reaping and purging (`wiring.rs`, `repo/refresh.rs`): `run_reaper` ticks every 60 s (`REAP_EVERY`) and `reap_once` clears the in-memory stores, the link-code store and the rate-limiter windows (`RateLimiter::reap`, which uses the same stale-window test as `check`). The refresh table has its own cleanup: `purge_expired` deletes the rows of each family past `family_expires_at` or idle longer than 30 days (a live family keeps its spent rows, so reuse detection still works), and `run_purge` calls it hourly (`PURGE_EVERY`) once the pool is known, from the `set_identity_routes` closure; a failed run logs `refresh_purge_failed` and no row data. The launch-ticket store is reaped by the auth service itself (`service.rs`).: one variant per C6 error code, plus `Unavailable` = `503 {"error":"unavailable"}` when `AuthRouteDeps.db` is `None`, and `RateLimited { retry_after }`. Every store, limiter and repository function takes `now` (an `Instant` or a `chrono::DateTime<Utc>`) as an argument; only route handlers read the clock.

## C13. Launcher sign-in bridge (`sign_in_command`)

One Tauri command, `sign_in_command`, carries every sign-in action between the frontend and the engine. The engine owns the types (`crates/launcher/desktop/engine/src/sign_in/commands.rs`, CT-40); the shell only forwards (CT-41); the frontend decodes them (`frontend/src/sign-in-contract.ts`, CT-42).

Request: `invoke('sign_in_command', {request})`. Rust: `#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)] pub enum SignInCommand`. Every variant has `schema_version: u32`; anything but `1` fails with `unsupported_schema`.

| `command` | Other fields | Does |
|---|---|---|
| `inspect` | - | probes the sign-in server (cached), restores a stored sign-in, reports |
| `begin` | `provider: String` | opens the browser and waits (up to 5 min) for the sign-in to finish |
| `cancel` | - | ends a waiting `begin` or `add_method` with `cancelled`; always succeeds |
| `link_new` | - | creates a new account for the pending `needs_link` identity |
| `link_existing_password` | `account_name: String`, `password: String`, `acknowledge_password_removal: bool` | links the pending identity to an existing account and removes its password; `false` fails with `password_removal_not_acknowledged` before any request |
| `link_existing_code` | `link_code: String`, `acknowledge_password_removal: bool` | links the pending identity with a GM link code; same ack rule |
| `account` | - | reads the account view |
| `add_method` | `provider: String` | browser flow through `link/start`, then the account view |
| `unlink` | `identity_id: i64` | removes one identity, then the account view |
| `sign_out` | - | revokes and forgets the stored sign-in; works offline |

Success, `SignInStatus` (every command):

```json
{"schema_version": 1,
 "available": true,
 "server_name": "Cimmeria",
 "signup_open": true,
 "providers": [{"id": "discord", "display_name": "Discord"}],
 "waiting": false,
 "state": {"state": "signed_out"},
 "account": null,
 "play_refusal": null}
```

- `play_refusal`: `null`, `"ticket_unavailable"` or `"sign_in_ended"`: why the last signed-in Play ended before the game started (C9, C15 d). The frontend texts: `ticket_unavailable` "Play stopped: could not get a sign-in for this launch. Check your connection and press Play again. If it keeps failing, sign out and sign in again."; `sign_in_ended` "Play stopped: your sign-in has ended. Sign in again, then press Play."

- `available`: the first login server (D-CT21) answered `GET /auth/v1/providers` with `api: 1`. When false, `providers` is empty and `state` is `signed_out`.
- `state` is one of `{"state":"signed_out"}`, `{"state":"needs_link","provider_display_name","identity_display_name","hint":"none"|"email_match","signup_open":bool}`, `{"state":"signed_in","account_name","display_name"}`.
- `account` is filled only by `account`, `add_method` and `unlink`: `{"account_name","display_name","identities":[{"identity_id","provider","display_name","created_at","last_used_at"}]}` (C6 `GET /account` minus `account_id`). `created_at` and `last_used_at` are integer epoch milliseconds (UTC); CT-40 adds `created_at: i64` and `last_used_at: i64` to `IdentityView` in `crates/launcher/desktop/engine/src/sign_in/api.rs`, which has neither field today.
- `waiting`: a `begin` or `add_method` is waiting for the browser.

Failure, `SignInFailure`: `#[serde(tag = "error", rename_all = "snake_case")]`:

| `error` | Extra fields | Meaning |
|---|---|---|
| `unsupported_schema` | - | `schema_version` is not 1 |
| `unavailable` | - | the first login server offers no sign-in |
| `busy` | - | another `begin` or `add_method` is waiting |
| `browser_failed` | `url: String` | the system browser could not be opened; the UI shows the address to copy |
| `offline` | - | the server could not be reached |
| `cancelled`, `denied`, `timed_out` | - | the browser step ended that way |
| `no_pending_link` | - | a `link_*` command with no live `needs_link` (10 min) |
| `password_removal_not_acknowledged` | - | a `link_existing_*` command with `acknowledge_password_removal: false` |
| `server` | `code: String`, `retry_after: Option<u64>` | a C6 error code (`invalid_credentials`, `not_linkable`, `signup_closed`, `rate_limited`, `reauth_required`, `last_method`, `account_disabled`, `signed_out`, `unknown_provider`, ...) |
| `protocol` | - | the server answered something the launcher cannot read |
| `io` | - | local state could not be read |

No request field, status or failure is ever logged; the password never leaves the engine except in the `link/existing` body over TLS.

## C14. Wave 5 server addition: password algo 3

### Algo 3, `sha1_argon2id` (CT-54, D-CT10)

- `account.password_algo = 3`: `password_hash_v2` holds the argon2id PHC string (the existing 64 MiB, t=3, p=1 parameters) of the **uppercase `SHA1HEX(password)`**, the 40 ASCII characters the unmodified client sends; `password` is NULL. No schema change (`password_algo` is a plain `smallint`).
- Verification: `LegacySha1Hex(h)` verifies `h.to_ascii_uppercase()`; `Plaintext(pw)` verifies `sha1_upper_hex(pw)`. Both work over plain HTTP and TLS exactly as the credential gate already allows. The link-form check (`password_check.rs` `check_row`) verifies an algo-3 row as it would `Plaintext(pw)`, `sha1_upper_hex(pw)` against `password_hash_v2`, and never migrates; a NULL hash spends the dummy verify and is refused. The `ALGO_SHA1_ARGON2ID = 3` constant already exists (wave 10).
- Migration on a verified login (best effort; a failed UPDATE never fails the login): algo 1 with either credential, and algo 2 with `Plaintext`, rewrite the row to algo 3. Algo 2 with `LegacySha1Hex` stays refused (the plaintext is unknown); its next plaintext login moves it to algo 3.
- Every argon2id hash and verify runs in `tokio::task::spawn_blocking` behind a `tokio::sync::Semaphore`. There are two gates (D-CT30): the SOAP credential path (`credentials.rs`, through `password_hash.rs` `hash_blocking` and `verify_blocking`) shares one of `ARGON2_PERMITS = 4`; the link-form check (`password_check.rs`, CT-99, D-CT28) has its own of 2. Neither path takes the other's permit, so a `/link/existing` flood never queues a game login; together at most 6 verifies (about 384 MiB) run at once.
- The algo-1 hex compare becomes constant time (`subtle::ConstantTimeEq` on the uppercase bytes).
- Kill switch: `CIMMERIA_PASSWORD_REHASH_DISABLED=1` stops the migration UPDATE; algo-3 rows still verify. There is no way back from algo 3 to algo 1: fix forward rather than revert CT-54 once it ran. Today only seed accounts carry passwords, so no player data is at stake.
- Algo 3 applies only to accounts without an identity (seed and other non-OIDC accounts). A rehash UPDATE that the C5 guard refuses (`account_has_identity`) is logged with `reason=account_has_identity` and the login still succeeds.
- Log fields: `account_id`, `from_algo`, `to_algo`, `reason`. Never the name, the hash or the credential.

### Game password: cancelled

The opt-in game password (`POST /password`, `DELETE /password`, the launcher's `set_password` and `clear_password`; CT-55, CT-59) is cancelled by D-CT11 (2026-10-24). No packet adds a route or command that sets a password.

## C15. No passwords on linked accounts (D-CT11, decided 2026-10-24)

**Rule:** an account with any `account_identity` row never has, and never uses, a password (`password` and `password_hash_v2` are both NULL). Only accounts without an identity (today: the seed users) have passwords.

- **(a) Linking clears the password.** `link/existing` (both forms) runs in one transaction: lock the account row (`FOR UPDATE`), set `password` and `password_hash_v2` to NULL and `password_algo` to 1 when either is set, write event `password_cleared`, insert the identity, write event `link`. This cannot be undone. The launcher shows the warning in CT-42 and sends `password_removal_ack: true`. A body without it gets `400 invalid_request` and changes nothing. After the link, the unmodified-client password login for that account stops (error 4), by design.
- **(b) GM link codes follow the same rule.** The `link_code` form clears the password exactly as (a). `.account linkcode` warns the GM in its output (CT-45).
- **(c) Nothing sets a password on a linked account.** The C5 guard triggers refuse it at the database (`account_has_identity`, `account_has_password`, SQLSTATE `23514`), whatever the path: admin tooling, a GM command, a seed reload, a manual `UPDATE`, or the CT-54 rehash. No admin or GM password setter exists today; any future one maps the refusal to a clear error. A seed reload is safe: `account` is a persistent table, and `db/persistent/seed-accounts.sql` inserts only missing seed accounts and never touches existing rows.
- **(d) Play never falls back to a password.** Signed in, a failed ticket mint ends the launch with a launcher error (C9). That covers every way to a missing ticket: the discovery probe fails, `soap_relay` is false, the relay does not start, or `/launch-ticket` refuses. Relay-password mode and the plain list exist only for players who are not signed in to the first server's auth base (D-CT21).
- **(e) The last identity cannot be removed.** `last_method` counts identities only, so unlinking the last one gives `409 last_method` (C6, CT-30, CT-45). A player who loses access to every provider recovers through a GM: `.account linkcode <name>`, then `link/existing` with the code from a new provider sign-in.

## C16. Revocation model (PR #2031 review, waves 6 and 7; ruling 2026-10-26)

Every revocation (refresh reuse, logout, account disabled, player unlink, GM unlink) must also stop credentials that are **in flight** when it runs: a rotation that has not committed, a sign-in that has resolved its account but not issued its family, a route that has not minted its access token, a launch ticket being minted, a SID or BaseApp ticket already issued. Established game connections stay up (unchanged, C15 d). The model has six parts; a packet names the part it builds.

### C16.1 Database: one lock per refresh family (CT-71)

Locking the live rows first is not enough: a rotation of the successor C that starts between the lock statement and the revoke statement inserts a D that neither statement saw. So every operation on an existing family takes one transaction-scoped advisory lock for that family first; a statement after it sees everything a rotation of the family committed, and no rotation of the family can run until the revoke commits.

```rust
// crates/identity/src/repo/refresh.rs
async fn family_lock(conn: &mut PgConnection, family_id: &str) -> Result<(), RepoError>;
//   SELECT pg_advisory_xact_lock(hashtextextended('cimmeria.refresh_family:' || $1, 0))
// rotate(pool, presented, now), same signature and Rotation variants:
//   BEGIN; SELECT family_id FROM auth_refresh_token WHERE token_hash = $1   -- no row: Unknown
//   family_lock(family_id); then today's SELECT .. FOR UPDATE row read and today's branches
async fn revoke_family_on(conn: &mut PgConnection, family_id: &str, now_ts: &str) -> Result<(), RepoError>;
//   family_lock(family_id); UPDATE auth_refresh_token SET revoked_at = $2::timestamptz
//     WHERE family_id = $1 AND revoked_at IS NULL
pub async fn revoke_account_on(
    conn: &mut PgConnection,
    account_id: u32,
    keep_family: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Vec<String>, RepoError>;
//   1. SELECT DISTINCT family_id FROM auth_refresh_token WHERE account_id = $1
//        AND ($2::text IS NULL OR family_id <> $2) AND revoked_at IS NULL ORDER BY family_id
//   2. family_lock(f) for each, in that order
//   3. UPDATE .. SET revoked_at = $3::timestamptz WHERE <same predicate> RETURNING family_id
//      (deduplicated, sorted)
```

`revoke_family` becomes `begin; revoke_family_on; commit`. `revoke_account` and `revoke_account_except` become `begin; revoke_account_on(.., None | Some(keep), ..); commit` and keep their signatures. One family lock per rotation and sorted order for several families means no lock cycle. A family whose first row is inserted after step 1 is a new sign-in, which C16.2 binds to the account row. The key prefix `cimmeria.refresh_family:` keeps the lock apart from the account-create lock (`pg_advisory_xact_lock(2000201)`).

### C16.2 Database: a new family is bound to the identity that earned it (CT-73)

A new refresh family is issued only in a transaction that holds the account row `FOR SHARE` and rechecks the identity binding. Unlink holds the account `FOR UPDATE` across its delete (C15 e), so issuance waits for the unlink to commit, then sees the identity gone.

```rust
// crates/identity/src/repo/refresh.rs
pub async fn issue_on(conn: &mut PgConnection, account_id: u32, family: Option<&str>, now: DateTime<Utc>)
    -> Result<(String, String), RepoError>;          // today's `issue` body, on a connection
pub async fn issue(pool: &PgPool, account_id: u32, family: Option<&str>, now: DateTime<Utc>)
    -> Result<(String, String), RepoError>;          // unchanged signature; acquires a connection, calls issue_on
pub enum Issued { Pair(String, String), Disabled, Unbound }   // Pair = (refresh_token, family_id)
pub async fn issue_bound(pool: &PgPool, account_id: u32, identity_id: i64, now: DateTime<Utc>)
    -> Result<Issued, RepoError>;
//   BEGIN
//   SELECT enabled FROM account WHERE account_id = $1 FOR SHARE      -- no row: Unbound
//   enabled = false: Disabled (rollback)
//   SELECT EXISTS (SELECT 1 FROM account_identity WHERE identity_id = $2 AND account_id = $1)  -- false: Unbound
//   issue_on(tx, account_id, None, now); COMMIT; Pair
```

`routes::token::signed_in` becomes `signed_in(state, account_id: u32, identity_id: i64, fence: u64, via_code: bool, now: Instant)`: it calls `issue_bound`; `Disabled` is `ApiError::AccountDisabled`, `Unbound` is `ApiError::InvalidGrant`. Its callers pass the identity that resolved the sign-in: `Resolution::Existing` the `find_bound` id, `AutoLink` and the two link routes `linked.identity_id`. `attach` (C16.5) does its own transaction and calls `issue_on`.

### C16.3 Memory: access-token fences (CT-72)

`AccessTokens` moves from `stores.rs` to a new `crates/identity/src/access_tokens.rs` (re-exported from `stores`, so `crate::stores::{AccessTokens, AccessGrant}` paths keep working). One `Mutex` holds:

```rust
struct Inner {
    seq: u64,                                        // bumped by every revoke_*
    by_hash: HashMap<[u8; 32], AccessGrant>,        // as today
    family_revoked: HashMap<String, (u64, Instant)>, // family -> (seq of the revoke, when)
    account_revoked: HashMap<u32, AccountTomb>,
}
struct AccountTomb { seq_all: u64, seq_except: u64, keep: Option<String>, at: Instant }
pub struct AccessGrant { /* today's fields */ pub minted_seq: u64 }

pub fn fence(&self) -> u64;                                   // the current seq
pub fn mint(&self, account_id: u32, family_id: &str, via_code: bool, now: Instant) -> String;   // unfenced: tests and tools only
pub fn mint_fenced(&self, account_id: u32, family_id: &str, via_code: bool, now: Instant, fence: u64) -> Option<String>;
pub fn grant_live(&self, grant: &AccessGrant) -> bool;        // the same predicate with fence = grant.minted_seq
```

`revoked_since(account, family, fence)` is true when `family_revoked[family].0 > fence`, or `account_revoked[account].seq_all > fence`, or `seq_except > fence` and `keep != Some(family)`. `mint_fenced` returns `None` when it is true; `mint` and `mint_fenced` record `minted_seq = seq` at mint. `revoke_family` and `revoke_account` bump `seq` and set `family_revoked` / `seq_all`; `revoke_account_except(account, keep)` bumps `seq` and sets `seq_except` and `keep`, and when an older except-tombstone had a different `keep` first raises `seq_all` to that old `seq_except` (so the older keep loses its exemption). Tombstones live `ACCESS_LIFE` and are pruned by `reap`; `make_room` counts them against `STORE_CAP` and evicts the oldest. An evicted tombstone only reopens the race window it covered (accepted). Signatures of the `revoke_*` methods do not change.

Routes capture `let fence = state.ctx.stores.access.fence();` at entry, before any await: `token()` (passed to `code_grant` and `refresh_grant`), `link::create` and `link::link_existing`. `session()` calls `mint_fenced`; `None` answers `ApiError::InvalidGrant` and best-effort revokes the just-issued family with `refresh::revoke_family` (a failure is a WARN with `reason = "fence_cleanup_failed"`, never the token). No route calls the unfenced `mint`.

### C16.4 Memory: launch-ticket guards (CT-77, CT-78, CT-79)

```rust
// crates/auth/src/auth/launch_ticket.rs, re-exported from crates/auth/src/auth/mod.rs
#[derive(Clone, Debug)]
pub struct TicketGuard(Arc<AtomicBool>);
impl TicketGuard { pub fn is_current(&self) -> bool; }
pub enum TicketCheck { NoMatch, Valid { account_id: u32, guard: TicketGuard } }   // drops Copy, PartialEq, Eq

impl LaunchTicketStore {
    pub fn guard(&self, account_id: u32) -> TicketGuard;        // a fresh guard registered under the account
    pub fn mint(&self, account_id: u32, name: &str, launch_id: &str, now: Instant) -> MintedTicket;   // unchanged; takes its own guard
    pub fn mint_guarded(&self, account_id: u32, name: &str, launch_id: &str, now: Instant, guard: &TicketGuard)
        -> Option<MintedTicket>;                                 // None when !guard.is_current()
}
```

The store keeps `guards: HashMap<u32, Vec<Weak<AtomicBool>>>` beside the entries (one lock, or entries then guards, always in that order). `revoke_account(id)` removes the entry, stores `false` into every guard of the account and drops the list. `end` and expiry do **not** flip guards. `reap` drops dead `Weak`s. `verify` compares the digest against **every** entry whose folded name matches (constant-time each; F13b) and answers the one that matches, or `NoMatch`; a matching entry whose guard is not current is `NoMatch` and is removed.

Order in `revoke::end_in`, load-bearing: access tokens first, then launch tickets. The ticket route (`routes/ticket.rs` `mint_ticket`), after the Bearer check: `let guard = launch_tickets.guard(account_id);` then `if !access.grant_live(&grant) { InvalidToken }`, then the DB reads, then `mint_guarded(.., &guard)`, `None` answering `InvalidToken`. Either the guard predates the ticket revoke (and is flipped) or the access revoke predates `grant_live` (and it fails).

Handoffs carry the guard: `SessionRecord.ticket_guard: Option<TicketGuard>` and `PendingLogin.ticket_guard: Option<TicketGuard>` (`None` for password logins). Checked: in UserAuth after the `ticket_account` await, before the SID insert (refuse as `InvalidCredentials`, `reason = "ticket_revoked"`); in ServerSelection after the SID is removed (as `sid_unknown`, `reason = "ticket_revoked"`); at BaseApp admission in `handle_login_with_issuer` after the age check (burn the pending login, WARN `reason = "ticket_revoked"`, the same refusal as an expired ticket).

### C16.5 Add method carries its family (CT-75)

`LinkIntent { account_id: u32, family_id: String }`: `/link/start` copies `grant.family_id`. `attach` runs one transaction: `SELECT enabled, accesslevel, test_account FROM account WHERE account_id = $1 FOR UPDATE` (no row: `InvalidGrant`; disabled: `AccountDisabled`; `!link::linkable(&facts)`: `NotLinkable`); `SELECT EXISTS (SELECT 1 FROM auth_refresh_token WHERE family_id = $1 AND account_id = $2 AND revoked_at IS NULL AND family_expires_at > now())` (false: `InvalidGrant`); `find_bound` on the transaction (bound elsewhere: `InvalidGrant`); `identity::insert_on(&mut tx, ..)` when unbound; `issue_on(&mut tx, account_id, None, ..)`; commit; then `session()` with the route's fence. `identity::insert` becomes a wrapper over `insert_on(conn: &mut PgConnection, account_id, &VerifiedIdentity, store_email) -> Result<Linked, RepoError>`.

### C16.6 Unlink revokes inside its transaction; reuse revokes before it audits (CT-74, CT-76)

Both `remove_identity` functions (player `routes/account.rs`, GM `base-session/.../account_admin/mod.rs`) call `refresh::revoke_account_on(&mut tx, account_id, keep, now)` after the delete and before `commit` (player: `keep = Some(&grant.family_id)`; GM: `None`). After commit they end the in-memory sessions (player: `access.revoke_account_except` then `launch_tickets.revoke_account`; GM: `revoke::end_live_sessions`). A failed revoke rolls the unlink back: the GM reply becomes `account: could not remove identity #<id> from <target>; nothing changed, see the server log`. `Rotation::Reused` runs `access.revoke_family(&family_id)` and `launch_tickets.revoke_account(account_id)` **before** `event::record`, whose failure is then a WARN, not a skipped revoke.

## C17. Launcher host sessions (wave 9; D-CT25)

### C17.1 One slot per auth host (CT-86)

```rust
// crates/launcher/desktop/engine/src/sign_in/host_slot.rs (new; `pub use` from session.rs)
pub struct HostSlot { state: tokio::sync::Mutex<HostState> }
struct HostState {
    access: Option<(String, Instant)>,
    account: Option<AccountSummary>,
    session_only: Option<Stored>,   // CT-88: a sign-in the store could not keep
    tombstone: bool,                // CT-88: a delete the store could not do
    warning: Option<&'static str>,  // CT-88: the C18.1 storage_warning value
}
impl SignIn {
    // unchanged signatures; each makes a private slot (tests and one-off use)
    pub fn new(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>) -> Self;
    pub fn restore(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>) -> Option<Self>;
    // the driver's constructors: the shared slot of the host
    pub fn new_in(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>, slot: Arc<HostSlot>) -> Self;
    pub async fn restore_in(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>, slot: Arc<HostSlot>) -> Option<Self>;
}
// driver.rs: SignInDriver gains `hosts: Mutex<HashMap<String, Arc<HostSlot>>>` and
// fn slot(&self, base: &Url) -> Arc<HostSlot>   // keyed by host_of(base); created on first use, never removed
```

Every `SignIn` the driver makes for one host shares one slot, so a game's retained `Arc<SignIn>` and the driver's current one serialize on the same lock. Every store read and write happens under the slot lock. `restore_in` locks the slot and reads the store: nothing stored is `None`; a stored entry sets the slot's `account` when it is empty (an access token already in the slot is kept) and answers a `SignIn` on that slot. The driver calls only `new_in` and `restore_in`, so `session_for` becomes `async`; no `DesktopState` guard is held across its await. `sign_out` locks the slot, reads the stored refresh token, forgets (store and memory), releases the lock, then posts `/logout`.

### C17.2 Operation epoch (CT-87)

`SignInDriver.epoch: AtomicU64`. Each `run` captures `let epoch = self.epoch.load(SeqCst)` at entry. `sign_out` and `cancel` bump it **while holding the `DesktopState` lock**; `sign_out` also calls `cancel_flow`. `SignIn::complete`, `link_new`, `link_existing_password` and `link_existing_code` take `fence: &Epoch` (`pub struct Epoch<'a> { counter: &'a AtomicU64, expected: u64 }` with `is_current()`); `adopt_signed_in` checks it under the slot lock before writing the store. A stale epoch skips the store write, posts `/logout` with the refresh token just received (best effort), and returns `SignInError::Superseded` (`reason()` `"Sign-in was cancelled."`), which the driver maps to `SignInFailure::Cancelled`. `signed_in` and `finish` check the epoch again under the `DesktopState` lock before `set_sign_in` / `keep_pending`; a stale epoch answers `Err(Cancelled)`. Refreshes (`access_token`) take no epoch: `sign_out` waits for the slot lock.

A server switch is not a supersede and does not bump the epoch. A completion for the old base is still stored under its own host (the player finished it there), but `signed_in` and `finish` install it, or keep its pending choice, only when `discovery::auth_base` of the current `sign_in_login_server` still equals `session.base()`; otherwise they answer the reply the current state gives.

### C17.3 Fallible credential store (CT-88, D-CT25)

```rust
pub enum StoreError { Unavailable, Io }      // no secret, no host in Display
pub trait CredentialStore: Send + Sync {
    fn get(&self, host: &str) -> Result<Option<Stored>, StoreError>;   // Ok(None) = nothing stored
    fn set(&self, host: &str, stored: &Stored) -> Result<(), StoreError>;
    fn delete(&self, host: &str) -> Result<(), StoreError>;            // deleting nothing is Ok
}
```

- `set` fails: the slot keeps the entry in `session_only` and the session works until the launcher exits; the status gets `storage_warning: "session_only"`.
- `delete` fails: the slot sets `tombstone` (the process treats the host as signed out and never restores it), `session_only` is cleared, and the status gets `storage_warning: "sign_out_incomplete"`.
- `get` fails: nothing is restored, and the status gets `storage_warning: "store_unreadable"`. Sign-in still works session-only.
- Reads prefer `session_only`, then the store; a tombstone answers `None` before either.
- The driver's `Site` gains `base: Option<&Url>` (the target's auth base), so `reply` reads the slot's warning even when no session is installed.

## C18. Launcher sign-in additions and OIDC rules (waves 8 and 9)

### C18.1 `SignInStatus` and commands (additive; `schema_version` stays 1)

- New field `storage_warning: null | "session_only" | "sign_out_incomplete" | "store_unreadable"` (CT-88; the frontend texts are in CT-91).
- New command `resume` (no other fields; CT-89). A `begin` or `add_method` whose browser would not open now **parks** its flow (listener, verifier, cancel handle, epoch and what it continues: a plain sign-in for `provider` at `base`, or an Add method on the current session), keeps `waiting: true`, and fails with `browser_failed {url}` as today. `resume` awaits that parked flow and completes it as the original command would have; with nothing parked it fails `no_pending_flow` (new `SignInFailure` code). `cancel` and the flow's own 5-minute limit clear the parked flow. The frontend sends `resume` at once after showing the URL.
- `available: false` may now carry `state: signed_in` (an offline restore, ruling CT-35-1). The panel keeps the account summary and **Sign out** visible then, and hides only network actions (CT-91).
- In `needs_link`, the panel offers **Back to sign in**, which sends `sign_out` (CT-91).

### C18.2 Which server sign-in uses (CT-90, D-CT21 amended)

`DesktopState::sign_in_login_server(&self) -> Result<Option<LoginServer>, StorageError>` answers the first server of the **effective** list, the list Play writes: the adopted list of a legacy-adopted installation, else saved preferences, else the defaults. Both it and `launch_login_servers` call one helper, `fn effective_login_servers(&self) -> Result<Vec<LoginServer>, StorageError>`. An error reading the installation never falls back to another list. That covers `Busy` while an install, update or repair of the installed copy runs, and `PersistenceUncertain`. In that case `inspect` and `cancel` answer the status of the sign-in the launcher holds, with `available: false` and no `server_name`, as the offline reply does. `sign_out` signs out with no server. Every other command answers `unavailable`.

### C18.3 OIDC token-endpoint authentication and claims (CT-83, CT-84; D-CT24)

- Provider setting, `kind = "oidc"` only (Google is configured as one): `token_endpoint_auth_method = "client_secret_basic" | "client_secret_post"`, default `client_secret_basic` (the OIDC Discovery default). Any other value is a config error at startup. `basic` sends `Authorization: Basic base64(form_urlencode(client_id) ":" form_urlencode(client_secret))` (RFC 6749 2.3.1) and keeps `client_id` but no secret in the form; `post` sends `client_id` and `client_secret` in the form, as today. The Discord and dev-mock providers ignore the key and keep their own method.
- Discovery reads `token_endpoint_auth_methods_supported` (optional). When it is present and lacks the configured method, `prepare` (which runs before the browser opens) fails with `ProviderError::BadResponse("auth_method_unsupported")` and a WARN naming the provider id and the method. Absent metadata means "basic only" per Discovery 1.0 section 3, but is **not** refused: a provider that omits it and accepts `post` stays usable when configured so.
- ID token claims (OIDC Core 3.1.3.7, D-CT24): `iat` is required (`Claims.iat: Option<u64>` checked explicitly, because `jsonwebtoken`'s required-claims list ignores names it does not validate; a missing `iat` is `Token("missing_claim")`) and refused when more than 60 s after now (`ProviderError::Token("iat_future")`); `azp` is required when `aud` has more than one value (`Token("azp_missing")`); `azp`, when present, must equal the client id (`Token("azp_mismatch")`). The reason string is the log field; the token is never logged. `nbf` is validated too (`Validation::validate_nbf = true`, the same 60 s leeway as `exp`); a token more than 60 s before its `nbf` is `Token("not_yet_valid")` (consolidated ruling C09 / A-L3).
