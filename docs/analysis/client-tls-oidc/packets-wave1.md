# client-tls-oidc wave 1: server foundations

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 1
> **Last updated:** 2026-10-24
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract](contract.md), [ledger](README.md)

Seven packets, one PR from `client-tls-oidc/wave1` (worktree `cto-w1`), based on main after wave 0 merged. Merge order: CT-10, CT-11, CT-14, CT-12, CT-13, CT-15, CT-16. CT-10, CT-11 and CT-14 start at once from main (CT-14 creates its files only inside the crate CT-11 adds, so `ops` branches CT-14 at CT-11's reviewed head); CT-12, CT-13, CT-15 and CT-16 branch at the wave head after CT-11 (and, for CT-15 and CT-16, CT-10) merged in. Every packet carries R1-R10 from [work-packets.md](work-packets.md#rules-every-packet-carries).

## CT-10 Schema: identity tables

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-10-schema` · **Worktree:** `cto-10`
- **Commit:** `feat(db): account identity, refresh token and identity event tables (CT-10)`
- **Rules:** R1-R9. Contract: [C5](contract.md#c5-schema-additive-persistent), C15 (D-CT11 guards).

Files:

1. New `db/sgw/Accounts/Tables/account_identity.sql`, `auth_refresh_token.sql`, `account_identity_event.sql`: the C5 DDL, one table and its indexes per file, in the house style of `db/sgw/Accounts/Tables/account.sql` (read it first: header comment, `SET` lines if any, ownership lines if any). LF or CRLF: match `account.sql`.
2. `db/database.sql`: three `\i` lines (same form as the `account.sql` line) right after the `account.sql` include.
3. `db/sgw/_foreign_keys.sql`: the three C5 `ALTER TABLE` lines, next to `sgw_player_account_id_fkey`.
4. `db/persistent/persistent-public-tables.txt`: add the three table names in the file's order (alphabetical if it is sorted).
5. `db/sgw/Audit/Tables/login_audit.sql`: the `outcome` CHECK list (l.15) gains `'launch_ticket'` and `'ticket_requires_tls'`.
6. `db/sgw/_functions.sql` and `db/sgw/_triggers.sql`: the two D-CT11 guard functions and triggers from C5, after the existing `org_account_before_delete` entries, in the files' existing style (one comment line above each saying it enforces "no password on an account with a linked identity").
7. New `crates/auth/src/auth/identity_schema_live_db.rs`, registered in `crates/auth/src/auth/mod.rs` as `#[cfg(test)] mod identity_schema_live_db;` next to the other test modules.

Tests (all `live_db`, `require_db_or_skip!`, sentinel account ids `2_000_101..=2_000_107`, cleanup by exact id before and after):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_identity_subject_is_unique` | a second `account_identity` row with the same `(provider, issuer, subject)` fails with a unique violation | the constraint is new |
| `live_db_identity_rows_cascade_with_the_account` | deleting the sentinel account removes its identity, refresh-token and event rows | the FKs are new |
| `live_db_identity_event_kind_is_checked` | kind `'bogus'` is refused; `'link_code_issued'` is accepted | the CHECK is new |
| `live_db_login_audit_accepts_ticket_outcomes` | inserting a `login_audit` row with each new outcome succeeds (copy the insert shape from the existing audit live-DB test: grep `login_audit` under `crates/auth/src`) | the CHECK list is new |
| `live_db_password_refused_on_an_account_with_an_identity` | an account with an identity row: `UPDATE account SET password_hash_v2 = 'x'` fails with SQLSTATE `23514` and message `account_has_identity`; setting both columns to NULL succeeds | D-CT11 (c): the guard is new |
| `live_db_identity_refused_while_the_account_has_a_password` | inserting an identity for an account whose `password` is set fails with `account_has_password` | D-CT11 (a): link must clear first |
| `live_db_clear_then_link_in_one_transaction` | in one transaction: NULL both password columns, then insert the identity; commit succeeds and the row has no password | the CT-15 link path works under the guards |

Checks, in order: S1 `cimmeria-auth`, S3 `identity_schema_live_db`. If a test or script checks `persistent-public-tables.txt` against the schema (grep the file name under `tools/` and `crates/`), run it and name it in the report.

## CT-11 The `cimmeria-identity` crate: skeleton, types, config

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-11-identity-crate` · **Worktree:** `cto-11`
- **Commit:** `feat(identity): new cimmeria-identity crate with provider types and config (CT-11)`
- **Rules:** R1-R10. Contract: [C3](contract.md#c3-identity-crate-cratesidentity-package-cimmeria-identity), [C4](contract.md#c4-provider-config-configauth-providerstoml), [C12](contract.md#c12-identity-crate-module-map).

Files:

1. `crates/identity/Cargo.toml`: package `cimmeria-identity`, edition and lints copied from `crates/auth/Cargo.toml`; dependencies from the workspace: `cimmeria-auth`, `cimmeria-common`, `serde`, `toml`, `thiserror`, `tracing`, `reqwest` (workspace 0.13 with `rustls` and `json`; copy the feature set another server crate uses), `async-trait`, `url`; `cimmeria-workspace-hack`. Dev: `tokio` (macros, rt).
2. Root `Cargo.toml`: `"crates/identity"` in `members` (and `default-members` if every other server crate is there), and `cimmeria-identity = { path = "crates/identity" }` in `[workspace.dependencies]` beside `cimmeria-auth`.
3. `crates/identity/src/lib.rs`: crate doc (what the crate is, that it depends on `cimmeria-auth` and never the reverse, a link to the ledger), `pub mod provider; pub mod http; pub mod config;`.
4. `provider.rs`: the C3 types exactly. `ProviderId::new(&str) -> Option<ProviderId>` enforces `^[a-z0-9_-]{1,32}$` without a regex crate. `VerifiedIdentity::sanitized_display_name(raw: &str) -> String` keeps printable ASCII, trims, cuts to 64, falls back to `"Player"` when empty. `impl ProviderError { pub fn reason(&self) -> &'static str }` (`denied`, `timeout`, `http_<class>` as `http_4xx`/`http_5xx`, the `BadResponse`/`Token` string).
5. `http.rs`: `provider_client() -> reqwest::Client` (10 s timeout, `redirect::Policy::none()`, rustls) and `async fn read_capped(resp: reqwest::Response, cap: usize) -> Result<Vec<u8>, ProviderError>` (`BadResponse("too_large")` past the cap; reads chunks, never `bytes()` whole).
6. `config.rs`: C4. `pub fn load(text: &str, env: &dyn Fn(&str) -> Option<String>, developer_mode: bool) -> Result<(IdentityConfig, Vec<ConfigWarning>), ConfigError>` and `pub fn load_file(path, developer_mode)` (missing file: `Ok` with no providers). `IdentityConfig { public_base_url, signup_open, limits: Limits, links: Links, providers: Vec<ProviderConfig>, kill: KillSwitches }`; `ProviderConfig { id, kind, display_name, client_id, client_secret: Secret, issuer: Option<String>, scopes: Vec<String>, trusted_email }`; `Secret(String)` with a `Debug` that prints `Secret(***)`. `KillSwitches::from_env(env)` reads the three C4 env names (`"1"` means on). `CIMMERIA_IDP_SIGNUP_DISABLED=1` forces `signup_open = false`; `CIMMERIA_IDP_DISABLED=1` empties `providers`. The `literal_secret` check scans the parsed table for any key named `client_secret`. Default `scopes`: `["openid","email","profile"]` for `oidc`, ignored for the others.
7. `config/auth-providers.toml.example`: the C4 example text with a header comment: what the file is, that the real file is `config/auth-providers.toml` (gitignored), that secrets go only in the named env variables, how to register the Discord redirect `{public_base_url}/auth/v1/callback/discord`, and the Microsoft warning. `.gitignore`: add `config/auth-providers.toml`.
8. New-crate checklist (each is one or two lines; read the neighbouring entries for `cimmeria-auth` first):
   - `crates/server/src/logging/target_scan_tests.rs` `IN_PROCESS_CRATES`: add `"identity"`.
   - `crates/server/src/logging/filters.rs` `OTEL_FILTER`: add `cimmeria_identity=info,\` beside `cimmeria_auth=debug,\` (l.324). If a `FILE_LAYERS` entry routes `cimmeria_auth`, route `cimmeria_identity` to the same layer.
   - `tools/test-live-db.ps1` (l.27) and `tools/test-live-db.sh` (l.34): add `cimmeria-identity` after `cimmeria-auth`.
   - `tools/crate-graph/groups.toml` (l.39 group): add `"cimmeria-identity"`.
   - `crates/README.md`: one row for the crate. If the crate table is a `<!-- gen:... -->` block, run `python tools/docs-gen/regen.py` instead of editing it.
   - `cargo hakari generate` and `cargo hakari manage-deps --yes` (direct; hakari does not compile), then S4.
   - `crates/server/Cargo.toml` does **not** depend on the crate yet (CT-31 does), so the crate builds alone.

Tests (`config.rs` `mod tests`, plus `provider.rs` `mod tests`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `example_file_loads` | `load(include_str!("../../../config/auth-providers.toml.example"), env_with_both_secrets, false)` gives 2 providers, `signup_open`, limits 3/30/0, 3 links | the example must stay loadable |
| `missing_secret_skips_the_provider` | env without `CIMMERIA_IDP_GOOGLE_SECRET`: 1 provider, one `secret_missing` warning | new |
| `refuses_literal_secret_http_base_and_bad_ids` | each case gives its `ConfigError` variant | new |
| `refuses_trusted_microsoft_email` | issuer `https://login.microsoftonline.com/x/v2.0`, `trusted_email = true`: `microsoft_email_untrusted` | the residual-risk rule (D-CT3) |
| `dev_mock_needs_developer_mode` | skipped with `dev_mock_refused` when false; kept when true | new |
| `kill_switches_apply` | `CIMMERIA_IDP_DISABLED=1` empties providers; `CIMMERIA_IDP_SIGNUP_DISABLED=1` closes sign-up | new |
| `secret_debug_is_redacted` | `format!("{:?}", cfg)` does not contain the secret value | R7 |
| `provider_id_pattern` | accepts `discord`, `my-idp_2`; refuses `""`, `Discord`, 33 chars, `a.b` | new |
| `display_name_is_sanitized` | `"\u{1F600}"` gives `"Player"`; 80 ASCII chars gives 64 | new |

Checks, in order: S1 `cimmeria-identity`, S2 `cimmeria-identity`, S4, S2 `cimmeria-server` (the logging parity tests see the new crate).

## CT-14 Linking, account names and rate limits (pure)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-14-link-rules` · **Worktree:** `cto-14` (at CT-11's reviewed head)
- **Commit:** `feat(identity): account linking rules, generated names and rate limits (CT-14)`
- **Rules:** R1-R9. Contract: [C7](contract.md#c7-linking-sign-up-and-limits-pure-cratesidentitysrclinkrs-namesrs-limitsrs).

Files:

1. `crates/identity/src/link.rs`: C7 types and `resolve`, `linkable` exactly. Module doc states the three rules and the residual risk in one paragraph: auto-link trusts the provider's verified email; an attacker who controls a trusted provider account with the victim's verified email gets the victim's account, so only admin-trusted providers auto-link, and staff and test accounts never do.
2. `names.rs`: `pub fn base_from_display(display: &str) -> String`; `pub fn candidates(base: &str, rng: &mut impl rand::Rng) -> Vec<String>` (the 8 `{base}_{dddd}` names then one `player{dddddddd}`); `pub fn is_valid_account_name(s: &str) -> bool` (`^[A-Za-z0-9_-]{3,20}$`, no regex crate); `pub const RESERVED_PREFIXES: [&str; 10]`. Add `rand` (workspace) to the crate.
3. `limits.rs`: `pub struct RateLimiter` (a `Mutex<HashMap<(Bucket, String), Window>>`, fixed window per key, capped at 100,000 keys, oldest window dropped first), `pub enum Bucket { AnyRoute, Authorize, Token, LinkExistingIp, LinkExistingAccount, SignupIp, SignupGlobal }`, `pub fn check(&self, bucket: Bucket, key: &str, now: Instant) -> Result<(), RetryAfter>` (counts the request when allowed), `pub fn with_limits(limits: &config::Limits) -> RateLimiter` taking the two sign-up numbers from config and the rest from C7. `SignupGlobal` uses key `""`. `pub fn discord_created_ms(snowflake: u64) -> i64` and `pub fn old_enough(created_ms: Option<i64>, now_ms: i64, min_days: u32) -> bool` (`true` when `min_days == 0` or `created_ms` is `None`).
4. `lib.rs`: `pub mod link; pub mod names; pub mod limits;`.

Tests (in each module):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `bound_identity_signs_in_or_is_refused` | rule 1, both arms | new |
| `auto_links_one_trusted_verified_linkable_match` | rule 2 gives `AutoLink(id)` | new |
| `never_auto_links_staff_test_disabled_or_ambiguous` | accesslevel 1, `test_account`, disabled, two candidates, untrusted provider, unverified email: each `NeedsLink` | the rule the owner set (D-CT5 linking) |
| `hint_only_for_trusted_verified_matches` | `EmailMatch` vs `None` per rule 3 | new |
| `names_match_the_account_pattern` | 1,000 random display names (seeded rng) all pass `is_valid_account_name` | new |
| `reserved_and_short_names_become_player` | `GM_Bob`, `admin`, `x`, `"\u{1F600}"` give a `player_dddd` candidate | new |
| `limiter_counts_per_bucket_and_key` | 5 `LinkExistingIp` allowed, the 6th refused with a positive `retry_after`, a different key allowed, allowed again after 15 min | new |
| `limiter_caps_its_keys` | 100,001 keys leave at most 100,000 entries | new |
| `discord_snowflake_age` | `175928847299117063` gives `1462015105796` | the formula in C7 |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`.

## CT-12 Discord and dev-mock providers

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-12-discord` · **Worktree:** `cto-12` (wave head after CT-11)
- **Commit:** `feat(identity): Discord and developer mock identity providers (CT-12)`
- **Rules:** R1-R9. Contract: C3, C4, C12.

Files:

1. `crates/identity/src/providers/mod.rs`: `pub struct Providers { list: Vec<Arc<dyn IdentityProvider>>, dev_mock: Option<Arc<DevMockProvider>> }` with `get(&str)`, `list()`, `dev_mock()`; `pub fn build(cfg: &IdentityConfig, http: reqwest::Client) -> Providers` building one provider per `ProviderConfig` (the `Oidc` arm calls `oidc::OidcProvider::new`, which CT-13 adds; until then, the arm logs `WARN reason=oidc_unavailable` and skips; CT-13 replaces the arm).
2. `providers/discord.rs`: `DiscordProvider::new(cfg: &ProviderConfig, http)` and `with_base(cfg, http, base: Url)` for tests (default base `https://discord.com`).
   - `authorize_url`: `{base}/oauth2/authorize?response_type=code&client_id=..&scope=identify%20email&state=..&redirect_uri=..&code_challenge=..&code_challenge_method=S256&prompt=consent`, every value percent-encoded with `url::form_urlencoded`. `nonce` is unused (Discord has no id_token).
   - `exchange`: `POST {base}/api/oauth2/token` form `grant_type=authorization_code, code, redirect_uri, code_verifier, client_id, client_secret`; then `GET {base}/api/users/@me` with the bearer; both through `read_capped`. Map: `subject = id`, `display_name` = `global_name` or `username`, `email` lowercased when present, `email_verified = verified == Some(true)`, `issuer = "https://discord.com"`, `provider_account_created_ms = Some(discord_created_ms(id))`. Token 400/401 gives `Token("invalid_grant")`; other non-2xx `Http(status)`; JSON errors `BadResponse("json")`; timeout `Timeout`.
3. `providers/dev_mock.rs`: `DevMockProvider { base_url, codes: Mutex<HashMap<String, (VerifiedIdentity, Instant)>> }`; `issue(identity) -> String` (32 random bytes base64url, 60 s life); `authorize_url` gives `{public_base_url}/auth/v1/dev-mock/authorize?state=..&redirect_uri=..`; `exchange(code, ..)` removes and returns the identity or `Token("invalid_grant")`; `trusted_email` from config. Issuer `"dev-mock"`.
4. `lib.rs`: `pub mod providers;`. Cargo: `base64` (workspace), `sha2`; dev `wiremock` (workspace).

Tests (`wiremock` stubs; no real Discord):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `discord_authorize_url_carries_pkce_and_state` | parsed query has every parameter, encoded | new |
| `discord_exchange_maps_the_user` | stubbed token and `@me`: subject, display, lowercased email, verified, created ms | new |
| `discord_unverified_email_is_not_verified` | `verified: false` gives `email_verified == false` | auto-link safety |
| `discord_refusals_map_to_errors` | token 400 gives `Token("invalid_grant")`; `@me` 500 gives `Http(500)`; a 70 KiB body gives `BadResponse("too_large")` | new |
| `discord_secret_never_in_errors_or_debug` | `format!("{:?}", provider)` and every error string lack the secret | R7 |
| `dev_mock_codes_are_single_use` | `exchange` twice: `Ok` then `Err` | new |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`.

## CT-13 Generic OIDC provider

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-13-oidc` · **Worktree:** `cto-13` (wave head after CT-11; merge after CT-12)
- **Commit:** `feat(identity): generic OpenID Connect provider (CT-13)`
- **Rules:** R1-R9. Contract: C3, C4, C12.

Files:

1. `crates/identity/src/providers/oidc_keys.rs`: `Discovery { authorization_endpoint, token_endpoint, jwks_uri, issuer }` fetched from `{issuer}/.well-known/openid-configuration` (every endpoint must be `https://`, except under `#[cfg(test)]` where `http://127.0.0.1` is allowed; the returned `issuer` must equal the configured one byte for byte); `KeyCache` holding the JWKS (`jsonwebtoken::jwk::JwkSet`) for 1 h, refetched once when a token's `kid` is unknown (at most one refetch per minute).
2. `providers/oidc.rs`: `OidcProvider::new(cfg, http) -> OidcProvider` (discovery is lazy, on first use, cached 1 h).
   - `authorize_url`: `{authorization_endpoint}?response_type=code&client_id&scope=<scopes joined by space>&state&redirect_uri&code_challenge&code_challenge_method=S256&nonce`. Since discovery is lazy and `authorize_url` is sync, add `async fn prepare(&self) -> Result<(), ProviderError>` to the provider (a default no-op method on the `IdentityProvider` trait, added in `provider.rs`), which CT-24 awaits before `authorize_url`.
   - `exchange`: token POST (form, `client_secret_post`), read `id_token`; verify with `jsonwebtoken`: algorithms `RS256` and `ES256` only (refuse `none` and HS*), `iss` equals the issuer, `aud` contains `client_id`, `exp` in the future with 60 s leeway, `nonce` equals the one sent, `sub` 1-255 chars. Claims: `email` (lowercased), `email_verified` (bool or the strings `"true"`/`"false"`; anything else `false`), `name` or `preferred_username` for the display name. `provider_account_created_ms = None`.
3. `providers/mod.rs`: the `Oidc` arm builds an `OidcProvider`.
4. Cargo: `jsonwebtoken` (workspace 11). Dev: `rcgen` if the workspace has it (else `p256` with `pkcs8`, whichever `crates/auth` tests use; say which in the report), `wiremock`.

Tests (an ES256 key generated in the test, a `wiremock` issuer serving discovery, JWKS and token):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `oidc_exchange_verifies_and_maps_claims` | valid token gives subject, email, verified, display | new |
| `oidc_refuses_bad_tokens` | wrong `aud`, wrong `iss`, expired, wrong `nonce`, `alg: none`, HS256 signed with the client secret: each `Err(Token(..))` | each check is the guard against a forged identity |
| `oidc_string_email_verified` | `"true"` gives true, `"yes"` gives false | provider quirk |
| `oidc_refetches_jwks_once_for_a_new_kid` | rotated key: one refetch, then success; unknown kid twice within a minute: one fetch | new |
| `oidc_discovery_issuer_must_match` | discovery with another `issuer`: `BadResponse("issuer_mismatch")` | mix-up defence |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`, S4.

## CT-15 Identity, account and event repositories (live-DB)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-15-repos` · **Worktree:** `cto-15` (wave head after CT-10, CT-11 and CT-14)
- **Commit:** `feat(identity): identity, account and event repositories (CT-15)`
- **Rules:** R1-R9. Contract: C5, C6 (`last_method`), C7, C12, C15 (a, e).

Files (`sqlx` from the workspace with the features `crates/auth` uses; runtime-checked `sqlx::query`, not the `query!` macros, unless `crates/auth` uses the macros with offline data):

1. `crates/identity/src/repo/mod.rs`: `pub mod identity; pub mod account; pub mod event;` and `pub enum RepoError { Db, Conflict }` (no `sqlx::Error` text leaves the module).
2. `repo/identity.rs`: `find_bound(pool, provider, issuer, subject) -> Option<(i64 identity_id, AccountFacts)>`; `email_candidates(pool, email_lower) -> Vec<AccountFacts>` (distinct accounts whose identity has that `lower(email)` and `email_verified_at` not null); `insert(pool, account_id, &VerifiedIdentity, store_email: bool) -> Result<i64, RepoError>` (`Conflict` on the unique key; email stored only when `store_email`); `touch(pool, identity_id, now)`; `list_for_account(pool, account_id) -> Vec<IdentityRow>` (`identity_id, provider, display_name, created_at, last_used_at`; never subject or email); `delete(pool, account_id, identity_id) -> Result<bool, RepoError>`; `count_methods(pool, account_id) -> i64` (identities only; C15). `insert` is one transaction: `SELECT ... FOR UPDATE` the account row; when `password` or `password_hash_v2` is set, NULL both, set `password_algo = 1` and record `password_cleared` (C15 a); insert the identity; it returns `Linked { identity_id: i64, password_cleared: bool }` in place of the bare `i64`. Map SQLSTATE `23514` from the C5 guards to `RepoError::Conflict`.
3. `repo/account.rs`: `facts(pool, account_id) -> Option<AccountFacts>`; `facts_by_name(pool, name) -> Option<AccountFacts>` (`lower(account_name)`); `create(pool, candidates: &[String]) -> Result<(u32, String), RepoError>`: tries each candidate with `INSERT ... ON CONFLICT DO NOTHING` style uniqueness on `lower(account_name)` (check how `account` enforces name uniqueness first; if it has no unique index on `lower(account_name)`, check `NOT EXISTS` in the same statement), with `password` and `password_hash_v2` NULL, `accesslevel 0`, `enabled true`, `test_account false`; `has_password(pool, account_id) -> bool` (`password` or `password_hash_v2` not NULL; read by CT-45 only).
4. `repo/event.rs`: `record(pool, account_id, identity_id: Option<i64>, kind: EventKind, provider: Option<&str>)` (plus a `record_tx` taking a transaction, used by `insert`); `EventKind` with the eight C5 kinds and `as_str()`.
5. `lib.rs`: `pub mod repo;`. Cargo: `sqlx`; dev: `cimmeria-common` test support as `crates/auth` uses it.

Tests (`repo/tests_live_db.rs` via `#[cfg(test)] #[path]`; `live_db` in every name; sentinel account ids `2_000_201..=2_000_220`; names `ct15_<n>`; cleanup by exact id):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_find_bound_and_insert_conflict` | insert then `find_bound`; second insert of the same subject is `Conflict` | new |
| `live_db_email_candidates_only_verified` | an identity with email but no `email_verified_at` is not a candidate | auto-link safety |
| `live_db_create_account_skips_taken_names` | first candidate taken (any case): the second is used | name uniqueness |
| `live_db_count_methods_counts_identities` | 0 identities gives 0 even with a password; 2 identities gives 2 | the last-method rule (C15 e) |
| `live_db_insert_clears_the_password` | an account with an algo-1 `password`: `insert` returns `password_cleared: true`; both columns NULL, `password_algo = 1`; one `password_cleared` event; a second identity insert returns `password_cleared: false` | D-CT11 (a) |
| `live_db_list_never_returns_subject_or_email` | `IdentityRow` has no such fields (compile-time) and the query text lacks them (`assert!(!SQL.contains("subject"))`) | R7 |
| `live_db_events_record_every_kind` | each `EventKind` inserts | the CHECK list matches the enum |

Checks: S1 `cimmeria-identity`, S2 `cimmeria-identity`, S3 `cimmeria_identity::repo`.

## CT-16 Refresh-token repository (live-DB)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-16-refresh` · **Worktree:** `cto-16` (wave head after CT-15)
- **Commit:** `feat(identity): rotating refresh tokens with reuse detection (CT-16)`
- **Rules:** R1-R9. Contract: C5, C6 (Tokens), D-CT14.

Files:

1. `crates/identity/src/repo/refresh.rs`:
   - `pub const REFRESH_IDLE: chrono::Duration` 30 days; `REFRESH_FAMILY: chrono::Duration` 180 days (use the time type `crates/auth` uses with sqlx; `chrono` or `time`).
   - `pub fn new_token() -> (String token, [u8; 32] hash)`: 32 random bytes base64url (no padding); hash = SHA-256 of the token string.
   - `issue(pool, account_id, family: Option<&str>, now) -> Result<(String token, String family_id), RepoError>`: a new family id is 16 random bytes hex (32 chars).
   - `rotate(pool, presented: &str, now) -> Result<Rotation, RepoError>`, one transaction with `SELECT ... FOR UPDATE`: `Rotation::Rotated { account_id, family_id, token }` when the row is live (not revoked, not replaced, `last_used_at + IDLE > now`, `family_expires_at > now`); `Rotation::Reused { account_id, family_id }` when the row exists but is revoked or replaced (then revoke every row of the family); `Rotation::Expired` when idle or family expired (revoke the family); `Rotation::Unknown` when no row.
   - `revoke_family(pool, family_id, now)`, `revoke_account(pool, account_id, now)`, `revoke_token(pool, presented, now)` (logout: revokes its family).
2. `repo/mod.rs`: `pub mod refresh;`.

Tests (`live_db`, sentinel account `2_000_301`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_rotation_replaces_the_token` | rotate gives a new token; the old row has `replaced_by` and `revoked_at` | new |
| `live_db_reuse_revokes_the_family` | rotate A to B, present A again: `Reused`; B now fails with `Reused` too | the theft defence in C6 |
| `live_db_idle_and_family_expiry` | `now` 31 days after `last_used_at`: `Expired`; family limit likewise | D-CT14 |
| `live_db_tokens_are_stored_hashed` | no column holds the token text (`SELECT` every bytea and text column, compare) | R7 |
| `live_db_revoke_account_ends_every_family` | two families, both revoked | disabled accounts (C6) |

Checks: S1 `cimmeria-identity`, S3 `cimmeria_identity::repo::refresh`.
