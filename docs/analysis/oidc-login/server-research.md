# OIDC / external-identity login: server and DB research

Date: 2026-10-19. Scope: read-only research, no code changed. Companion to any client/launcher research under this directory.

Legend: **[V]** verified in this repo or a fetched doc this session. **[I]** inferred or from background knowledge, not re-verified. Anything marked [I] needs a spike before a packet depends on it.

## 0. Bottom line

- The cheapest viable design needs **no client patch and no registration site**. The launcher runs a loopback OAuth flow in the system browser, the server does the code exchange (it holds the provider client secret) and returns a **one-time login code**. The player types or pastes the code into the stock client's password field. The stock client sends `SHA1(code)` as the "password"; the server already accepts a 40-hex credential on plain HTTP, so it matches that hash against its in-memory pending-code table. [V for the server half, I for client field limits, see §2.6]
- Recommended first provider: **Discord** (the player base lives there, the server already ships a Discord crate for webhooks). Then **generic OIDC** (covers Google and anything with a discovery document), then **Steam OpenID 2.0** (no secret needed).
- DB change is **additive only**: one new `account_identity` table plus one new `password_algo` value. The persistence design already supports additive changes with no migration script. [V]
- Existing password accounts keep working unchanged. Linking a Discord identity to an existing account is done by a one-time link code issued from a logged-in session (or by GM), never by password re-entry.

## 1. Current login flow and account schema (all [V] unless noted)

**Protocol.** The stock client (`SGW.exe`) has a native `LoginWin` CEGUI window with account and password edit boxes. It POSTs SOAP over libcurl 7.17.0 (statically linked, OpenSSL 0.9.8g, cannot do modern TLS) to `/SGWLogin/UserAuth`, then `/SGWLogin/ServerSelection`, then Mercury UDP `baseAppLogin`. Sources: `docs/reverse-engineering/findings/auth-and-crypto-modernization-targets.md`, `docs/technical/login-auth-flow.md`, `docs/protocol/login-handshake.md`.

