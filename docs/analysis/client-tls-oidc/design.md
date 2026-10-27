# Client TLS and OIDC login: design note

> **Status:** research and design only. No code, no packets. Written 2026-10-24 by the architect.
> **Question:** how do we upgrade the 2009 client's OpenSSL so login uses real HTTPS, and how do we replace the
> account/password login with OIDC sign-in (Discord and other game providers)?
> **Short answer:** don't upgrade the client's OpenSSL. Do the OIDC sign-in in the launcher, have the server
> mint a per-launch game ticket, and send the client's existing SOAP login through a loopback TLS forwarder
> in the launcher. That needs **no binary patch, no new opcode and no wire-crypto change**. The only change on
> the client side is the content of `LoginInternal.lua`, a file the launcher already rewrites before every
> launch. Several points are owner decisions (see [§ 9](#9-owner-decisions-blockeddecision)).

Evidence labels used below:

- **[V]** verified in this pass or by a cited finding, with the evidence named.
- **[I]** inferred: reasoning from code, docs or general knowledge, not checked against the live client or the provider.
- **[U]** unknown: needs a spike or an answer.

## 1. Facts

### 1.1 The client's TLS stack

| # | Fact | Label and evidence |
|---|---|---|
| F1 | The login transport is **libcurl 7.17.0**, linked statically into `SGW.exe`. gSOAP only serializes the XML. | [V] Ghidra 2026-06-19 ([auth-and-crypto-modernization-targets.md § 1](../../reverse-engineering/findings/auth-and-crypto-modernization-targets.md)); string `libcurl 7.17.0` at file offset `0x016E97CF` (scan 2026-10-24). |
| F2 | The SSL library is **OpenSSL 0.9.8g (19 Oct 2007)**, also static. `Working\Binaries` ships no `libeay32.dll`, `ssleay32.dll` or `libcurl.dll`, so there is no DLL to swap. | [V] strings `OpenSSL 0.9.8g 19 Oct 2007`, `SSLv3 part of OpenSSL 0.9.8g`, `TLSv1 part of OpenSSL 0.9.8g` at file offsets `0x017278EC`..`0x0172C480`; directory listing 2026-10-24. |
| F3 | OpenSSL 0.9.8g tops out at **TLS 1.0**: TLS 1.1 and 1.2 arrived in OpenSSL 1.0.1 (2012). It has no AES-GCM, no ECDHE suites in a default build, and no TLS 1.3. | [I] upstream release history; matches F2 (only the `SSLv3`/`TLSv1` method strings are present). |
| F4 | curl 7.17.0 sends **no SNI**. curl added SNI in 7.18.1. | [I] upstream changelog. |
| F5 | A modern endpoint (rustls, Caddy or nginx with current defaults, any CDN) refuses TLS 1.0. rustls cannot speak TLS 1.0 at all. | [I] |
| F6 | Whether the client's curl build has SSL enabled, and which CA bundle it would verify against. | [U] Not needed by the recommended path. |
| F7 | `SGW.exe` has no anti-debug and no anti-tamper. Injection and inline, IAT or vtable hooks are all safe. | [V] finding § 4. |

### 1.2 Where the login call happens

| # | Fact | Label and evidence |
|---|---|---|
| F8 | The login button runs Lua: `LoginMod.onLoginClicked` reads `Login_AccountEdit` and `Login_PasswordEdit`, picks the server URL from `LoginMod.servers`, clears the password box and calls the native `accountLogin(server, username, password)`. | [V] `Working\SGWGame\Content\UI\Startup\Login\Login.lua` in the QA install (this copy carries Project Giza's website links, so it may differ from the original CME file). |
| F9 | `LoginMod.startup()` puts the saved account name in the box and **then** calls `LoginMod.loadServerSystems()`, which `LoginInternal.lua` defines. A launcher-written `loadServerSystems` therefore runs after the edit boxes exist and can fill them. | [V] reading `Login.lua`; [U] not exercised live. |
| F10 | The launcher rewrites `LoginInternal.lua` before every launch from its `login_servers` setting. The default URL is `http://play.cimmeria.app:8081`, and the client appends `/SGWLogin/UserAuth` to it. | [V] `crates/launcher-core/src/client_setup/login_servers.rs`; [docs/client/sgw-launcher.md § Login servers](../../client/sgw-launcher.md). |
| F11 | The password box has `MaskText` set and no `MaxTextLength`, so CEGUI's default limit applies. A 32-character ticket fits. | [V] `Login.layout`; [I] CEGUI default. |
| F12 | `accountLogin` leads to `logOnBegin @ 0x00DDF580`, which builds the SOAP POST through the `LoginReplyHandler` ctor `@ 0x00DDED60` (`curl_easy_setopt` thunk `@ 0x013A96E0`). The password is SHA-1 hashed as uppercase hex before it is sent. | [V] finding §§ 1 and 3. |
| F13 | The live capture of `ServerConnection+0x130` (`A94A8FE5CCB19BA61C4C0873D391E987982FBBD3`) is **SHA-1("test")**, the password hash. The draft `docs/drafts/spec/mercury-wire-format.md` (R16 and Figure 39) calls it an "internal dispatch-table hash". That label is wrong. | [V] computed SHA-1("test") 2026-10-24. Adjacent doc fix, not made here. |
| F14 | "Change Server" calls `relogin()`, which re-runs the SOAP login without the player re-typing anything, so the client re-sends its stored credential hash. | [V] that relogin re-runs SOAP ([known-issues.md KI](../../known-issues.md)); [I] that it re-sends the stored `+0x130` hash. **Design consequence:** a strictly one-time ticket would break Change Server. |
| F15 | A bare URL in `LoginMod.servers` (`http://host:port`) works. Whether a base URL with a path prefix works is unknown. | [V] production use; [U] path prefix. |

### 1.3 What the server login endpoint does today

| # | Fact | Label and evidence |
|---|---|---|
| F16 | Phase 1 `POST /SGWLogin/UserAuth` checks the SKU, the account-name format (3 to 20 characters of `[A-Za-z0-9_-]`) and the protocol digest, then validates the credential. A 40-character hex value is a legacy SHA-1 hash and is accepted over HTTP or TLS. Anything else is treated as a plaintext password, accepted **only over TLS** and checked with argon2id, with on-login migration. Success issues a 40-character `SID` cookie (`SESSION_TTL`). | [V] `crates/auth/src/auth/handlers.rs`, `credentials.rs`. |
| F17 | Phase 2 `POST /SGWLogin/ServerSelection` consumes the SID once and returns the **BaseApp address, the 32-byte Mercury session key and a 10-byte ticket** in the XML. The ticket goes into `pending_logins` with `TICKET_TTL` 30 s. A cross-IP SID use only logs a WARN. | [V] `handlers.rs`; TTLs from the network-security-auth audit memory. |
| F18 | The server has a **TLS listener** (tokio-rustls, arc-swap hot reload; port `13443`), but it is **off by default** and the colo does not publish it. Players reach login on **plain HTTP `8081/tcp`** only. | [V] `crates/common/src/config.rs`; [colo-deploy.md § Ports](../../operations/colo-deploy.md). |
| F19 | So today a passive sniffer sees the SHA-1 hash (which is the credential itself: replaying it logs in) **and the Mercury session key**, which decrypts the whole game session. | [V] F16 and F17 together; audit 2026-05-31 findings #5 and #8. |
| F20 | No OIDC or OAuth code exists in the repo. The only "auth token" is the HMAC telemetry token from `/api/auth/dev-session`, which needs no credential. `jsonwebtoken` is unused. | [V] grep 2026-10-24; network-security-auth memory. |
| F21 | Accounts, characters and player tables persist on the colo's named volume. Persistent-table schema changes must be **additive** (D-PP7), and new tables go in `db/persistent/persistent-public-tables.txt`. | [V] [persistent-player-data.md](../../operations/persistent-player-data.md). |
| F22 | How real players get accounts today (self-service, the admin panel or by hand) is not documented in `docs/`. | [U] |

### 1.4 The launcher

| # | Fact | Label and evidence |
|---|---|---|
| F23 | The desktop launcher (Tauri 2) already has `reqwest` with `rustls-tls`, tokio, and `tauri-plugin-opener` for opening the system browser. | [V] `crates/launcher/desktop/{engine,shell}/Cargo.toml`. |
| F24 | The game can outlive the launcher window. The i686 `cimmeria-launch-worker.exe` "stays alive until the game exits". Closing the launcher while the game runs ends the launcher's telemetry session. | [V] `crates/launcher/desktop/docs/launch.md`. |
| F25 | The launcher injects `cimmeria-client-patches.dll` on every launch unless the player opts out. Its hook primitives already exist (`install_inline_hook`, `replace_iat_slot`, `swap_vtable_slot`). | [V] `crates/client-patches/README.md`, encryption ADR Phase 0. |

## 2. Why "upgrade the client's OpenSSL" is the wrong goal

Because of F1 and F2 there is no OpenSSL DLL to replace. Upgrading in place would mean detouring every
statically linked OpenSSL entry point that curl 7.17.0 calls (`SSLv23_client_method`, `SSL_CTX_new`,
`SSL_CTX_set_cipher_list`, `SSL_CTX_load_verify_locations`, `SSL_new`, `SSL_set_fd`, `SSL_connect`,
`SSL_read`, `SSL_write`, `SSL_pending`, `SSL_get_error`, `SSL_shutdown`, the `X509_*` verify path and more),
each found without symbols, and mapping 0.9.8 struct semantics onto a modern library. curl 7.17.0 would
still send no SNI (F4) and would still carry its own 2007 CVEs. All that work buys the same result the
loopback forwarder gives without touching the binary.

The real goals are different: **(1)** the reusable credential and the Mercury session key must never cross the
internet in clear (F19), and **(2)** the player signs in with an external identity. Neither goal needs the
client to speak TLS.

## 3. Options compared

| | Option | Client change | Effort | Main risk | Verdict |
|---|---|---|---|---|---|
| a | Replace or inject a modern OpenSSL/libcurl | Binary: dozens of detours on unnamed static functions | Very high | Fragile ABI shimming; no SNI from curl 7.17; per-build fingerprints | **Reject** |
| b | Injected DLL hooks curl (`curl_easy_setopt` URL rewrite, as encryption ADR Phase 1 plans) and runs a rustls proxy in-process, or replaces the transport with WinHTTP or Schannel | Binary patch through `client-patches` | Medium to high | tokio and rustls inside the 32-bit game process; login breaks when the player turns off "Load client patches"; AV heuristics; Schannel under Wine is GnuTLS | **Fallback only.** The curl hook is not needed, because the URL is already data (F10). |
| c | Launcher loopback forwarder: `LoginInternal.lua` points at `http://127.0.0.1:<port>`; the launcher forwards `/SGWLogin/*` over TLS 1.2+ to the server | **None** (Lua data the launcher already writes) | Low | The forwarder must be running when the client logs in or relogs (F24); local-process abuse (see § 6.3) | **Adopt** for transport |
| d | OIDC in the launcher (system browser, Authorization Code + PKCE, Discord and other providers); the server mints a game ticket; the client presents it through the existing login | `LoginInternal.lua` fills the account and ticket boxes (Lua UI behaviour; a maintainer decision) | Medium (mostly server and launcher) | Ticket handling (F14); account-linking UX | **Adopt** for identity |

Ways to hand the ticket to the client in (d):

| | Handoff | Notes | Verdict |
|---|---|---|---|
| d1 | The launcher writes the account name and ticket into `LoginInternal.lua`; `loadServerSystems()` fills the two boxes (F9). The player presses Login, or the Lua auto-submits. | No binary patch. The ticket sits on disk briefly (see § 6.3). | **Recommend** |
| d2 | `client-patches.dll` pushes a `CimmeriaAuth` table into the UI Lua, as it does with `CimmeriaBMNative` | Keeps the ticket off disk, but login then depends on DLL injection and its opt-out | Alternative if d1's disk exposure is refused |
| d3 | The launcher shows a one-time code for the player to paste as the password | No UI change, but poor UX | Fallback for users without the launcher |
| d4 | The forwarder injects the credential into each `UserAuth` request | Turns the forwarder into a confused deputy: any local process could log in as the player | **Reject** |

**Classification under the project rules.** (c) and (d1) are launcher plus server work. They add no opcode, no
wire-crypto change and no binary patch. The rule "client UI needs a maintainer decision" does apply to d1,
because it changes what the login screen does (pre-fills, and maybe auto-submits). (b) is a client patch. (a)
is a client patch of a much larger kind.

## 4. Recommended architecture

```text
 Player        Launcher (native)                     Cimmeria auth (HTTPS)                 IdP (Discord, ...)
   |  Sign in     |                                          |                                    |
   |------------->| PKCE v1, state1; loopback :p/cb          |                                    |
   |              |-- browser: /auth/v1/authorize?provider=discord&cc=S256(v1)&redirect=127.0.0.1:p --->|
   |              |                                          | state2, nonce, PKCE v2 (server's)  |
   |              |                                          |-- 302 to IdP authorize ------------>|
   |   (consent in the browser)                              |<-- /auth/v1/callback?code&state2 ---|
   |              |                                          | exchange code (secret + v2), check  |
   |              |                                          | id_token or userinfo; resolve link  |
   |              |<-- 302 127.0.0.1:p/cb?code=H&state1 -----| H: one-time, 60 s, bound to cc+uri |
   |              |-- POST /auth/v1/token (H, v1) ---------->| verify S256(v1)==cc                 |
   |              |<-- access (15 min) + refresh (rotating) -|                                    |
   |  Play        |                                          |                                    |
   |------------->|-- POST /auth/v1/launch-ticket ---------->| ticket T, scoped to account+launch  |
   |              | write LoginInternal.lua:                 |                                    |
   |              |   server = http://127.0.0.1:f            |                                    |
   |              |   fill account + T                       |                                    |
   |              | start forwarder on 127.0.0.1:f           |                                    |
 SGW.exe          |                                          |                                    |
   |-- accountLogin -> curl POST http://127.0.0.1:f/SGWLogin/UserAuth (password = SHA1HEX(T))       |
   |              |== TLS 1.2+/1.3 ==> /SGWLogin/UserAuth -->| T matches -> SID (unchanged flow)   |
   |-- ServerSelection via the forwarder ===================>| session key + ticket, now over TLS  |
   |-- Mercury UDP to the BaseApp, unchanged ----------------------------------------------------->|
```

1. **The server is a small authorization server for the launcher** and a confidential OAuth/OIDC client to
   each upstream provider. Provider secrets never reach the launcher.
2. **The launcher is a public native client** (RFC 8252): system browser, loopback redirect on `127.0.0.1`
   with an ephemeral port, PKCE S256 required, and a `state` check. Our server is the authorization server,
   so it can accept any loopback port. That sidesteps providers that match redirect URIs exactly, which
   Discord is believed to do [I].
3. **The game ticket** is a per-launch bearer secret the launcher fetches on Play. The client sends it as the
   password, and the client's own SHA-1 step turns it into `SHA1HEX(T)`. The server matches that value
   against the live tickets for the account in constant time, then runs the existing Phase 1 and 2 code
   unchanged.
4. **The forwarder** carries both SOAP phases over TLS. It adds no authority: it only relays bytes to the
   one configured server. This closes F19 for launcher users: the Mercury session key travels inside TLS.
5. **Password login stays** for players without the launcher, the lab, `wireclient` and self-hosters. It
   also benefits from the forwarder when the launcher is used.

What this makes unnecessary for launcher users: encryption ADR Phase 1 client side (the curl hook and in-DLL
rustls) and Phase 2 client side (the plaintext-password swap). Mercury v2 (Phase 3) is not affected and still
needs its client patch.

## 5. Server design

### 5.1 Provider abstraction

Put this in a new crate (working name `cimmeria-identity`) so it stays pure and testable against a mock
identity provider. Mount its routes on the auth router. Do not put them on admin-api `8443`, which has no
authentication (#439).

```rust
pub struct ExternalIdentity {
    pub provider: ProviderId,          // config key, e.g. "discord"
    pub issuer: String,                // OIDC iss, or a fixed per-provider string for OAuth2-only providers
    pub subject: String,               // stable provider user id; THE link key
    pub display_name: Option<String>,
    pub email: Option<String>,         // never used for linking
    pub email_verified: bool,
}

#[async_trait]
pub trait IdentityProvider: Send + Sync {
    fn id(&self) -> &ProviderId;
    fn authorize_url(&self, state: &str, nonce: &str, pkce_challenge: &str, redirect_uri: &Url) -> Url;
    async fn complete(&self, code: &str, pkce_verifier: &str, nonce: &str, redirect_uri: &Url)
        -> Result<ExternalIdentity, IdpError>;
}
```

There are three provider kinds, each built from `config/auth-providers.toml`. Commit an `.example` file only.
Secrets come from environment variables such as `CIMMERIA_IDP_<ID>_CLIENT_SECRET`, which the colo fills from
Azure Key Vault `cimmeria-kv`.

| Kind | How the identity is proven | Example providers [I, verify at implementation time] |
|---|---|---|
| `oidc` | Discovery document and JWKS. Validate the `id_token`: `iss`, `aud`, `exp`/`iat` with skew, `nonce`, an allowlist of algorithms (RS256 and ES256, never `none` or HS*), `azp` when several audiences are present. | Google, Microsoft (consumer accounts), Twitch, Battle.net, Epic Account Services, any self-hosted IdP |
| `oauth2_userinfo` | Code exchange, then a userinfo GET with the access token. The subject comes from a configured field. | **Discord** (`identify` scope, `GET /users/@me`, field `id`), GitHub (`id`) |
| `steam_openid2` (later) | OpenID 2.0 `check_authentication`; the SteamID64 comes from `claimed_id`. Not OIDC. | Steam |

Rules for every kind: an exact `redirect_uri`, a single-use `state` with a 10-minute TTL that binds the
pending record, PKCE to the provider wherever it supports it, the `nonce` for OIDC, TLS-verified HTTPS for
every back-channel call, and bounded response sizes and timeouts. Never accept an identity from the front
channel (the implicit flow).

### 5.2 Endpoints (HTTPS only)

| Endpoint | Purpose |
|---|---|
| `GET /auth/v1/providers` | Lists the enabled providers (id, display name, icon key). The launcher builds its buttons from this. |
| `GET /auth/v1/authorize?provider&redirect_uri&state&code_challenge&code_challenge_method=S256` | Checks that `redirect_uri` is `http://127.0.0.1:<port>/<path>` (or `[::1]`), stores the pending record, and redirects to the provider. |
| `GET /auth/v1/callback/{provider}` | Validates `state`, runs `complete()` and resolves the link (§ 5.4). Mints a one-time code `H` (60 s, bound to the launcher's `code_challenge` and `redirect_uri`) and redirects to the loopback address. When the identity is not linked, `H` carries a "needs link" marker instead of an account. |
| `POST /auth/v1/token` | `grant_type=authorization_code` (H + verifier) or `refresh_token`. Returns an opaque access token (15 min) and a rotating refresh token. |
| `POST /auth/v1/link/existing` | Needs a "needs link" grant **plus** the existing account's name and password. The launcher sends `SHA1HEX(password)` so the existing verifier works unchanged. |
| `POST /auth/v1/link/new` | Needs a "needs link" grant plus a requested account name. Creates an account with no password. Gated by D-CT5. |
| `GET/DELETE /auth/v1/identities` | Lists or unlinks identities. Unlinking requires at least one remaining login method and a token issued within the last 10 minutes. |
| `POST /auth/v1/launch-ticket` | Takes an access token and returns `{account_name, ticket, expires_at}`. Revokes the account's previous live ticket. |
| `POST /auth/v1/launch-ticket/end` | The launcher reports that the game exited; the ticket is revoked. |
| `POST /auth/v1/logout` | Revokes the refresh-token family. |

Changes to `UserAuth` (`handlers.rs`): before the password check, look up a live launch ticket for the account
and compare `SHA-256(submitted)` with the stored `SHA-256(SHA1HEX(T))` in constant time. Accept ticket logins
**only over TLS** (the `TlsConn` marker) unless D-CT7 says otherwise. Audit with a new
`LoginOutcome::LaunchTicket`. Everything after the SID is unchanged.

### 5.3 Data model (additive, `db/sgw/Accounts/Tables/`, listed as persistent)

- `account_identity (identity_id, account_id FK, provider, issuer, subject, display_name, created_at, last_used_at, UNIQUE(provider, issuer, subject))`.
- `auth_refresh_token (token_hash PK, family_id, account_id, issued_at, expires_at, revoked_at, replaced_by)`. Store only SHA-256 hashes. A rotated token presented again revokes its whole family.
- `account` gains no columns. A launcher-created account has `password` and `password_hash_v2` both NULL. `validate_credentials` must treat that as "no password login", not as a failure path that can be bypassed. Add a test for exactly that.
- Pending authorize records, one-time codes `H`, access tokens and launch tickets live **in memory** with TTL reapers, as `pending_logins` does. A server restart already ends every game session, and the launcher simply asks again.

### 5.4 Account linking rules

- The link key is `(provider, issuer, subject)`. **Never link by email.** Discord and GitHub emails are mutable,
  and auto-linking on email is a known takeover path.
- An unlinked identity gets two choices in the launcher: **link an existing Cimmeria account** (name and
  password once, verified server-side, rate limited) or **create a new account** (D-CT5).
- One identity maps to at most one account. One account can have several identities.
- The colo's lab and GM accounts (`test_account`, access level ≥ 2) should not be linkable from the public
  flow without a GM step. [I] This needs a policy decision; it is listed under D-CT6.
- GM unlink belongs to the GM console, not admin-api, until #439 is fixed.

### 5.5 Launch ticket

- 160 bits of CSPRNG output, rendered as 32 characters of `[A-Za-z0-9]`, so it survives the Lua box and the
  plaintext-length gate unchanged.
- Scope: one account, one launch id, one server.
- Life: until the launcher reports game exit, or 12 hours absolute, or 30 minutes idle after its last use,
  whichever comes first. **Several uses within that life are allowed** (F14: Change Server re-sends it).
  Only one ticket is live per account.
- Client IP: record it, and WARN on mismatch like the SID check does. Do not reject yet: the forwarder may
  reach the server over IPv6 while the game's UDP uses IPv4.
- Never logged. Use `CredentialPrefix`, as the SID and Mercury ticket already do.

### 5.6 Network-security-auth implications

| Audit item (2026-05-31) | Effect |
|---|---|
| #5 plain-HTTP SOAP; the SHA-1 hash is the credential | Closed for launcher users: the forwarder uses TLS, and ticket logins carry no reusable password. Stays open for direct clients until D-CT7. |
| #8 session key in cleartext XML | Closed for launcher users (ServerSelection goes over TLS). |
| #4 SID and ticket not IP-bound | Unchanged (WARN only). The launch ticket adds an account-scoped secret in front of it. |
| #3 dev-session mint needs no credential | Can be fixed next: mint the telemetry token from the launcher access token, so `sub` becomes the account rather than an install id the caller chooses. Optional follow-up. |
| #6 unsalted SHA-1 at rest | Not fixed by this design. A no-client-patch option exists: store `argon2id(SHA1HEX(pw))` and migrate on the next login. The client always sends `SHA1HEX`, so this needs neither plaintext nor encryption ADR Phase 2. It amends the ADR, so it is listed as D-CT10. |
| New surface | Rate limits on `/auth/v1/*` and link endpoints; strict redirect validation (no open redirect); CSRF on the callback through `state`; response-size caps on IdP calls; a JWKS cache with refresh on unknown `kid`; clock-skew tolerance; and a kill switch (`CIMMERIA_IDP_DISABLED=1`) like the telemetry one. |
| Telemetry | Spans `auth.idp.authorize`, `auth.idp.callback`, `auth.idp.link`, `auth.launch_ticket.{mint,consume,revoke}` with `provider`, `outcome`, `reason` and latency. No codes, tokens, subjects or emails in log fields: use the account id. Debuggable from SigNoz alone. |

## 6. Launcher design

### 6.1 Sign-in

- A **Sign in** panel with one button per entry in `/auth/v1/providers`, plus "use password in game" for
  servers without OIDC. The first press gives visible feedback ("Opening your browser…"), and the panel shows
  a cancel and a timeout state.
- A loopback listener on `127.0.0.1:0` serves exactly one `GET /cb`, checks `state`, shows a "you can close
  this tab" page and shuts down.
- The refresh token is stored in the OS credential store: Windows Credential Manager or DPAPI, the macOS
  Keychain, and Secret Service on Linux. Never store it in `launcher-config.json`.
- The privacy disclosure (#1999) gains a row: the provider id, subject and display name the server stores.

### 6.2 Play

1. Mint the launch ticket.
2. Start the forwarder on `127.0.0.1:<ephemeral>`.
3. Write `LoginInternal.lua`. The server entry becomes `http://127.0.0.1:<port>`, and `loadServerSystems()`
   also does `Login_AccountEdit:setText(name)` and `Login_PasswordEdit:setText(ticket)`. Optional: auto-submit,
   but only after a lab spike shows that calling `accountLogin` from `startup()` is safe. The name and ticket
   go through the same quote and backslash refusal as `login_servers::valid_field`, and the ticket alphabet
   is alphanumeric by construction.
4. Launch.
5. When the game exits, report the exit to the server (revoking the ticket) and rewrite
   `LoginInternal.lua` without the ticket.

### 6.3 Forwarder rules

- Bind `127.0.0.1` only. Serve only `POST /SGWLogin/UserAuth` and `POST /SGWLogin/ServerSelection`. Cap the
  body at 64 KiB. Forward to the **one** server chosen at Play (it is not an open proxy).
- Upstream: TLS 1.2+ with normal WebPKI verification. Use webpki roots or the platform verifier; no pinning in
  v1 (pinning needs the rollover plan in the encryption ADR). Pass the `Set-Cookie` and `Cookie` headers
  through.
- Reject requests whose `Host` is not exactly `127.0.0.1:<port>`; this stops DNS rebinding. Reject any request
  that carries an `Origin` header, because browsers send one and `SGW.exe`'s curl does not.
- It adds **no credential**, so a local process that reaches it gains nothing it could not get by calling the
  server directly. On Windows, an extra check can match the peer socket's owning PID against the launched
  `SGW.exe` (`GetExtendedTcpTable`). Owning-PID support under Wine is [U], so make the check best effort.
- Lifetime: the forwarder must outlive the login and character-select screens (F14). See D-CT8 for where it
  runs.
- Disk exposure of the ticket (d1): it sits in `LoginInternal.lua` from launch until scrubbed. Anyone who can
  read that file runs as the same user and could read the client's memory anyway. The narrow window,
  the per-launch scope, the "one live ticket" rule and revocation on exit bound the risk. If the owner wants
  it off disk, use d2.

### 6.4 Hosts

Under Wine (macOS and Linux) the guest uses host sockets, so `127.0.0.1` reaches the native launcher's
forwarder [I]. The system browser, loopback redirect and keychain all live on the native side.

## 7. Risks

| Risk | Mitigation |
|---|---|
| `loadServerSystems()` runs before the boxes accept `setText`, or `Login.lua` differs between installs | Spike V2. Fallback: d2, or a full `Login.lua` overlay shipped as a patch set like the Black Market overlay. |
| Relogin re-sends something other than the stored hash, or the ticket is cleared | Spike V3. The ticket is multi-use within its life, so either answer works unless relogin needs the plain text. |
| The launcher is closed while the game is at character select, so the forwarder is gone | D-CT8: run the forwarder in the launch worker, or keep the launcher alive in the tray while its game runs. Either way, show a clear error in the client when it is gone (the existing login-failure prompt). |
| A provider changes its rules (Discord redirect matching, PKCE support, the `openid` scope) | Provider facts are [I]. Verify each one when its provider packet starts. The mock-IdP test suite keeps our side honest. |
| The colo has no public HTTPS for auth (F18) | D-CT4 is a prerequisite for everything else. |
| The persistent schema must stay additive | Two new tables only; no change to `account`. |
| Account takeover through linking | Never link by email, require proof of password for link-existing, rate-limit it, and audit every link and unlink. |
| AV flags the loopback listener | Low. The launcher already opens network connections. A loopback listener is ordinary for OAuth desktop apps. |
| Self-hosted servers without IdP app registrations | Password login stays. `/auth/v1/providers` returns an empty list, and the launcher hides the sign-in panel. |

## 8. Unknowns and spikes

The live spikes need the lab. **Ask the user before any lab use.**

| Id | Question | How |
|---|---|---|
| V1 | Does `LoginMod.servers["X"] = "http://127.0.0.1:<port>"` produce `POST http://127.0.0.1:<port>/SGWLogin/UserAuth`? Does a base URL with a path prefix work (F15)? | Lab: a netcat-style listener and the stock client. |
| V2 | Can `loadServerSystems()` fill both boxes? Is an auto-submit from `startup()` safe? | Lab, with a hand-written `LoginInternal.lua`. |
| V3 | What does Change Server re-send? | Lab with a server log, or x64dbg at `0x00DDED60` during `relogin()`. |
| V4 | `client_ips_match` behaviour when the auth leg uses IPv6 and the Mercury leg uses IPv4 | Server unit test plus one colo check. |
| V5 | Wine: loopback reachability and the owning-PID check | Lab, after V1. |
| V6 | (Confirms § 2.) An `https://` server URL against a TLS 1.2-only endpoint fails | Lab, optional. |
| F22 | How players get accounts today | Owner. |

## 9. Owner decisions (BlockedDecision)

| Id | Decision | Recommendation |
|---|---|---|
| **D-CT1** | Adopt "OIDC in the launcher, plus a loopback TLS forwarder, plus a launch ticket" as the direction, replacing the encryption ADR's Phase 1 and 2 *client* patches for launcher users. | Yes |
| **D-CT2** | Allow the launcher-written `LoginInternal.lua` to pre-fill the login boxes (a client UI behaviour change; d1). Pre-fill only, or auto-submit too? | Pre-fill now; auto-submit after V2 |
| **D-CT3** | v1 providers, and who registers the provider apps and holds the secrets | Discord first, then a generic OIDC slot (Google, Microsoft, Twitch); Steam later. App registrations owned by the project account; secrets in `cimmeria-kv`. |
| **D-CT4** | Public HTTPS for auth on the colo: the in-process rustls listener on `443` or `13443`, or a reverse proxy (Caddy) with ACME. Includes the domain and opening the port at the edge. | Caddy on 443 in front of the auth router (automatic certificates, standard redirect URIs); the in-process listener stays for self-hosters |
| **D-CT5** | Account creation through OIDC: open sign-up, link-existing only, or gated (for example, membership in the Cimmeria Discord server through the `guilds` scope)? | Link-existing plus gated new accounts |
| **D-CT6** | Password policy for linked accounts, and linking of GM and lab accounts | Keep passwords by default with an opt-in "disable password login"; GM and lab accounts are not linkable from the public flow |
| **D-CT7** | When to retire plain-HTTP `8081` SOAP and the legacy-hash path for direct clients (lab, `wireclient`, Codex users) | Not in this campaign; revisit after adoption telemetry |
| **D-CT8** | Where the forwarder runs: the launcher engine (simplest; needs the launcher alive) or the i686 launch worker (lives exactly as long as the game; changes its pinned SHA-256) | Launcher engine, plus "keep running in the tray while your game runs" |
| **D-CT9** | Privacy: storing the provider subject and display name, and the disclosure text | Store the subject and display name only, never the email; update the disclosure |
| **D-CT10** | Wrap legacy SHA-1 hashes as `argon2id(SHA1HEX)` server-side (fixes #6 at rest with no client patch; amends ADR Phase 2) | Yes, as a separate network-security-auth packet |

## 10. Campaign shape once the decisions are made

This answer is viable. Slicing waits on D-CT1, D-CT2, D-CT4, D-CT5 and D-CT8, because they change the packet
set. Expected shape: three waves of about 6 packets each, at the usual 40k-token packet size.

- **Wave 0, spikes (lab, ask first):** V1, V2 and V3, together with the F13 doc fix.
- **Wave 1, server:** the identity crate and provider trait with a mock-IdP test harness; the Discord
  provider; the generic OIDC provider; the two tables and their seed and persistent-list entries;
  `/auth/v1/*` routes with rate limits; the launch ticket in `UserAuth`; telemetry spans.
- **Wave 2, launcher:** the sign-in panel and loopback redirect; credential-store custody; the forwarder;
  the `LoginInternal.lua` writer with the ticket and the scrub on exit; the privacy row.
- **Wave 3, ops and close-out:** colo HTTPS (D-CT4), provider app registration, docs (the encryption ADR
  amendment, `connection-flow.md`, the launcher docs) and a lab UAT of sign-in, Play, Change Server and
  game exit.

## Sources

- [docs/reverse-engineering/findings/auth-and-crypto-modernization-targets.md](../../reverse-engineering/findings/auth-and-crypto-modernization-targets.md)
- [docs/architecture/encryption-modernization.md](../../architecture/encryption-modernization.md)
- [docs/client/sgw-launcher.md](../../client/sgw-launcher.md), [crates/launcher/desktop/docs/launch.md](../../../crates/launcher/desktop/docs/launch.md)
- [docs/operations/colo-deploy.md](../../operations/colo-deploy.md), [docs/operations/persistent-player-data.md](../../operations/persistent-player-data.md)
- `crates/auth/src/auth/{handlers,credentials}.rs`, `crates/common/src/config.rs`, `crates/launcher-core/src/client_setup/login_servers.rs`
- `.claude/agent-memory/network-security-auth/` (audit 2026-05-31, argon2id), the main-session memory note on the #434 RE findings
- Client files (not in git): `Working\SGWGame\Content\UI\Startup\Login\{Login.lua,Login.layout}` and `Working\Binaries\SGW.exe` in the QA install
