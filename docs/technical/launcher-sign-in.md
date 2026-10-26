# Launcher sign-in and linked accounts

> **Type:** Explanation plus reference
> **Audience:** Engineers and operators who touch the login path
> **Last updated:** 2026-10-26
> **Companions:** [Login handshake](../protocol/login-handshake.md), [Connection flow](../connection-flow.md), [GM commands: `.account`](../commands.md), [Player guide](../client/launcher-sign-in-guide.md), [Launcher relay and ticket](../../crates/launcher/desktop/docs/sign-in.md), [Auth over HTTPS](../operations/auth-https.md)
> **Campaign record:** [client-tls-oidc ledger](../analysis/client-tls-oidc/README.md) and its [shared contract](../analysis/client-tls-oidc/contract.md). The contract holds the exact wire shapes; this page explains the flow and what you can rely on.

## Status: what runs, what has never run live

Everything below is merged code with unit and live-database tests. Three things have not been run against the real game and are not verified:

- **The client overlay in the live game** (patch `017-launcher-sign-in`): whether the stock `Login.lua` accepts the rewritten `LoginInternal.lua` and shows the signed-in screen. The ticket is pre-filled into the stock login boxes, so the player may need to press Login. Auto-submit stays off (`TICKET_AUTO_SUBMIT = false`).
- **The relay under Wine** on macOS and Linux.
- **The packaged keyring** on Windows (Credential Manager) and macOS (Keychain). The code path is covered by an in-memory store in tests; the real stores have not been exercised from a packaged build. Off Windows and macOS the store is memory only, so a sign-in on Linux or WSL ends when the launcher closes.

The live lab UAT has not happened. Treat the sections below as a description of the code, not of observed behavior.

## The flow

```mermaid
sequenceDiagram
    participant P as Player
    participant L as Launcher
    participant B as System browser
    participant S as Auth server (/auth/v1, TLS)
    participant I as Provider (Discord, OIDC)
    participant G as Game client

    P->>L: Sign in with <provider>
    L->>S: GET /auth/v1/providers (discovery)
    L->>B: open /authorize (launcher PKCE challenge, state, loopback redirect)
    B->>S: /authorize
    S->>I: redirect (server PKCE, state, nonce)
    I->>S: /callback/{provider}?code
    S->>B: 302 to http://127.0.0.1:port/cb?code=H (handoff code)
    B->>L: loopback callback
    L->>S: POST /token (code, code_verifier)
    S->>L: signed_in, or needs_link
    P->>L: Play
    L->>S: POST /launch-ticket (Bearer, launch_id)
    S->>L: account_name, ticket
    L->>L: write LoginInternal.lua (ticket mode), start relay
    G->>L: POST /SGWLogin/UserAuth (loopback relay)
    L->>S: same request over HTTPS
    S->>G: login result (ticket spent over TLS)
    G-->>L: game exits
    L->>S: POST /launch-ticket/end, scrub LoginInternal.lua, stop relay
```

Step by step:

1. **Discovery.** The launcher asks the first login server for `GET https://<host>/auth/v1/providers` (4 s timeout, cached 10 minutes). No answer, or no `api: 1`, means sign-in is unavailable and the launcher behaves as before.
2. **Browser sign-in and PKCE handoff.** The launcher binds `127.0.0.1:<random>`, opens the system browser at `/authorize`, and waits up to 5 minutes. The server is a confidential OAuth client of the provider with its own PKCE and `state`. It returns a single-use 60-second handoff code to the launcher's loopback address. The launcher redeems it at `POST /token` with its PKCE verifier, so a code stolen from the redirect is useless without the verifier.
3. **`needs_link`.** If the provider identity is not bound to an account and auto-link does not apply, `/token` answers `needs_link` with a 10-minute single-use `link_grant`. The player chooses: create a new account, link an existing one with its password, or link with a GM code.
4. **Play.** The launcher mints a launch ticket for this launch, starts the loopback relay for the first login server, and writes a one-server `LoginInternal.lua` pointing at the relay. The game talks plain HTTP to `127.0.0.1`; the relay re-sends the request over HTTPS, so the ticket never crosses the network unencrypted.
5. **Heartbeat.** While the game runs, the launcher heartbeats the ticket every 10 minutes. Three failures in a row stop the heartbeats quietly; the game may already be in the world.
6. **Exit and scrub.** When the game exits, the launcher first writes the plain server list back over `LoginInternal.lua` (so no ticket stays in the install), then ends the ticket on the server, then stops the relay. If the launcher is closed while a relayed game runs, a "Game still connected" prompt offers "Keep running" or "Close anyway".