**Server code.** Crate `crates/auth/src/auth/`:
- `handlers.rs::handle_user_auth` (Phase 1). Checks `sku == "SGW_BETA"`, account name format (3..20 chars of `[A-Za-z0-9_-]`), classifies the credential, checks the protocol digest (skipped in developer mode), calls `validate_credentials`, then mints a 40-char alphanumeric `SID` cookie (`random_alphanumeric`, `rand::rng()` ThreadRng, a CSPRNG) held in an in-memory map with `SESSION_TTL` 300 s.
- `handle_server_selection` (Phase 2). Consumes the SID (single use), checks `shard.protected && access_level < 2`, mints a 32-byte AES session key and a 10-byte ticket (both `random_hex`) into `pending_logins`. `TICKET_TTL` is 30 s and the BaseApp re-checks age at Phase 3. Cross-IP use of the SID and ticket is only **warn-logged** (issue #442), not enforced.
- `credentials.rs::classify_credential`: a 40-char hex string is `LegacySha1Hex` (accepted on HTTP and TLS). Anything else is `Plaintext`, accepted only on the TLS listener (`TlsConn` marker layered onto the TLS router clone only). `validate_credentials` branches on `(password_algo, credential)`; legacy+plaintext opportunistically migrates to argon2id.
- Listeners (`service.rs`): HTTP on `logon_port` 8081, optional TLS on `auth_tls_port` 13443 (rustls, hot cert reload). `AuthService::set_public_routes` already lets the composition root mount extra routes on both listeners (used today for launcher telemetry, `/api/telemetry/*`). The colo compose publishes **only 8081 (plain HTTP)**; 13443 is not published, so the plaintext-over-TLS path is not reachable on the colo today. `docker/compose.yml` lines 45-57.
- Audit: `login_audit` table (`db/sgw/Audit/Tables/login_audit.sql`) with CHECK-constrained `phase` and `outcome` lists; any new outcome string must be added to the CHECK and to `LoginOutcome::as_str()` (a live-DB test enforces parity).
- Rate limiting / lockout: grep for `governor|rate_limit|lockout|throttle|failed_attempts` in `crates/auth` returns **zero hits**. There is no login throttling today. (Pre-existing gap, relevant because a new public endpoint inherits it.)

**Account row** (`db/sgw/Accounts/Tables/account.sql`):

| column | meaning |
|---|---|
| `account_id` int, sequence default | stable key; FK target of `sgw_player.account_id`, org tables, etc. |
| `account_name` varchar(32) | login name; handler enforces 3..20 chars `[A-Za-z0-9_-]` |
| `password` varchar(64) NULL | legacy unsalted upper-hex SHA-1, NULLed after migration |
| `password_hash_v2` text NULL | argon2id PHC string |
| `password_algo` smallint default 1 | 1 = sha1_legacy, 2 = argon2id |
| `accesslevel` int default 0 | 0 Player, 1 Moderator, 2 GameMaster, 3 Admin, 4 Developer (comment in `db/sgw/Accounts/Seed/account.sql`). Shard `protected` needs >= 2 |
| `enabled` bool default true | the ban switch; `false` returns login error 5 "suspended" |
| `test_account` bool default false | lab automation marker, copied into `session_kind` |

**Account creation today.** There is **no in-server registration path**. Accounts come from the seed (`db/sgw/Accounts/Seed/account.sql`, 8 dev accounts at accesslevel 2 with the SHA-1 of a shared test password) or from the admin panel docs (`docs/tools/admin-panel.md` "Create account" does a raw INSERT; the admin-api crate has no accounts route in `crates/admin-api/src`). Players therefore have no self-service path at all; this feature would be the first one.

**Dev-mode shortcut.** `developer_mode && !db_configured` accepts any well-formed credential as account 1, access level 99. A configured DB that fails to connect is fatal at startup. Container image default is `DEVELOPER_MODE=false`. (`db-fail-closed-startup` memory, `handlers.rs` lines 168-226.) The OIDC path must never be reachable through that shortcut and needs its own dev story (§4.4).

**Persistence rules that bound the schema change.** `docs/architecture/persistent-player-data.md`: `public` tables persist across deploys; accounts are carried over; **additive changes only** (new table, nullable or defaulted column); every new `public` table must be classified in `db/persistent/persistent-public-tables.txt` or `reseed-public-tables.txt` (a lint fails otherwise); "no migration framework". The `db/scripts/*.sql` migration prohibition in CLAUDE.md therefore does not block this feature: the schema lives in `db/sgw/Accounts/Tables/` and the colo/local refresh carries the data.

## 2. Recommended architecture

### 2.1 Why the stock client's constraints shape everything

- The client owns the login UI and only knows "account name" + "password". It will not do a browser flow itself.
- Its TLS is dead (OpenSSL 0.9.8g), and the colo serves login over plain HTTP. So whatever the client sends must be safe to be sniffed once: **single use, short TTL, high entropy, bound to an IP**. [V constraints]
- The password it sends is `UPPER(HEX(SHA1(typed_text)))`, a 40-char hex string. [V: `auth-and-crypto-modernization-targets.md` §3, and server comment in `credentials.rs`]

### 2.2 The flow ("loopback + server-side exchange + login code")

```
Launcher                    System browser          Provider (Discord)      Cimmeria server (:8081)
   | POST /api/oidc/start {provider, redirect_port, poll_secret_hash}  --------------> |
   | <------- {state, authorize_url}   (server builds URL, holds PKCE verifier by state)
   | open authorize_url ---> |  -- user logs in/consents -->  |
   |                         | <-- 302 http://127.0.0.1:PORT/cb?code&state            |
   |<== launcher's loopback listener receives code+state ==========================    |
   | POST /api/oidc/complete {state, code, poll_secret} ----------------------------> |
   |                         server: token exchange (client_secret) -> GET /users/@me
   |                         server: upsert account_identity -> account; mint login code
   | <------- {account_name, login_code, expires_in=60}                                |
   | launch SGW.exe; user (or autofill) enters account_name + login_code               |
   | client -> SOAP UserAuth(account_name, SHA1(login_code)) -> normal Phase 1/2/3     |
```

Key properties:
- **The provider client secret never leaves the server.** The launcher is a public client. (Required for Discord, see §2.3.)
- **No public web page is hosted.** The only new server surface is two JSON POST endpoints mounted via `set_public_routes`. The "callback page" is the launcher's own `127.0.0.1` listener.
- The server still uses PKCE (it generates `code_verifier` per `state`, keeps it in memory), which protects the code in transit through the browser/loopback hop; `poll_secret` (random, hash sent at start, preimage sent at complete) ties `complete` to the process that called `start`, so a code stolen from the loopback URL by another local process cannot be redeemed without it.
- State, PKCE verifier and pending login codes are **in-memory with TTLs** (`state` 10 min, login code 60 s), the same pattern and reaper as `sessions` and `pending_logins`. No table needed; a server restart merely invalidates in-flight attempts.

### 2.3 Provider fact check

Discord (fetched `docs.discord.com/developers/topics/oauth2` this session):
- [V] Grants listed: authorization code, implicit, client credentials, bot/webhook flows. **Device authorization grant is not mentioned.**
- [V] The page says the implicit grant should be replaced by "the authorization code grant with PKCE", but gives no PKCE parameters.
- [V] Token endpoint authentication is HTTP Basic or `client_id`+`client_secret` in the form body. Public-client (no secret) use is not documented. So plan on the secret living server-side. This is why the server does the exchange.
- [V] `identify` scope gives `/users/@me` (user id snowflake, username); `email` is a separate scope. An OIDC discovery URL (`/.well-known/openid-configuration`) is listed in the URL table, but the page documents no `openid` scope or `id_token`. Treat Discord as **OAuth2 + userinfo**, not strict OIDC.
- [I] `redirect_uri` must exactly match a registered URI; whether `http://127.0.0.1:PORT` is allowed and whether the port can vary is **not stated** on that page. Spike required. Fallback: register a fixed small set (for example 5 ports) and let the launcher pick the first free one.
- [I] The snowflake id encodes the account creation time; this gives a free minimum-account-age abuse check.

### 2.4 Provider comparison

| Provider | Protocol | Needs our secret | Loopback redirect | Device code | Stable subject | Registration / gating | Fit |
|---|---|---|---|---|---|---|---|
| **Discord** | OAuth2 + `/users/@me` (not strict OIDC) [V] | Yes, server-side [V docs] | Likely, unverified [I] | No [V not in docs] | snowflake `id` | Free app in dev portal; no review for `identify` [I] | **Best first**: audience is already on Discord |
| **Steam** | OpenID 2.0 (not OAuth) [I] | No (assertion verified via `check_authentication` POST to Steam) [I] | `return_to` can be loopback with matching `realm` [I] | No | SteamID64 from `claimed_id` | No app registration for identity; API key only to fetch persona name [I] | Good second; zero secrets. SGW was never on Steam, so fewer fans there |
| **Google** | Full OIDC, discovery, id_token [I] | Desktop client "secret" is non-confidential [I] | Yes (desktop loopback + PKCE) [I] | Yes (limited-input devices) [I] | `sub` | Consent screen needed; `openid email profile` are non-sensitive so no review [I] | Easy via generic OIDC; low demand |
| **Battle.net** | OAuth2 + OIDC (`openid` scope) [I] | Yes | Redirect rules unclear, likely needs registered URI [I] | No [I] | `sub` (account id) | Developer portal client; regional endpoints [I] | Possible via generic OIDC; low demand |
| **Epic** | OAuth2 via Epic Account Services [I] | Yes | Unclear | Not for third-party web apps [I] | Epic Account ID | Requires an Epic Developer Portal product and review [I] | Heavy; skip |
| **GOG** | No public third-party OAuth known [I] | n/a | n/a | n/a | n/a | n/a | **Not viable** |
| **Generic OIDC** | Discovery + code + PKCE + `id_token` (JWKS, nonce) [I] | Per provider | Per provider | Per provider | `iss`+`sub` | Operator config | The extensibility mechanism |

Recommendation: Discord first, then one generic-OIDC implementation (covers Google, Battle.net, Keycloak, etc.), then Steam OpenID as a separate small provider. Skip Epic and GOG.

### 2.5 Server-side extensibility

Define a trait in a new module `crates/auth/src/oidc/`:
`IdentityProvider { id() -> &'static str; authorize_url(state, challenge, redirect_uri) -> Url; exchange(code, verifier, redirect_uri) -> Result<VerifiedIdentity> }` where `VerifiedIdentity { provider, subject, display_name, account_created_at: Option<..>, email_verified: Option<bool> }`. Providers: `discord.rs` (token exchange + `/users/@me`, revoke the token after use), `oidc.rs` (discovery, JWKS signature + `iss`/`aud`/`exp`/`nonce` checks; use a maintained crate, for example `openidconnect`, instead of hand-rolling JWT validation; note repo memory says `jsonwebtoken` is an unused dep, do not assume it is a validated path), `steam.rs`. Provider list and enable flags come from config; client ids in config, secrets from env only.

### 2.6 Turning the identity into a login the stock client accepts

1. `complete` resolves `(provider, subject)` to `account_id` (create-on-first-login, §2.7), checks `enabled`, and mints `login_code`: 16 chars Crockford base32 (80 bits) from `OsRng`/`rand::rng()`. Store `sha1_upper_hex(login_code) -> { account_id, account_name, issued_ip, expires_at }` in a mutex-guarded map (same shape as `pending_logins`; reuse the reaper).
2. In `handle_user_auth`, **before** the password lookup: if the credential is `LegacySha1Hex(h)` and the map contains `h`, then `remove` it (single use, atomic under the lock), require `entry.account_name == req.account_name` (compare the derived name; reject on mismatch), re-check `enabled` against the DB (ban between issue and use), then continue into the normal SID mint with that account's `account_id/access_level/test_account`. Everything downstream (SID, ticket, session key, Mercury) is unchanged.
3. For OIDC-created accounts set `password = NULL`, `password_algo = 3` ("external"). `validate_credentials` must treat algo 3 as `InvalidCredentials` for every password credential, so a legacy-hash guess against such an account can never succeed. This is a required fail-secure arm (the existing `match` needs an explicit algo-3 arm; do not let it fall into the legacy branch).
4. Add `LoginOutcome` variants (for example `oidc_code_ok`, `oidc_code_invalid`) and extend the `login_audit.outcome` CHECK (live-DB parity test).

Why this works without a client patch [I]: the stock client hashes whatever is typed. The one thing to verify is the **client's password field length and character limits** and whether it trims/normalizes input. A memory note says the lab "keeps [the account] under the client's 8-character cap" [V it exists, I what it caps]. If the password field is also capped, shrink the code (an 8-char base32 code is 40 bits, acceptable only with the 60 s TTL, single use, IP binding and rate limiting below) or add the small client patch in §3.3. This is **open question Q1** and the top RE spike.

TLS note: if the later client patch ships (Phase 2b, `LoginReplyHandler` ctor `0x00DDED60`, plaintext over the rustls loopback proxy) the same code can be sent as `Plaintext` over TLS, with no server change to the code table; it simply gets classified `Plaintext` and looked up by `SHA1(plaintext)`.

### 2.7 Account linking and name derivation

- Primary key of identity: `(provider, subject)`. Never key on email or username (mutable, reusable). Discord `id`, Steam SteamID64, OIDC `iss`+`sub` (store `iss` in provider id for generic OIDC, for example `oidc:https://accounts.google.com`).
- **First login:** create an account: `accesslevel 0`, `enabled true`, `password NULL`, `password_algo 3`, `account_name` derived.
- **Name derivation:** do **not** use the Discord username. It is mutable, can collide, contains characters the handler rejects, and can impersonate staff. Use an opaque name from `account_id`, for example `d` + base36(account_id) zero-padded (short, inside 3..20, alphanumeric). Store the human display name in `account_identity.display_name` (informational, shown only in tooling). If the client caps names at 8, keep it <= 8 [I]. Because the typed name must match the derived one, the launcher auto-fills or displays it.
- **Existing account holders:** a logged-in player cannot link from inside the stock client (no UI). Provide two routes: (a) GM/admin links an identity to an existing account (`account_identity` INSERT via admin tooling or a console command; the owner currently does account edits through the admin panel or SQL); (b) launcher "link existing account" that asks for the old account name and password, calls `/api/oidc/link` with both plus the completed `state`, server verifies the legacy credential via `validate_credentials`, then inserts the identity row. Route (b) puts a password into the launcher once; acceptable only if sent over HTTPS (not available on the colo today) or the sha1 form. Recommend (a) for the initial release and (b) as a later packet gated on the TLS deployment. Owner decision Q4.
- **Unlink / rotate:** unlink only via GM. One identity per `(account_id, provider)`; multiple providers per account allowed (the `PRIMARY KEY (provider, subject)` plus `UNIQUE (account_id, provider)` constraint).

### 2.8 Ban and abuse handling

- Ban = `account.enabled = false` (already enforced at Phase 1 with error 5). Because the identity maps to exactly one account, re-registration via the same Discord id lands on the same disabled account. No new mechanism needed.
- Add `account_identity.blocked boolean` so a provider subject can be blocked **before** an account exists (pre-emptive ban list) and so a ban survives account deletion.
- New-account friction (all optional, configurable): minimum Discord account age from the snowflake timestamp [I]; require membership in a specific Discord guild (needs the `guilds.members.read` or `guilds` scope [I], increases consent surface); per-IP cap on new accounts per day.
- Rate limits are required on the new endpoints (§3.4) and the existing SOAP login has none (pre-existing gap).

## 3. Changes by area

### 3.1 DB (additive, edit `db/sgw/`, not `db/scripts/`)

New file `db/sgw/Accounts/Tables/account_identity.sql` (sketch):

```sql
CREATE TABLE account_identity (
    provider        varchar(64)  NOT NULL,   -- 'discord', 'steam', 'oidc:<issuer>'
    subject         varchar(128) NOT NULL,   -- provider's stable id
    account_id      integer      NOT NULL REFERENCES account(account_id),
    display_name    varchar(64),             -- informational, last seen
    blocked         boolean      NOT NULL DEFAULT false,
    linked_at       timestamptz  NOT NULL DEFAULT now(),
    last_login_at   timestamptz,
    PRIMARY KEY (provider, subject),
    UNIQUE (account_id, provider)
);
CREATE INDEX idx_account_identity_account ON account_identity (account_id);
```

- Classify `account_identity` in `db/persistent/persistent-public-tables.txt` (carried across rebuilds); check the table-order and FK handling in `db/persistent/reconcile.sql` and `seed-accounts-stage.sql` (the latter does `TRUNCATE account CASCADE`, which will also truncate `account_identity`; the rebuild must carry it the same way it carries `sgw_player`). **Verify before the packet**: this is the one place the persistence machinery can silently drop identities.
- `account.sql`: add a comment that `password_algo = 3` means external identity (no new column needed; there is no CHECK constraint today). No seed rows required. Optionally seed one dev identity row for tests.
- `login_audit.outcome` CHECK: add the new outcome strings.
- Owner question (flag, do not decide): the persistent-data design says "anything else than additive fails closed", and this change is additive, so I read it as no `db/scripts` migration being needed. Confirm Q5.

### 3.2 Server (crates)

- `crates/auth/src/oidc/` (new): `mod.rs` (state/pending-code maps, TTL reaper hook), `provider.rs` (trait), `discord.rs`, `oidc.rs`, `steam.rs`, `routes.rs` (`/api/oidc/start`, `/complete`, optional `/providers`).
- `crates/auth/src/auth/handlers.rs`: new pre-lookup branch for login codes (§2.6); `credentials.rs`: algo-3 arm that denies password credentials; `audit.rs` outcomes.
- `crates/auth/src/auth/mod.rs::HandlerState`: add `oidc: Option<Arc<OidcState>>`; share the pending-code map with the handler.
- Mounting: through `AuthService::set_public_routes` (already used for telemetry), so the endpoints appear on 8081 and, when configured, 13443. Composition root: `crates/services` orchestrator (where telemetry routes are passed today).
- `crates/common/src/config.rs`: `oidc_enabled`, provider list, public `client_id`s, redirect port set, TTLs. Secrets via env only.
- Do **not** store provider access/refresh tokens. Call `/users/@me`, then revoke or discard.
- Tracing: log provider, subject hash prefix, account_id, outcome; never log `code`, `state`, `poll_secret`, login codes or tokens. Reuse `credential_redaction::CredentialPrefix` style and add the new field names to `credential_log_guard.rs` (a test there guards credential leaks into logs).

### 3.3 Launcher and client

- Launcher (`crates/launcher/desktop`, `crates/launcher-core`): loopback listener on `127.0.0.1` (bind loopback only, random free port from the registered set, one request then close, validate `state`), open the system browser, call start/complete, then show account name + code with copy-to-clipboard and a countdown, and launch the client (`client-launch::launch_sgw*`; today it takes only the install dir, no credentials [V]).
- Optional QoL (small client-side helper, not a protocol patch): the injected telemetry DLL can fill `LoginWin` through the same CEGUI path `lab_login` uses (the lab already types into the account and password widgets [V: `crates/lab/src/supervisor/flows`]). That removes the paste step and the field-length risk for the account name. It does not change the wire protocol. Needs an owner call because the lab code is research tooling (Q3).
- No Mercury or SOAP change is required for the MVP.

### 3.4 Security considerations

- **Replay.** Login code: single use (`remove` under the lock), 60 s TTL, 80-bit random, IP-bound. Because the colo is plain HTTP, `SHA1(code)` is visible to a network observer, so IP binding on consume is what limits a race: enforce `client_ips_match(issue_ip, consume_ip)` for codes (unlike the warn-only #442 check). CGNAT/VPN risk: launcher and game run on one host, so the same public IP is expected; offer a config switch (Q6).
- **Fixation / CSRF.** Random `state` (128-bit), bound to the starting launcher by `poll_secret`; one use; 10 min TTL. Loopback listener rejects any request whose `state` does not match.
- **Open redirect / SSRF.** `redirect_uri` is built server-side from the allow-listed port set; the client never supplies a URL. Discovery URLs come from config only.
- **Secret handling.** Discord `client_secret` is injected from the Azure Key Vault (`cimmeria-kv`, see the key vault memory reference) into an env var at deploy, like `CIMMERIA_TELEMETRY_HMAC_SECRET` (compose line 81 pattern), never in repo, seed, image or logs; refuse to enable a provider whose secret is missing (fail closed, log `reason=oidc_secret_missing`).
- **Rate limits.** None exist today [V]. Add per-IP limits (for example `tower_governor`) on `/api/oidc/*` (start 10/min, complete 10/min) and a global cap on the pending maps (bounded memory, evict oldest). Add the same to SOAP `UserAuth` as a separate hardening item, since a code-guess attack goes through that path: 80 bits makes guessing infeasible, a small code (<= 40 bits) does not without throttling.
- **Account enumeration.** Return the existing generic "name or password incorrect" (code 4) for any code failure.
- **Takeover on provider side.** A hijacked Discord account is a hijacked game account; mitigate by a GM-visible `last_login_at`/IP history (already in `login_audit`) and the `blocked` flag. Do not auto-link on matching email.
- **Impersonation by display name.** Never show provider display names as in-game names; character names are separate (`sgw_player`).
- **Developer mode.** The dev shortcut accepts any credential as account 1 access 99 when no DB is configured. The OIDC endpoints must return 404 in that mode, so the shortcut cannot be reached through the new surface.
- **Transport.** Public `/api/oidc/*` on plain HTTP exposes `code`/`state` between launcher and server in clear. Mitigations: the provider `code` is single use and PKCE-bound to a verifier the server holds; `poll_secret` binds the caller. Strongly prefer publishing the TLS listener or the existing Cloudflare Tunnel HTTPS route (`docs/operations/telemetry.md` step 2) for these two endpoints; the launcher talks real TLS (the launcher is modern Rust, unlike the client). This is Q7.

## 4. Compatibility, existing accounts, fallback

### 4.1 Existing password accounts
Unchanged. They keep logging in with the typed password (legacy SHA-1 or argon2id migration). No forced migration. Operators can choose to link them (§2.7). The login-code branch is checked only when the 40-hex value matches the code map, so there is no change for password users.

### 4.2 Eventual removal of passwords (optional later)
Set `password`/`password_hash_v2` NULL and `password_algo = 3` once every active account is linked. Not recommended before a link UX exists.

### 4.3 Seed/dev accounts
Leave `test`, `cady`, etc. untouched. Lab accounts stay password-based (`test_account = true`); the lab and wireclient test harnesses do not go through the browser flow.

### 4.4 Fallback and dev login
- **Fallback when no browser/loopback works** (locked-down machine, Wine, Steam Deck): launcher shows the authorize URL, user opens it anywhere; the redirect lands on a dead loopback; the user copies the final URL (or just the `code`) back into the launcher. No hosted page required. If the owner accepts a hosted page, a 20-line static "paste this code" callback on the server removes the loopback dependency, but then redirect_uri is a public https URL, which needs TLS (Q7).
- **Dev/test provider.** Add a `fake` provider compiled only with `developer_mode` (and refusing to register when `db_configured` and not dev) so tests and local runs exercise start/complete without Discord. Live-DB tests use the sentinel account-id window convention already used in `credentials.rs`.
- **Operator break-glass.** Keep one password admin account (seed) so a Discord outage does not lock out staff.

## 5. Packet slicing (sizes: S under a day, M one to two days, L several days)

| # | Packet | Size | Notes |
|---|---|---|---|
| P0 | RE/spike: client password/account field length+charset limits; Discord loopback `redirect_uri` and port rules; Discord token endpoint without secret | S | Decides code length and whether the DLL autofill is needed. Needs a live client (ask first, lab etiquette) and one throwaway Discord app |
| P1 | DB: `account_identity` table, `password_algo=3` doc, persistent-table classification, `login_audit` outcomes, live-DB tests incl. carry-across-rebuild | S | Check `reconcile.sql` / stage `TRUNCATE account CASCADE` |
| P2 | Server core: login-code map, handler branch, algo-3 deny arm, audit, tests (revert-proof) | M | No provider needed; testable with an injected identity |
| P3 | `IdentityProvider` trait, `/api/oidc/start|complete`, state/PKCE/poll_secret, rate limits, fake dev provider | M | Mount via `set_public_routes` |
| P4 | Discord provider (+ config, secret via env/Key Vault, deploy docs) | S | After P0 |
| P5 | Launcher: loopback listener, browser open, UI, code display/copy, manual-paste fallback | M | Includes UAT row for unified UAT guide |
| P6 | Generic OIDC provider (discovery, JWKS, nonce) | M | Use a maintained crate |
| P7 | Steam OpenID 2.0 provider | S/M | |
| P8 | Account linking: GM command/admin tooling, then launcher "link existing" | M | Second half waits on TLS (Q7) |
| P9 | Optional client helper: autofill of `LoginWin` via injected DLL | M | Needs P0 outcome and owner OK |
| P10 | Hardening: SOAP login rate limit/lockout, enforce IP binding option, docs (login-handshake, operations, security-audit), memory notes | M | Pre-existing gap worth doing regardless |

Dependencies: P1 -> P2 -> P3 -> P4 -> P5; P0 gates P4/P5/P9. Docs to walk per `doc-update-map.md`: `docs/protocol/login-handshake.md`, `docs/connection-flow.md`, `docs/operations/*` (new env vars, Key Vault), `docs/tools/admin-panel.md` (account creation text is now stale for OIDC accounts), the encryption ADR cross-reference.

## 6. Open owner decisions

- **Q1.** Is a typed one-time code acceptable UX for v1 (paste into the stock client), or is the DLL autofill (P9) a day-one requirement? Depends on P0 field limits.
- **Q2.** Provider order and scope: Discord only for v1, or Discord + generic OIDC? Skip Epic and GOG? (Recommended: Discord, then OIDC, then Steam.)
- **Q3.** May research tooling (injected telemetry DLL / lab typing path) ship in the player build to autofill the login window?
- **Q4.** Existing accounts: GM-only linking at first (recommended), or launcher "link with old password" (needs HTTPS)?
- **Q5.** Confirm that an additive `db/sgw/` table plus persistent-table classification satisfies the "no `db/scripts` migrations without asking" rule. I believe yes; flagged because the rule is the owner's.
- **Q6.** IP binding of login codes: enforce (recommended) with a config escape hatch, or warn-only like #442?
- **Q7.** Transport for `/api/oidc/*`: publish TLS (13443) or use the Cloudflare Tunnel HTTPS route? Plain HTTP on 8081 works but exposes the code/state hop. Also decides whether any hosted callback page is acceptable.
- **Q8.** New-account friction: minimum Discord account age, guild membership requirement, per-IP signup cap, and an invite/allowlist mode for closed betas.
- **Q9.** Who owns the Discord developer application, and which Key Vault secret name/rotation policy applies.
- **Q10.** Name policy: opaque `d<base36 id>` (recommended) versus a human-readable derived name (collision, impersonation, and 8-char-cap risks).

## 7. Verified vs inferred summary

Verified in repo: all of §1; the existing 40-hex-on-HTTP acceptance; `set_public_routes`; colo publishes only 8081; no rate limiting in `crates/auth`; account schema; persistent-table rules; launcher launches the client with no credentials; client has a native `LoginWin`; the lab can type into it.

Verified from Discord docs fetched today: grant types listed, device grant not mentioned, secret-based token endpoint auth, `identify` scope semantics, OIDC discovery URL listed without documented `openid` scope.

Inferred / not verified (spike before relying): client password field limits; Discord loopback redirect and port flexibility; Discord PKCE-without-secret; Steam, Google, Battle.net, Epic, GOG specifics in the §2.4 table; snowflake-based account age; `reconcile.sql` carrying `account_identity` across rebuild.
