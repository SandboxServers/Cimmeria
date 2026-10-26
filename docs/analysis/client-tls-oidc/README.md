# client-tls-oidc: launcher sign-in, TLS login relay and launch tickets

> **Type:** Reference (campaign ledger)
> **Audience:** The owner, the orchestrator and every agent working this campaign
> **Last updated:** 2026-10-26
> **Companions:** [Design](design.md), [contract](contract.md), [work packets](work-packets.md), waves [1](packets-wave1.md) [2](packets-wave2.md) [3](packets-wave3.md) [4](packets-wave4.md) [5](packets-wave5.md) [6](packets-wave6.md) [7](packets-wave7.md) [8](packets-wave8.md) [9](packets-wave9.md) [10](packets-wave10.md), [PR #2031 review ruling](worknotes/pr2031-ruling.md), [release note draft](release-note.md), [resume card](RESUME.md), sibling [mercury-replacement](../mercury-replacement/design.md)

## Purpose

Players sign in through the desktop launcher with an external provider (Discord first, then any OIDC provider an admin configures), and the unmodified 2009 client logs in without ever seeing a reusable password:

- The launcher runs the OAuth flow in the system browser (PKCE, loopback callback), and the server links the provider identity to a game account or creates one (open sign-up, D-CT5).
- At Play, the server mints a per-launch **ticket**; the launcher writes it into `LoginInternal.lua`, and patch 017 (a `Login.lua` and `Login.layout` overlay, no binary patch) shows "Signed in as ..." with an Enter button, auto-submits after spike V2, and shows launcher-written error texts.
- The game's plain-HTTP SOAP login goes to a **loopback relay inside the launcher**, which forwards it over TLS 1.2+ to the server; this also hides the Mercury session key from the network for launcher users (design F19).
- The colo serves `/auth/v1` and `/SGWLogin` over HTTPS through the board's existing Caddy (`board-proxy`, already on 443 for `play.cimmeria.app`), which forwards them to the server's proxy listener on host loopback (D-CT4, amended 2026-10-25).
- Unmodified clients and direct logins keep working on plain 8081 for accounts without a linked identity (D-CT7). An account with any linked identity never has or uses a password (D-CT11, contract C15). Legacy SHA-1 hashes of non-OIDC accounts move to argon2id(SHA1HEX) at rest with no client change (D-CT10, CT-54).

The relay is the same in-process component the [mercury-replacement](../mercury-replacement/design.md) relay extends. D-MR6 is decided (owner, 2026-10-24): the launch ticket (C1) is the session identity on its tunnel, because the OIDC access token never leaves the launcher and the ticket already binds one account to one launch. `RelayConfig.session` stays reserved (`Option<Infallible>`) until that campaign fills it.

## Findings

The design's facts F1-F25 stand ([design § 1](design.md#1-facts)). The architect's findings while slicing:

| # | Finding | Evidence | Packet |
|---|---|---|---|
| A1 | A plaintext TLS login migrates an algo-1 row to algo 2 (argon2id of the plaintext); after that the unmodified client, which sends SHA-1 hex, is refused. Linking now clears the password instead (D-CT11), so only non-OIDC accounts keep a password path. | `credentials.rs` `validate_credentials`, `(ALGO_ARGON2ID, LegacySha1Hex)` arm | CT-21 (amended: no migration), CT-26 (clears on link), CT-54 |
| A2 | Argon2id (64 MiB, t=3) runs synchronously on the async runtime with no bound. Once every legacy login hashes, a login burst stalls the runtime. | `password_hash.rs` | CT-54 (`spawn_blocking`, 4 permits) |
| A3 | The algo-1 hex compare is not constant time. | `credentials.rs` l.190 arm | CT-54 |
| A4 | Host 443 and 80 on the colo are held by the board's Caddy (`board-proxy`, host networking). It already serves `https://play.cimmeria.app`, the login host, with a certificate, the DNS record and the 443 edge forward in place, and forwards five telemetry routes to the server. A second Caddy cannot bind 443; nothing new is needed at the edge. | [CT-50 worknote](worknotes/colo-https-check.md), [telemetry-upload-endpoint.md § The colo route](../../operations/telemetry-upload-endpoint.md#the-colo-route) | CT-51, CT-52 |
| A5 | The board's Caddyfile is host config in no repository (OPS-LT1). A `board-proxy` redeploy from the agent-board repo before it is mirrored there drops every Cimmeria route on it, sign-in included. If it is baked into the board image rather than bind-mounted, only that repo can change it. | [telemetry-upload-endpoint.md § The colo route](../../operations/telemetry-upload-endpoint.md#the-colo-route) | CT-51 (runbook), CT-52 (stops if baked) |
| A6 | The server never reads the TLS cert and key paths from env, so the in-process TLS listener cannot be turned on in the container. | `crates/server/src/main.rs` `config_from_env` | CT-22 |
| A7 | The cell has no DB pool; GM commands that read the database go cell to base, as `.bankdump` does. | `BankCellToBase::GmDump` | CT-44, CT-45 |
| A8 | `docs/client/sgw-launcher.md` is the retired egui launcher's archived page; the desktop launcher's docs live under `crates/launcher/desktop/docs/`. | doc-update-map | CT-05, CT-46 |
| A9 | Files near or over the caps: shell `main.rs` 634 lines, `docs/protocol/login-handshake.md` 646, `credentials.rs` 562, `data/client-patches/README.md` 1019. | line counts on `2019922fe` | CT-41 (3 lines max), CT-46 (3 lines max), CT-54 (tests split out), CT-01 (one row only) |
| A10 | The default login server is a DNS name, so a public certificate can cover it; a server list entry by IP gets no sign-in (discovery is `https://<host>`). | `login_servers::default_servers()` | CT-50, CT-46 |
| A11 | Microsoft's `email` claim is mutable, and GitHub, Twitch and Discord flags are provider-asserted; same-email auto-link is safe only for admin-trusted providers. | design § 5.4; provider docs | CT-11 (config refuses Microsoft trusted), CT-14 |
| A12 | Only seed users exist (all staff or test accounts, never linkable), so nothing migrates and no player claims an account. Accounts carry no email; link/existing (password or GM link code) serves accounts made later without the launcher. | `account` table has no email column; `db/resources/` seed accounts | CT-26, CT-45 |
| A13 | Change Server re-sends the stored credential (F14), so the ticket allows several uses within its life; the relay must outlive the login screens, hence the close guard. | design F14, F24 | CT-20, CT-41 |

The PR #2031 review (2026-10-26) found 19 defects, F01-F19, and six owner items. The [ruling](worknotes/pr2031-ruling.md) maps each to a packet in waves 6-9 or the tail (CT-70..CT-94); its F-numbers are the review's, not the design's. Two external reviews (A and B) then added findings C01-C19; the consolidated architect ruling (`.claude\reports\pr2031\architect-ruling-consolidated.md`) is the **authority** for waves 6-10 and the tail, and [wave 10](packets-wave10.md) holds its packets (CT-95..CT-102). Still one PR, #2031.

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-CT1 | OIDC in the launcher, a loopback TLS relay and a launch ticket, replacing the encryption ADR's Phase 1-2 client patches for launcher users | Decided 2026-10-24 | No binary patch; closes F19 for launcher users |
| D-CT2 | Full login-screen rewrite driven by the launcher: overlay `Login.lua`/`Login.layout` plus `LoginInternal.lua`; hidden boxes, status text, Enter, Use password, launcher error texts; auto-submit after V2 | Decided 2026-10-24 (widened) | No typing, no binary patch |
| D-CT3 | v1 providers: Discord plus one generic OIDC slot; admins bring their own registrations and keys (`config/auth-providers.toml.example`, secrets in env) | Decided 2026-10-24 | Steam (OpenID 2, no email) later |
| D-CT4 | Public HTTPS on the colo: a route in the board's existing Caddy on `play.cimmeria.app` sends `/auth/v1/*` and `/SGWLogin/*` to the proxy listener, published on host `127.0.0.1:13480` by `docker/compose.auth-proxy.yml`; every other path stays 404 (the telemetry routes excepted); no second Caddy | Decided 2026-10-24; amended 2026-10-25 (owner: board Caddy route) | 443 is the board's; its certificate, DNS and edge forward already serve the login host |
| D-CT5 | Open sign-up through provider sign-in, with per-IP and global limits and an optional provider-account age gate | Decided 2026-10-24 | |
| D-CT6 | OIDC-created and linked accounts have no password (owner 2026-10-24); GM, staff and test accounts are never linkable from the public flow; D-CT11 makes the rule strict | Decided 2026-10-24 (owner) | Keeps staff accounts off the public path |
| D-CT7 | The server stays compatible with unmodified clients; plain 8081 and the SHA-1 path are not retired | Decided 2026-10-24 | Lab, wireclient and direct players |
| D-CT8 | The relay runs in the launcher, with a "Keep running / Close anyway" close guard | Decided 2026-10-24 | The launch worker keeps its pinned hash |
| D-CT9 | Store the provider subject, display name and, only from a trusted provider, the verified email; update the privacy disclosure | Decided 2026-10-24 | Needed for safe auto-link |
| D-CT10 | Legacy hashes stored as argon2id(SHA1HEX) (algo 3), as a separate packet | Decided 2026-10-24 | Fixes design § 5.6 #6 at rest with no client patch |
| D-CT11 | An account with any linked OIDC identity never has or uses a password; only non-OIDC accounts (seed users) have one. Link clears the password irreversibly (launcher warning, confirm, audit event); GM link codes follow the same rule; every password-setting path (admin, GM console, seed reload) is refused by DB triggers; Play never falls back to relay-password mode, it shows a launcher error; `last_method` still refuses unlinking the last identity, recovery is a GM link code (contract C15). The opt-in game password (CT-55, CT-59) is Cancelled | Decided 2026-10-24 (owner, strict rule) | No plain-HTTP replayable secret on a linked account |
| D-CT12 | Generated account names: 3-20 of `[A-Za-z0-9_-]`, reserved prefixes refused (C7) | Decided by the architect | The client's name rules |
| D-CT13 | Same-email auto-link only when `email_verified` and the provider is admin-trusted (`trusted_email`); Microsoft is refused as trusted; otherwise the player confirms with a linked login, the account's password (non-OIDC accounts only; cleared by the link) or a GM link code; no email | Decided 2026-10-24 (owner rule) | Account-takeover risk |
| D-CT14 | Refresh tokens: 30-day idle, 180-day family limit, rotation with reuse detection | Decided | |
| D-CT15 | Emailed confirmation link | Deferred indefinitely (owner 2026-10-24) | The colo addresses are on Spamhaus; v1 confirmation is a linked login or a GM link code |
| D-CT16 | Full account deletion (row delete with cascades, or disable and anonymise) | BlockedDecision | Unlink already erases provider data |
| D-CT17 | Website, account and forum links on the login screen come from the server (`[links]`) | Decided | Replaces Project Giza's hard-coded URLs |
| D-CT18 | The login screen ships as its own patch 017, not inside the BM overlay | Decided by the architect | Append-only patches; independent rollback |
| D-CT19 | The relay's upstream leg uses IPv4, like the game's UDP | Decided by the architect | Same source address for the audit's IP checks |
| D-CT20 | Login error texts are written by the launcher into `LoginInternal.lua` | Decided by the architect | No client string tables |
| D-CT21 | Only the first server of the **effective** login-server list gets sign-in and the relay: the list Play writes (a legacy adoption's reviewed list, else saved preferences, else the defaults; contract C18.2) | Decided by the architect; amended 2026-10-26 (review F07, CT-90) | One sign-in per launcher; sign-in and Play must name the same server |
| D-CT22 | On the colo, `AUTH_PROXY_TRUSTED_PEERS` is narrowed to the `signoz-net` gateway (`/32`, in the colo `.env` only); if the host check then fails, CT-52 restores `.env` and returns blocked (since D-CT26 = b the default trusts loopback only, so there is no wider list to fall back to) | Decided by the architect | Keeps the SigNoz containers from speaking for the proxy |
| D-MR6 | Session identity on the mercury-replacement tunnel: the launch ticket | Decided 2026-10-24 (owner; shared with mercury-replacement) | `RelayConfig.session` reserved here; mercury-replacement fills it |
| D-CT23 | Stored email authority (review owner item 1, C01): refresh or clear the stored email and `email_verified_at` on every sign-in, and accept only identities whose provider is trusted now (option a; CT-94) | Decided 2026-10-26 (owner, option a) | A stale email kept its auto-link authority after the provider changed it |
| D-CT24 | ID-token claim policy: `iat` required and refused more than 60 s in the future; `azp` required with several audiences and equal to the client id when present; `nbf` checked with the same 60 s leeway (amended 2026-10-26, C09) (contract C18.3) | Decided by the architect 2026-10-26 (CT-84) | OIDC Core 3.1.3.7; no provider we target sends a token this refuses |
| D-CT26 | Code default of `auth_proxy_trusted_peers` is loopback only (127/8, ::1); the compose overlay sets the bridge gateway (option b; CT-101) | Decided 2026-10-26 (owner, option b) | A fresh host fails closed instead of trusting every Docker bridge (C04) |
| D-CT27 | Launch ticket: 5 min life until its first SOAP redemption, then the current 30 min idle and 12 h absolute (option b; CT-102) | Decided 2026-10-26 (owner, option b) | A stolen unredeemed ticket cannot be kept alive for 12 h (C05); no false refusals |
| D-CT28 | Argon2 on the link path runs under `spawn_blocking` behind its own 2-permit semaphore (CT-99), separate from CT-54's 4-permit SOAP gate (D-CT30) | Decided by the architect 2026-10-26 | Argon2 ran inline on tokio workers (C15) |
| D-CT29 | OIDC and the provider-age gate (C10): (a) document only, (b) when the gate is on, refuse sign-up from a provider that reports no creation time unless marked exempt, (c) drop the gate. Recommendation (b); joins the CT-53 item | BlockedDecision (owner) | OIDC tokens carry no account age, so only Discord is age-gated today |
| D-CT25 | Launcher credential-store failures are returned, never dropped: a refused write keeps the session for this run only (`session_only`), a refused delete tombstones the host for this run (`sign_out_incomplete`), an unreadable store restores nothing (`store_unreadable`); each shows a panel warning (contract C17.3, C18.1) | Decided by the architect 2026-10-26 (CT-88) | A lost rotated token trips reuse revocation; an undeleted entry undoes Sign out |
| D-CT30 | Two argon2 gates, not one: the link-form check keeps its 2-permit gate in `password_check.rs` (CT-99); the SOAP credential path gets CT-54's 4-permit gate in `password_hash.rs`; neither takes the other's permit (at most 6 verifies, about 384 MiB, at once). The link check verifies algo-3 rows from wave 10 on (contract C14 amended; CT-54 amended) | Decided by the architect 2026-10-26 (wave 10 ruling) | A `/link/existing` flood cannot queue game logins; CT-54 cannot break link-by-password |

## Packets

Status values: Ready, BlockedDependency, BlockedDecision, Writing, Review, Integrated, UATPending, Done, Cancelled (decision removed the packet), Deferred (decision parked with no date).

| Packet | Title | Wave | Status |
|---|---|---|---|
| CT-01 | Overlay patch 017: `Login.lua`, `Login.layout` | 0 | Ready |
| CT-02 | Lua logic UAT for 017, and CI | 0 | BlockedDependency (CT-01, CT-03) |
| CT-03 | `LoginInternal.lua` ticket rendering | 0 | Ready |
| CT-04 | Lab spike V1-V3 | 0 | Ready (lab approved in principle; confirm the lab is free and the MCP is up) |
| CT-05 | Doc fixes: F13, encryption ADR amendment | 0 | Ready |
| CT-10..CT-16 | Schema, identity crate, providers, linking, repositories | 1 | Ready (CT-10, CT-11, CT-14 first; the rest BlockedDependency inside the wave) |
| CT-20..CT-26 | Ticket store, UserAuth tickets, proxy listener, stores, routes | 2 | BlockedDependency (wave 1) |
| CT-30..CT-35 | Account routes, wiring, relay crate, engine sign-in, Play | 3 | Merged (wave 3 fix pass 1 merged; campaign head 53ec9287d) |
| CT-40..CT-46 | Sign-in driver, shell, frontend, GM `.account`, docs | 4 | Merged (wave 4 fix passes 1-2 plus architect ruling merged on the campaign; head 0ea4a42df; owner added: self-unlink revokes other sessions, base GM check) |
| CT-50 | Colo read-only check | 5 | Done (2026-10-25, [worknote](worknotes/colo-https-check.md)) |
| CT-51 | Auth proxy overlay and runbook (board Caddy route; re-sliced 2026-10-25) | 5 | In merge queue (PR #2030, green CI, reviewed `ship`) |
| CT-52 | Apply on the colo: overlay, then the board Caddy route (`hands-on` agent) | 5 | BlockedDependency (CT-51 merged; the campaign released in the colo image); no owner gate |
| CT-53 | Register providers | 5 | BlockedDecision (owner) |
| CT-54 | Password algo 3 | 5 | BlockedDependency (wave 2) |
| CT-55 | Server game-password routes | 5 | Cancelled (D-CT11) |
| CT-56 | Full account deletion | 5 | BlockedDecision (D-CT16) |
| CT-57 | Auto-submit on | 5 | BlockedDependency (CT-04 V2) |
| CT-58 | Build, lab-load, publish 017 | 5 | BlockedDependency (wave 0, CT-04; owner signing) |
| CT-59 | Launcher game-password UI | 5 | Cancelled (D-CT11) |
| CT-U1..U6 | Live UAT | 5 | BlockedDependency (CT-52, CT-53, CT-58; lab approved in principle) |
| CT-70..CT-76 | Revocation model, server half (F01-F04, contract C16) | 6 | Ready (CT-70, 71, 72, 74 at once; then CT-73, 75, 76) |
| CT-77..CT-82 | Ticket guards, handoffs, name lookup, triggers, test cleanup (F05, F13, F14, F19) | 7 | BlockedDependency (wave 6 merged) |
| CT-83..CT-85 | OIDC token-endpoint method, claim policy, relay CI and secret forwarding (F15, F18, D-CT24) | 8 | Ready |
| CT-86..CT-91 | Launcher host slots, epoch, store failures, browser fallback, panel controls (F06-F12, F17, D-CT25) | 9 | Ready (CT-90 and CT-91 at once; then the chain CT-86, 87, 88, 89) |
| CT-95 | Proxy trusted-peers parse fails closed (C04) | 10 | Ready |
| CT-96 | Peek the link code, redeem it last (C02) | 10 | BlockedDependency (CT-73, CT-82 merged) |
| CT-97 | Password check without oracles (C08) | 10 | Ready |
| CT-98 | Limiter keeps account windows (C06) | 10 | Ready |
| CT-99 | Argon2 off the async workers, two at a time (C15, D-CT28) | 10 | BlockedDependency (CT-97 merged) |
| CT-92 | Campaign record in the PR (F16); amended with the accepted-finding documentation | tail | Docs written ([worknote](worknotes/CT-92.md)); review pending |
| CT-93 | Reauth provider chooser | tail | Deferred |
| CT-94 | Stored email authority (D-CT23 = a; sentinels `0x7000_EBC0..0x7000_EBFF`) | tail | Ready |
| CT-100 | `main.rs` env-table row after CT-95 and CT-101 | tail | Deferred (launcher-cache-presync owns `main.rs`) |
| CT-101 | Loopback-only default proxy peers (D-CT26 = b) | tail | BlockedDependency (CT-95 merged) |
| CT-102 | Short unredeemed ticket life (D-CT27 = b) | tail | BlockedDependency (CT-77 merged) |
| CT-60 | Close-out | 5 | BlockedDependency |

## Still open for the owner

1. OPS-LT1 (an existing owner/ops task): after CT-52, mirror the live board Caddyfile into the agent-board repo; a `board-proxy` redeploy before that drops both the telemetry and the sign-in routes.
2. CT-53: provider app registrations under the project account, secrets in `cimmeria-kv`, which providers are `trusted_email`, and the age gate.
3. D-CT16 (full deletion semantics).
4. (D-CT14 and D-CT17 are Decided.) Accepted trade-offs to keep in mind: W3-1 fix b, W3-2, W3-7; the caller's own pending launch ticket is also revoked on self-unlink.
5. A privacy policy page for open sign-up (the launcher's disclosure covers the launcher; the server's data needs a public page, linked through `[links]`).
6. D-CT29 (OIDC and the provider-age gate; decide it with CT-53). D-CT23, D-CT26 and D-CT27 were decided 2026-10-26 (CT-94, CT-101, CT-102 run).
7. Release gates from the PR #2031 review: the live client run (CT-04, CT-58, CT-U1..U6), the native Windows Credential Manager and macOS Keychain proof (CT-U6, now including the CT-88 failure paths), and what to do with ticket files a crashed launcher leaves behind.

## What did we miss

| Gap | Answer in this campaign | Residual |
|---|---|---|
| Existing accounts and their emails, migration, claiming | Only seed users exist (staff or test, never linkable), so nothing migrates and nobody claims. A later non-OIDC account links with its password (cleared by the link) or a GM link code (`.account linkcode`) | A player who forgot the password needs a GM |
| Steam and other providers without email | Not in v1; the provider trait allows them later (D-CT3). Battle.net and Epic also give no email; they would always go through `needs_link` | Steam is OpenID 2, not OIDC: its own provider kind |
| Providers without a reliable verified flag | `trusted_email = false` makes the email a hint only; Microsoft cannot be trusted (config error) | An admin who marks a weak provider trusted accepts takeover risk; the example file says so |
| Bans and GM levels across identities | Bans and `accesslevel` stay on the account; every identity of a disabled account is refused and its refresh families revoked; staff accounts are never linkable | A banned player can sign up again with a fresh provider account; mitigations are the age gate and IP limits |
| Recovery, unlinking the last method | `last_method` counts identities only (a linked account has no password) and refuses removing the last one; recovery is another linked provider or a GM link code | No self-service recovery (D-CT15 deferred) |
| GDPR deletion | Unlink erases the only provider data (`account_identity` row); events hold no subject, email or IP | Full deletion is D-CT16 |
| Provider outage | Refresh tokens keep players signed in for up to 30 days idle; Play needs only the server | A new sign-in waits for the provider |
| Multiple accounts per person | One identity maps to one account; a person may still hold several accounts through several identities | Sign-up limits only |
| Rate limits and sign-up abuse | C7 buckets per IP, per account name and global; age gate; kill switches | In-memory: a restart resets them |
| Account names | Generated 3-20 of `[A-Za-z0-9_-]`, reserved prefixes refused, case-insensitive unique | Players cannot pick a name in v1 |
| The launcher is required | For linked accounts, yes (D-CT11); a failed ticket mint shows a launcher error, never a password fallback. The stock password path on 8081 (D-CT7) serves only non-OIDC accounts | A linked player cannot play without the launcher |
| The d3 one-time code option (type a short code into the game) | Assessed: low value. Linked accounts play through the launcher by design, and a typed code would be replayable over plain HTTP | Not built |
| Wine and macOS | The relay listens on host loopback, which Wine should reach (V5, unverified); credentials in the Keychain (`keyring` `apple-native`) | CT-U5; there is no Linux launcher |
| Server restart | Tickets live in memory, so a restart ends them; the launcher's next Play mints a new one; refresh tokens are in the database | A player at character select must press Play again |
| Self-hosters | Without 443 or the in-process TLS listener, the launcher never offers sign-in and Play works as today | Documented in CT-51's runbook |
| External-review residuals (accepted, documented by CT-92) | C07 `/link/existing` lockout, C10 OIDC age gate, C11 live token after kill switch, C14 pending-store eviction | Availability-only or documented; see the consolidated ruling |
| Telemetry | Spans and launcher events carry outcome and reason only (C11) | Adoption is visible through `launcher.sign_in.*` counts |