## Linking rules

Source: `crates/identity/src/link.rs` (pure functions, C7 in the contract).

| Situation | Result |
|---|---|
| The provider identity is already bound to an account | Signed in to that account (refused with `account_disabled` if the account is disabled) |
| The provider is `trusted_email`, the email is verified, exactly one account holds a trusted verified identity with the same email, and that account is linkable | Auto-link |
| Anything else | `needs_link`, with `hint = email_match` only when a trusted, verified email matches at least one account |

A linkable account is enabled, has `accesslevel < 1` and is not a test account. Staff and test accounts cannot be linked, by any path. The stored email is refreshed on every sign-in (cleared when the sign-in carries no verified email or the provider is no longer `trusted_email`), so only a provider trusted now can auto-link.

**Residual risks.** The provider's email is only as trustworthy as the provider: that is why `trusted_email` is a per-provider setting and why config loading refuses `trusted_email = true` for Microsoft's consumer endpoints (the `email` claim there is mutable). A player who loses every provider has no self-service recovery; a GM issues a link code. Rate limits are in memory and reset on restart.

**Guessing defences and what they cost.** The per-account `/link/existing` limit is 5 attempts per 15 minutes, which is about 480 guesses a day, and the link-code route has a server-wide cap of 50 per 15 minutes (a code is 50 bits, so the cap bounds guessing even behind spoofed IPs). Both are availability trade-offs you should know about:

- Anyone holding a link grant can lock a given account name out of password linking for 15 minutes. The owner retries after the window, or a GM issues a link code.
- Anyone can use up the server-wide link-code cap for 15 minutes. Link codes then fail for everyone until the window ends.
- There is no per-grant cap. Grants are free to mint, so a cap on them would bound nothing the account limit does not already bound. The limiter never evicts a live account window to make room (a full map refuses the new key instead).

**Provider age gate.** `[limits] min_provider_account_age_days` (default 0, off) applies to Discord only. The OIDC provider path reports no account age, and the gate treats an unknown age as old enough, so an OIDC sign-up skips it. Whether a configured gate should refuse providers that report no age is an open owner decision (D-CT29); until it is made, do not rely on the gate for OIDC providers.

### No passwords on linked accounts (D-CT11)

- A linked account has no password. Linking an existing account with its password (or with a GM link code) clears the password in the same transaction as the link, and **cannot be undone**. The launcher asks the player to confirm, and the server refuses a link without `password_removal_ack: true`.
- Two database triggers (`account_password_guard`, `account_identity_password_guard`) refuse any write that would give an account both a password and an identity, whatever the path (SQLSTATE `23514`). The identity guard locks the account row, so concurrent writers serialize: a password set waits for an identity insert that is still open, and the other way round. It also covers moving an identity to another account (`UPDATE account_identity SET account_id`), not only inserts.
- Play never falls back to a password. A signed-in Play that cannot get a ticket ends as `NotStarted` with a reason shown in the panel (see Failure modes).
- The last sign-in method cannot be removed (`409 last_method`). Recovery is `.account linkcode <name>`, then "Link with a code" with a new provider.
- Removing a method needs a sign-in made by an authorization-code grant in the last 10 minutes (`reauth_required` otherwise); so does adding one.

## What stays compatible

Accounts **without** a linked identity (today the seed accounts) keep every old path, unchanged:

- Unmodified clients keep logging in over plain HTTP on the existing port.
- The legacy SHA-1 hex credential and plaintext password verification are untouched (D-CT7).
- A launcher that is not signed in, or a server with no `/auth/v1`, uses the old server list. When the server offers the relay and the player is not signed in, the relay carries the player's typed password over HTTPS (relay-password mode).

A linked account is the exception: it has no password, so an unmodified client cannot log into it. That is deliberate.

## Launch tickets

- 32 random alphanumeric characters. The client sends `SHA1HEX(ticket)` as the password field, which the existing credential classifier already treats as a legacy hash. The server stores only `SHA-256` of that hex and compares in constant time.
- One live ticket per account; minting replaces the old one. Life: 5 minutes until the first SOAP redemption (an unredeemed ticket is not extended by heartbeat); after that 12 hours absolute, 30 minutes idle (use or heartbeat refreshes the idle clock). A reconnect or Change Server inside the life re-sends the same ticket.
- **TLS only.** A matching ticket that arrives over plain HTTP is refused with audit outcome `ticket_requires_tls` and the ticket is revoked. Successful use is audited as `launch_ticket`.
- Tickets live in server memory. A server restart clears them; the launcher has to mint a new one on the next Play.

## Revocation model

One server call, `revoke_account`, cuts an account off. What it reaches, and what it does not (the exact rules are [contract C16](../analysis/client-tls-oidc/contract.md)):

| Reached | How |
|---|---|
| Refresh tokens | Every refresh family of the account is revoked in the database. Self-service method removal keeps the caller's own family. |
| Access tokens | The in-memory access store drops the account's tokens. |
| Launch tickets | The account's ticket is revoked. |
| Handoffs the ticket issued | Each login a ticket lets through carries the ticket's guard. Revoking flips the guard, so a SOAP login that re-checks it after the revoke is refused. |
| **Not reached** | A game connection that is already established. The ticket gates login only, so a player who is already in the world stays there until they leave or a GM acts. The `.account unlink` reply says so. |

Which events use it:

- **GM `.account unlink` and self-service method removal** revoke the whole set above.
- **Refresh-token reuse** revokes the reused token's family, that family's access tokens and the account's launch ticket. The account's other refresh families stay signed in: only one device is proven stolen, so a player's second device keeps working and the stolen one is cut.
- **`POST /logout`** revokes that token's refresh family.
- **A disabled account** is not swept. It is refused at each use (every Bearer route, ticket mint, heartbeat and ticket login), so a live access token cannot reach the game. Only unlink sweeps the account's sessions.

## `/auth/v1` routes in short

All JSON, TLS only (`403 https_required` otherwise), all `Cache-Control: no-store`, all rate limited. Exact request and response shapes: [contract C6](../analysis/client-tls-oidc/contract.md).

| Route | Purpose |
|---|---|
| `GET /providers` | Discovery: `api`, `soap_relay`, `signup_open`, provider list, optional links |
| `GET /authorize`, `GET /callback/{provider}` | Browser leg; redirect URIs are loopback only |
| `POST /token` | Authorization-code or refresh grant; `signed_in` or `needs_link` |
| `POST /link/new`, `POST /link/existing` | Finish a `needs_link` |
| `POST /link/start` | Begin adding a method to a signed-in account |
| `GET /account`, `DELETE /identities/{id}` | Account view; remove a method; ends the account's other sessions (refresh families, access tokens, launch tickets) except the caller's own family |
| `POST /launch-ticket`, `/launch-ticket/heartbeat`, `/launch-ticket/end` | Ticket life cycle |
| `POST /logout` | Revoke a refresh token |

Access tokens last 15 minutes; refresh tokens rotate on every use and a reused one revokes its whole family. A disabled account is refused on every Bearer route.

## Kill switches and environment

Provider list: `config/auth-providers.toml` (path from `CIMMERIA_AUTH_PROVIDERS`); only `config/auth-providers.toml.example` is in git. Provider secrets come from the environment variable each provider names in `client_secret_env` (for example `CIMMERIA_IDP_DISCORD_SECRET`); a literal secret in the file is a fatal config error. For an `oidc` provider, `token_endpoint_auth_method` sets how the secret reaches the token endpoint: `client_secret_basic` (the default, an `Authorization: Basic` header) or `client_secret_post` (the form body); any other value is a fatal config error, and an issuer whose discovery document lists token endpoint auth methods without the configured one fails the sign-in before the browser opens (`auth_method_unsupported`). The `id_token` must carry `iat`, which may be at most 60 s ahead of the clock; `nbf` is checked with the same 60 s leeway; `azp` is required when `aud` lists several audiences and must equal the client id when present.

| Variable | Effect |
|---|---|
| `CIMMERIA_IDP_DISABLED=1` | No providers; every `/auth/v1` route except `providers` and `launch-ticket*` answers `503 disabled` |
| `CIMMERIA_IDP_SIGNUP_DISABLED=1` | `signup_open = false`: no new accounts, linking still works |
| `CIMMERIA_SOAP_RELAY_DISABLED=1` | `soap_relay = false`: the launcher starts no relay |
| `AUTH_TLS_CERT_PATH`, `AUTH_TLS_KEY_PATH` | In-process TLS listener |
| `AUTH_PROXY_ADDR` | Extra listener for a TLS-terminating reverse proxy; trusts `X-Forwarded-*` only from `AUTH_PROXY_TRUSTED_PEERS` (default `127.0.0.0/8,::1`, loopback only) |

**The kill switch stops new sign-ins, not tokens already issued.** `/providers` and `/launch-ticket*` are exempt from `CIMMERIA_IDP_DISABLED`, so an access token that is still live (15 minutes at most) can mint a launch ticket after you set it. That is deliberate: the switch is for the sign-in surface. To cut a compromised account, use `.account unlink` (see the revocation model above). Whether the switch should also clear the access store is an open owner choice.

**Deploying the proxy listener.** On the colo, name the proxy gateway in `AUTH_PROXY_TRUSTED_PEERS` before the first deploy (D-CT26 = b: the code default is loopback only, so an unlisted gateway's `X-Forwarded-*` headers are ignored and every request reads as plain HTTP). Steps: [Auth over HTTPS](../operations/auth-https.md).

How to terminate TLS and deploy: [Auth over HTTPS](../operations/auth-https.md), [colo deploy](../operations/colo-deploy.md), [container](../operations/container.md).

## What the launcher reports

The panel reads one status object. Three parts of it matter when you debug a sign-in.

**`storage_warning`.** `null` or one of three values, set when the OS credential store misbehaves. The panel shows these texts:

| Value | Text |
|---|---|
| `session_only` | "Signed in for this session only: this computer's credential store would not save the sign-in." |
| `sign_out_incomplete` | "Signed out here, but the saved sign-in could not be removed from this computer's credential store. It will not be used again in this session." |
| `store_unreadable` | "This computer's credential store could not be read, so no saved sign-in was restored." |

**`resume`.** When the system browser will not open, the flow parks instead of failing: the panel shows the authorize address to copy into a browser, and the launcher sends `resume`, which completes the parked flow as the original sign-in or Add method would have. Nothing parked answers `no_pending_flow` ("That sign-in expired. Sign in again."). Cancel and the flow's 5-minute limit clear a parked flow.

**Which server sign-in uses (CT-90).** The first server of the effective list, the same list Play writes: the adopted list of a legacy-adopted installation, else saved preferences, else the defaults. While an install, update or repair runs, or when the installation cannot be read, the launcher falls back to nothing: Inspect and Cancel answer the held sign-in in the offline shape, Sign out works with no server, and every other command answers `unavailable`.

A signed-in player whose server cannot be reached keeps the account summary and **Sign out** on screen; the panel hides only network actions. In the `needs_link` step, **Back to sign in** sends `sign_out`.

## Telemetry

Spans (`crates/identity`, target `cimmeria_identity`): `auth.idp.authorize`, `auth.idp.callback`, `auth.idp.token`, `auth.idp.link`, `auth.idp.link_start`, `auth.idp.unlink`, `auth.launch_ticket.mint`, `auth.launch_ticket.heartbeat`, `auth.launch_ticket.end`. `crates/auth` emits `auth.launch_ticket.consume` where the ticket is spent. Fields are limited to `provider`, `outcome`, `reason`, `account_id` and `latency_ms`: never a code, token, ticket, subject, email, display name or IP.

The auth reaper writes one DEBUG line `Reaped expired auth entries` with the counts `expired_sessions`, `expired_tickets` and `expired_launch_tickets`, only when a count is above 0.

Launcher events: `launcher.sign_in.{started,completed,failed}` and `launcher.relay.{started,stopped}` (also `ticket_failed`, `relay_start_failed`, `scrub_failed`, `end_failed`, `heartbeat_stopped`, `close_prompt`), each with outcome and reason; the relay `started` and `stopped` events also carry `mode`. No event carries a ticket, token, code or password.

GM `.account` commands log one `gm_action` event on the `account_admin` target: `action`, `result`, a stable `reason` on refusals, and the GM and target ids. A refusal the GM caused is INFO; one the server caused (no pool, a failed query) is WARN. A non-GM caller is logged with `reason=not_gm`; a closed base channel with `reason=base_channel_closed` (WARN).

Both targets (`cimmeria_identity`, `account_admin`) have a row in the [observability target catalog](../architecture/observability-target-catalog.md).

## GM recovery: `.account`

`.account identities <name>`, `.account unlink <name> <identity_id>`, `.account linkcode <name>`. GM-gated; the cell forwards to the base, which reads the identity tables (the `.bankdump` shape). Output never shows a subject or email. `unlink` revokes the account's refresh tokens, access tokens and launch tickets, so the removed identity's sign-ins stop working; a game session already running is not ended (the reply says so). Full entry: [commands.md](../commands.md). A link code is 10 characters, valid 15 minutes, single use, replaces any older code for that account, and is held in process memory (a restart voids it).

## Failure modes

| What happens | What the player sees and why |
|---|---|
| Provider outage or slow provider | Sign-in fails in the browser or times out after 5 minutes; nothing is stored |
| Server unreachable at Play | Play stops, "could not get a sign-in for this launch" (`ticket_unavailable`) |
| Stored sign-in refused (`invalid_grant`, disabled account) | "Your sign-in has ended" (`sign_in_ended`); sign in again |
| Server restarted while the game ran | Tickets are in memory, so the game's next login with the old ticket fails (error 4); close the game and press Play |
| Rate limited | `429 rate_limited` with `retry_after`; shown as a wait message |
| `soap_relay` off, relay will not start, or discovery fails while signed in | Play stops (`ticket_unavailable`); it never offers a password |
| Credential store unavailable | Not exercised: the real Windows and macOS stores have not been run from a packaged build, so this failure's behavior is unverified |

## Known limits

Accepted by the security passes; none lets one player reach another's account. Listed so you do not rediscover them.

| Limit | Effect | Where |
|---|---|---|
| Kill switch leaves ticket minting open | A live access token can still mint a ticket for up to 15 minutes after `CIMMERIA_IDP_DISABLED=1` | `routes/mod.rs` `always_open` |
| Password-link lockout is keyed by account name | Five failed attempts at one name lock that name for 15 minutes, whoever sent them | `routes/link.rs`, `limits.rs` |
| A failed dummy hash would be a timing oracle | Unknown names and legacy rows spend a dummy argon2id verify; if hashing the dummy ever failed, the verify would return at once. Not reachable today | `password_check.rs` |
| Two argon2 gates (D-CT30) | The link check holds 2 permits. The SOAP credential gate of 4 arrives with CT-54 and is not on this branch yet; with both in place at most 6 argon2 verifies run at once, and a request past its gate waits without holding memory. A link check that waits 5 seconds fails closed | `password_check.rs` |
| The proxy listener warns once per request from an untrusted peer | Log volume only | `proxy.rs` |
| The launch-ticket re-check after the login-map removal has no hook test | A race with no `.await`; the peek-stage check is tested, the second check is not | `login/mod.rs` |
| The pending sign-in store evicts oldest-first at 10,000 entries | An attacker past the Authorize limit (20 per 10 minutes per IP) can push out other players' pending sign-ins, who then start again. Availability only. Reaping expired entries before evicting is deferred | `stores.rs` |
| Limiter state is in memory | A restart resets every window and drops pending links and link codes | `limits.rs` |
| A stale proxy-peers comment | The comment at the top of the server `main.rs` still describes the old trust list; CT-100 fixes it | `crates/server/src/main.rs` |
| Cancel racing a completed sign-in | A Cancel that lands after the slot and keyring are written but before the state installs replies Cancelled, and the next Inspect shows signed in. The state is consistent and the session is the player's own | launcher `session.rs` |
