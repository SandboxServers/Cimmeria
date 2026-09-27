# Batch sec-b — security issues triage (main-ro @ 059d6038)

Issues: #469 #470 #471 #472 #473 #474 #476 #477 #443 #439 #447 #448 #434 #532 #294

Note for the executor: the audit findings files now live on main at
`docs/security-audit/2026-05-31-server-authority/findings/CAT-*.md`. Every category issue body still
links the `worktree-server-authority-audit` branch. Rewritten bodies below point at `main`.

---

## #439 — [Critical] Admin API on 0.0.0.0:8443 has zero authentication

- Verdict: REWRITE
- Priority: P0
- Labels: add `ready-for-human` (the compose/publish change is an operator/maintainer call). Keep `bug,security`.
- Summary: Step 1 of the issue (a loopback bind) landed in PR #724. The binary now binds the admin API to `127.0.0.1` by default. **The exposure is still live on the shipped colo deployment.** The container image sets `ADMIN_BIND=0.0.0.0`, and `docker/compose.yml` publishes `"8443:8443"` on every interface. So the unauthenticated API (stop server, content rewrite/delete, `/ws/logs`) is still reachable from the internet on a default colo deploy. `colo-deploy.md` only warns about it. JWT middleware (step 2) is untouched: the TODO is still in `middleware.rs`, `/api/auth/login` still returns "not implemented", and CORS is still `Any`. The credential-harvest part of vector 4 is reduced because PR #698 (#440) stopped logging full SIDs, tickets and the SOAP body. Launcher telemetry (`/api/auth/dev-session`, `/api/telemetry/*`, HMAC-gated with a quota since #740) shares the listener. That shared listener is why the publish was never narrowed.
- Evidence:
  - `crates/server/src/main.rs:21` (env-table row), `:193` `format!("{admin_bind}:{admin_port}")`; `crates/common/src/config.rs:86` `admin_bind` (PR #724)
  - `docker/Dockerfile:269-274` `ADMIN_BIND=0.0.0.0` with a comment saying the publish is the exposure control
  - `docker/compose.yml:64` `- "8443:8443"  # admin REST API (TCP) — telemetry ingest lives here` (all interfaces)
  - `docs/operations/colo-deploy.md:39` warning ("remove 8443 … until the JWT middleware lands")
  - `crates/admin-api/src/middleware.rs:12-17` (`allow_origin(Any)`), `:19-28` (`// TODO: JWT authentication middleware`)
  - `crates/admin-api/src/routes/auth.rs:44` `"message": "not implemented"`
  - `crates/admin-api/src/routes/config.rs:191-213` `/api/config/stop` → `orchestrator.stop_all()`
  - Memory note: `jsonwebtoken` is a declared but unused dependency (PR #641 agent-memory)
- Related/duplicates: #441 (dev-session quota, PR #740), #440 (log redaction, PR #698), #460 CAT-A

### Action text

Comment:
> Status re-check against main @ 059d6038. PR #724 made the binary default to a `127.0.0.1` admin bind. **The colo deployment is still exposed**, though. The image sets `ADMIN_BIND=0.0.0.0` (`docker/Dockerfile:269-274`) and `docker/compose.yml:64` publishes `8443:8443` on every interface, so on a default colo deploy the unauthenticated admin API (`/api/config/stop`, `/api/editor/content`, `/ws/logs`) can still be reached by anyone. The JWT step is untouched: `middleware.rs` still has the TODO, `/api/auth/login` returns "not implemented", and CORS is `Any`. I rewrote the body to track what is left: (1) close the colo publish, either by moving the launcher telemetry routes to their own listener or port or by publishing `127.0.0.1:8443` and tunnelling; (2) add JWT or another auth scheme to all `/api` and `/ws` routes; (3) replace the CORS allowlist. This stays P0 while the colo compose publishes the port.

#### New body

## Problem

The admin REST API and its WebSockets have **no authentication**. Anyone who can reach the listener can stop the server (`POST /api/config/stop` → `orchestrator.stop_all()`), rewrite or delete content chains (`POST/DELETE /api/editor/content`), kick players, set entity properties, and stream server logs (`/ws/logs`). CORS is `allow_origin(Any)`, so a web page open in an operator's browser can also drive it (CSRF).

## Current state (main @ 059d6038)

- Done (PR #724): the binary defaults `ADMIN_BIND` to `127.0.0.1` (`crates/common/src/config.rs:86`, `crates/server/src/main.rs:193`).
- **Still exposed on the colo:** `docker/Dockerfile:269-274` sets `ADMIN_BIND=0.0.0.0`, because a published port can't reach an in-container loopback bind. `docker/compose.yml:64` publishes `"8443:8443"` on every interface. The only mitigation is a warning in `docs/operations/colo-deploy.md:39`.
- The port stays published because launcher telemetry (`/api/auth/dev-session`, `/api/telemetry/*`) shares this listener. Those routes have their own HMAC token plus a quota (#441 / PR #740). The admin routes have nothing.
- `crates/admin-api/src/middleware.rs:19` still has `// TODO: JWT authentication middleware`. `routes/auth.rs:44` login returns `"not implemented"`. `jsonwebtoken` is a declared but unused dependency.
- Partly mitigated: PR #698 (#440) stopped logging full SIDs, tickets and the Phase 1 SOAP body, so `/ws/logs` no longer streams live credentials. It still streams everything else.

## Plan

1. **Containment (P0, no client impact).** Split the launcher-telemetry routes onto their own listener or port, which keeps publicly reachable ingest working. Then change `docker/compose.yml` to publish admin `8443` as `127.0.0.1:8443:8443`, or drop the publish. Operators reach admin through an SSH or Cloudflare tunnel.
2. **Authentication.** Apply an auth middleware (`axum::middleware::from_fn_with_state`) to every `/api/*` route except the login and telemetry endpoints, and add a token check on the WebSocket upgrade (`/ws/logs` and any other `/ws/*`). Store operator credentials hashed with argon2id (the `argon2` crate is already a dependency via `crates/services/src/auth/credentials.rs`). Use short-lived signed tokens.
3. **CORS.** Replace `allow_origin(Any)` with an explicit allowlist for the ServerEd / Tauri origins.

## Acceptance

- The default colo compose does not expose admin routes on a public interface. A test or CI check pins the compose publish.
- Every admin `/api/*` route returns 401 without a valid token. A WebSocket upgrade without a token is rejected.
- A regression guard fails when the middleware is removed.
- Update `docs/tools/admin-api.md`, `docs/operations/colo-deploy.md`, `docs/operations/container.md`, and the env table in `crates/server/src/main.rs`.

Client impact: none (server and ops only).

---

## #447 — [Medium] Auth XML response builders use format! with no entity escaping — latent injection

- Verdict: KEEP
- Priority: P3
- Labels: no change
- Summary: Still true. `login_error`, `login_success_xml`, `select_error` and `server_location_xml` interpolate `msg`, the shard name, `session_key`, `ticket`, and the shard host/IP into attributes with `format!` and no escaping. Every interpolated value is still server-controlled (fixed error strings, config, RNG), so the issue remains latent, as filed. Open PR #604 (`fix/447-auth-xml-escaping`) implements the fix. It is MERGEABLE, and CI shows 10 successes and one pending. The line numbers in the issue moved by about 50.
- Evidence:
  - `crates/services/src/auth/handlers.rs:409` `fn login_error`, `:415` `ErrorStr=\"{msg}\"`; `:427` `fn login_success_xml`, `:432` `ServerName=\"{}\"`; `:450` `fn select_error`; `:467` `fn server_location_xml`, `:471` `SessionKey=\"{session_key}\" … IP=\"{ip}\"`
  - All `login_error`/`select_error` callers pass literals (`handlers.rs:62-294`)
  - Open PR #604 (mergeable, CI green)
- Related/duplicates: #460 (CAT-A)

### Action text

Status comment:
> Re-checked on main @ 059d6038. Still latent, as described: the builders are now at `crates/services/src/auth/handlers.rs:409/427/450/467` and every interpolated value is still server-controlled. PR #604 implements the escape, is mergeable, and CI is green. Merge PR #604 to close this.

---

## #448 — [Low] requestAmmoChange falls open on missing weapon definition

- Verdict: KEEP
- Priority: P3
- Labels: no change
- Summary: Still true. `requestAmmoChange` snapshots `item_defs.get(&item_id)`. When the def is missing it skips the whitelist and accepts any positive `ammo_type`. The code comment still cites the legacy `pass` semantics. The file was split, so the path in the issue is stale. PR #602 (`fix/448-ammo-change-fail-closed`) is open but CONFLICTING because the file split landed after it (PR #541 split bandolier).
- Evidence:
  - `crates/services/src/cell/cell_methods/inventory/bandolier/ammo_change.rs:77-84` (comment and `weapon_def` snapshot), `:124-140` (`if let Some(def) = weapon_def.as_ref()` means a missing def falls through to mutate)
  - `crates/services/src/cell/cell_methods/inventory/tests/ammo_change.rs:167` pins the fall-open behaviour as a test (the PR must invert it)
  - Open PR #602: CONFLICTING (last updated 2026-07-06)
- Related/duplicates: #463 (CAT-D), #445 (instance-id persistence, PR #520)

### Action text

Status comment:
> Re-checked on main @ 059d6038. Still falls open. The code is now at `crates/services/src/cell/cell_methods/inventory/bandolier/ammo_change.rs:77-84` and `:124-140` after the bandolier split in PR #541, and `tests/ammo_change.rs:167` currently pins the fall-open behaviour. PR #602 has the fix but conflicts with the split. It needs a rebase onto `bandolier/ammo_change.rs`, and the pinned test needs inverting.

---

## #443 — [Critical] AVATAR_UPDATE_EXPLICIT trusts client position — teleport + speed hack

- Verdict: CLOSE (completed)
- Priority: P3 (the residual speed enforcement belongs elsewhere)
- Labels: remove `ready-for-human`
- Summary: This is a duplicate of #478 (CAT-B P0-4), which PR #522 closed (2026-06-19). Every client 0x03 update now passes through `apply_client_position_update`. Hard rejects cover non-finite coordinates, space bounds, navmesh containment for non-GM players, and teleport distance, and a rejected update gets a `BASEMSG_FORCED_POSITION` snap-back. The only residual piece is the speed layer, which is **deliberately warn-only** until the tolerance is calibrated from SigNoz samples. That work belongs with #63 (movement validation) or #461 (CAT-B), not a Critical issue. The aurablacklight comment of 2026-09-19 reached the same conclusion.
- Evidence:
  - `crates/services/src/cell/space_manager/client_move.rs:124-126` (layer doc), `:222-245` (non-finite), `:260` (bounds), `:269-320` (navmesh), `:351-388` (speed warn-only, "calibrate before enforcing"; teleport hard-reject)
  - Tests: `crates/services/src/cell/space_manager/tests/movement_validation/{bounds,kinematics,navmesh,recovery,...}.rs`
  - PR #522 (closes #478), PR #437, PR #639/#643/#644/#645 (follow-ups), `docs/architecture/movement-validation.md`
- Related/duplicates: #478 (closed), #461 (CAT-B), #63 (speed and navmesh epic), #477 (updateId replay)

### Action text

Closing comment (reason: completed):
> Closing as a duplicate of #478, which PR #522 completed. On main, every client `AVATAR_UPDATE_EXPLICIT` goes through `SpaceManager::apply_client_position_update` (`crates/services/src/cell/space_manager/client_move.rs`). It hard-rejects non-finite coordinates, out-of-bounds positions, off-navmesh positions (non-GM) and teleport-distance jumps, and a rejected move gets a forced-position snap-back. Regression tests are under `space_manager/tests/movement_validation/`. The one residual piece is the speed layer, which is warn-only on purpose until the tolerance is calibrated (`client_move.rs:351-388`). That is tracked under #63 / #461. Position-packet replay (`updateId`) is tracked in #477.

---

## #434 — Encryption modernization: HTTPS for auth, argon2id passwords, hardened Mercury wire crypto

- Verdict: REWRITE
- Priority: P2
- Labels: add `ready-for-human` (the remaining work is client-patch engineering plus the transition-window decisions)
- Summary: The server halves of Phases 0–3 have landed. Phase 0 hook primitives were extracted (#504 / #566). Phase 1 has a tokio-rustls TLS listener on `auth_tls_port` 13443 with mtime cert hot-reload (#566 / #577). Phase 2 has argon2id storage with login-time migration, and the `password_hash_v2`/`password_algo` columns are in `db/sgw/Accounts/Tables/account.sql` and the seed. Phase 3 has Mercury v2 (version byte, random IV, HKDF-split keys, HMAC-SHA256), with v1 as the default, plus v2-gated session-key rotation. **Every client-side patch is still pending**, and no phase has been exercised against a live 2009 client (ADR status line). The colo does not enable TLS: no cert, and 13443 is not published in `docker/compose.yml`. The body is a pre-implementation plan and should become a remaining-work tracker. The open decisions (transition windows, hook crate) are partly settled: the hook layer lives in `client-telemetry`.
- Evidence:
  - `docs/architecture/encryption-modernization.md:3-5` ("server halves of Phases 0–3 implemented; every client-side patch is still pending … no phase has been exercised against a live 2009 client"), `:54-75` (Phase 0 complete), `:102-105`, `:136-137`, `:166-167`
  - `crates/services/src/auth/{tls.rs,cert_watcher.rs,credentials.rs,tls_smoke.rs}`; `crates/common/src/config.rs:27-48,143` (`auth_tls_port: 13443`)
  - `crates/mercury/src/encryption/mod.rs:36` (v2 frame), `:87` (version byte), `:157` (unknown version → v1 default), `crates/mercury/src/encryption/rotation.rs`
  - `db/sgw/Accounts/Tables/account.sql:12-20`
  - PRs #543 (RE and ADR), #566 (foundation), #577 (cert reload)
  - `docker/compose.yml:59-65` publishes 13001/8081 (plain HTTP) and not 13443
  - Memory (2026-06-20): encryption back-compat OK, v2 untested
- Related/duplicates: #476 (subset: Phase 1; recommend close into this), #477 item 1 (the zero IV is Phase 3), #294, #439 (admin auth is out of scope here)

### Action text

Comment:
> Status re-check (main @ 059d6038). The server halves of Phases 0–3 are merged (PRs #543/#566/#577): TLS auth listener with cert hot-reload, argon2id storage with login-time migration, and Mercury v2 crypto plus rotation (v1 remains the default). The ADR (`docs/architecture/encryption-modernization.md`) records that every client patch is still pending and nothing has run against a live client. I rewrote the body as a remaining-work tracker and folded in #476 (SOAP TLS, the Phase 1 server half, now done) and the zero-IV item from #477.

#### New body

## Goal

Modernize auth and transport crypto: HTTPS for SOAP login, argon2id password storage, and Mercury v2 packet crypto (random IV, HKDF-split keys, HMAC-SHA256, key rotation). Design and RE targets: `docs/architecture/encryption-modernization.md` (ADR) and `docs/reverse-engineering/findings/auth-and-crypto-modernization-targets.md`.

## Done (server halves)

- [x] Phase 0: hook primitives extracted into `client-telemetry` (PRs #504 / #566)
- [x] Phase 1 server: tokio-rustls listener on `auth_tls_port` (default 13443), cert/key config, mtime hot-reload (PRs #566, #577)
- [x] Phase 2 server: argon2id storage (`account.password_hash_v2`, `password_algo`), login-time migration from SHA-1, plaintext accepted only over TLS (`crates/services/src/auth/credentials.rs`)
- [x] Phase 3 server: Mercury v2 frame `[0x02][IV][ct][HMAC-SHA256/16]`, HKDF key split, v1/v2 dispatch with v1 as the default; v2-gated session-key rotation (`crates/mercury/src/encryption/`)

## Remaining

- [ ] Phase 1 client: `curl_easy_setopt @ 0x013A96E0` hook plus a loopback rustls proxy in the injected shim (cert pinning; dev build accepts a local CA)
- [ ] Phase 2 client: hook `LoginReplyHandler ctor @ 0x00DDED60` to send the plaintext from `ServerConnection+0x3c`; confirm that the second `logOnBegin` variant (ref near `0x019cf248`) has no separate SHA-1 path
- [ ] Phase 3 client: `Mercury::Channel::send @ 0x01576F90` vtable hook runs v2; client handling for rotation; per-client v1/v2 negotiation; production rotation scheduler
- [ ] Ops: enable TLS on the colo (cert provisioning, publish 13443) once a patched client exists
- [ ] Live-client validation of each phase (the ADR confidence line)
- [ ] Phase 4 cleanup: drop legacy `password`, the HTTP listener, and the v1 Mercury path once all clients are patched; update `docs/protocol/login-handshake.md` and `docs/protocol/mercury-wire-format.md`

## Owner decisions still open

1. Transition windows: parallel HTTP+HTTPS and v1+v2 for one release, or a hard cut.
2. Whether the patched-client distribution (launcher-injected shim) is in scope for the public colo, or local-only.

Client impact: requires client patches (DLL injection). Anti-debug verdict CLEAN.

---

## #476 — [security-audit P0-2] Move SOAP auth (Phase 1/2) to TLS — passwords + session keys currently plaintext (CAT-A-01)

- Verdict: CLOSE (completed)
- Priority: P2 (as tracked in #434)
- Labels: no change
- Summary: The server-side fix this issue asks for, rustls in front of the auth axum server, landed in PRs #566 and #577: a TLS listener on `auth_tls_port` with cert hot-reload. The TLS path cannot be used by the stock 2009 client, whose libcurl links OpenSSL 0.9.8g. It needs the Phase 1 client shim, which is already tracked as a checklist item in #434. The two issues duplicate each other, and the evidence line numbers (`service.rs:135`, `handlers.rs:172-181`) are stale. The colo still serves SOAP over plain HTTP (8081/13001), and that stays true until the #434 client work ships.
- Evidence: `crates/services/src/auth/tls.rs`, `cert_watcher.rs`, `tls_smoke.rs`; `crates/common/src/config.rs:27-48`; ADR `docs/architecture/encryption-modernization.md:77-98`; PRs #566, #577
- Related/duplicates: #434 (superset), #460 CAT-A-01

### Action text

Closing comment (reason: completed):
> The server half of this is done. PRs #566/#577 added a tokio-rustls auth listener (`auth_tls_port`, default 13443) with certificate hot-reload, and `credentials.rs` accepts plaintext only over TLS. The stock client can't use it, because its libcurl bundles OpenSSL 0.9.8g. It needs the Phase 1 client shim (curl hook plus loopback rustls proxy) and colo cert provisioning, both tracked as open checklist items in #434. Closing as consolidated into #434 so the TLS work has one tracker.

---

## #477 — [security-audit P0-3] Crypto + anti-replay foundation: per-packet IV, inbound dedup, position replay sequence (CAT-A-02 + CAT-A-03 + CAT-B-05)

- Verdict: REWRITE
- Priority: P2
- Labels: no change
- Summary: The three items have diverged. (1) The zero IV (CAT-A-02) is fixed on the server in Mercury v2 (random per-packet IV). v1 stays the default until the client patch in #434 ships, so this item belongs in #434. (2) The inbound dedup (CAT-A-03) is **still unwired**. `handle_encrypted_datagram` goes decrypt → parse → ACK-queue → dispatch and never calls `Channel::receive_packet` (now in `channel/channel_core.rs:361`). A captured reliable datagram replayed from the same source address is re-dispatched. (3) `updateId` (CAT-B-05) is still unread. Before anyone codes item 3, the "strictly increasing" premise needs checking. `updateId` is a u8 (it wraps every 256 packets), and its meaning is not documented anywhere under `docs/protocol`. The replay damage is also now bounded: the #478 movement validator hard-rejects teleport-distance jumps, so a replayed stale position can only rewind within the teleport threshold. Since #738 (#442) the replay also needs source-address spoofing, because sessions are bound to the issuing client IP and the connected map is keyed by `SocketAddr`.
- Evidence:
  - `crates/services/src/base/connect_loop/encrypted/mod.rs:48-140` (no `receive_packet` call; ACK queued for every reliable seq at `:103-108`); `:147-160` (0x01 authenticate skipped); `:214-217` (0x03 layout comment including `updateId:u8`, never read)
  - `crates/mercury/src/channel/channel_core.rs:361` `receive_packet` (RX window dedup; only callers are tests)
  - `crates/mercury/src/encryption/mod.rs:36-39,289-300` (v2 random IV), `:157` (v1 default)
  - `docs/architecture/movement-validation.md:418` lists the `updateId` anti-replay as an open follow-up
  - `docs/protocol/client-verified-wire-formats.md:333-337` (updateId exists; semantics not given)
- Related/duplicates: #434 (item 1), #294 (0x01 authenticate validation, the same anti-replay theme), #461/#460

### Action text

Comment:
> Re-checked on main @ 059d6038 and re-scoped. The zero-IV item (CAT-A-02) is fixed on the server by Mercury v2 in PR #566. v2 is off by default until the client patch lands, so that item now lives in #434. The other two items are server-only and still open. `handle_encrypted_datagram` never calls `Channel::receive_packet`, and the 0x03 `updateId` byte is never read. Before the `updateId` check is implemented, its semantics need confirming: it is a u8 that wraps, and it may be an ack or reference id rather than a monotonic counter. Rejecting on a wrong premise would drop legitimate movement. The #478 validator now caps replay-rewind damage at the teleport threshold.

#### New body

## Problem

Encrypted game datagrams have no replay protection on the server.

1. **Inbound reliable dedup is unwired (CAT-A-03).** `crates/services/src/base/connect_loop/encrypted/mod.rs::handle_encrypted_datagram` decrypts, parses, queues an ACK and dispatches every message. It never calls `Channel::receive_packet` (`crates/mercury/src/channel/channel_core.rs:361`), which implements the RX-window dedup. A captured reliable datagram, replayed from the session's source address, is dispatched again. v1 crypto has no IV or sequence number inside the unit, so the ciphertext replays verbatim.
2. **The position `updateId` is unvalidated (CAT-B-05).** `AVATAR_UPDATE_EXPLICIT` (0x03) carries `updateId:u8` at offset 39 (`encrypted/mod.rs:217`). It is never read. Replayed position packets are bounded by the #478 movement validator (the teleport hard-reject), but a rewind within the threshold is still possible.

Mitigations already in place: since #738 (#442) sessions are bound to the issuing client IP, so a replay needs on-path source spoofing. The zero-IV problem (CAT-A-02) is fixed by Mercury v2 and tracked in #434.

## Plan

1. Route reliable inbound packets through the per-session `Channel::receive_packet` (or an equivalent seen-seq window) before dispatch. Drop duplicates and out-of-window packets. Keep ACKing duplicates so the client stops retransmitting. Log a `mercury.replay_dropped` negative seam (see `docs/architecture/negative-logging-convention.md`).
2. **RE first:** establish what 0x03 `updateId` means (a monotonic counter, or an echo of a forced-position/reference id). Only then add a wrap-aware freshness check (for example, reject a packet whose `updateId` repeats inside a short window). Record the finding in `docs/protocol/client-verified-wire-formats.md`.

## Acceptance

- A replayed reliable datagram (same seq) is not dispatched twice. The regression guard fails when the dedup call is removed.
- A test at the Mercury-session level (TESTING.md type 9) covers retransmits: a legitimate retransmit after a lost ACK is ACKed but not re-dispatched.
- The `updateId` semantics are documented before any position-replay rejection ships.

Client impact: none (server only).

---

## #294 — [Security] Implement per-tick authenticate token validation (msg 0x01) — replay-attack vector

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: keep `ready-for-human`; consider removing `bug` (the premise is unverified)
- Summary: Still unimplemented. `handle_encrypted_datagram` skips the 0x01 body. The issue's premise, a token that *rotates* every tick, comes from the draft spec §2.5.2, which itself says the derivation is not pinned. The canonical `docs/protocol/message-dispatch-table.md:168` and the legacy C++ describe 0x01 as the static 20-byte session ticket, which the original server also ignored. Two premise reviews on 2026-09-19 already flagged the conflict. If the token is static, validating it adds almost no replay protection: a replayed packet carries the right ticket, and #738 already binds the session to the client IP. The anti-replay goal is better served by #477 (reliable-seq dedup). Note that 0x01 is skipped before `wire_log::log_inbound`, so the cheapest evidence route (comparing consecutive payloads) needs a one-line logging change or a pcap.
- Evidence:
  - `crates/services/src/base/connect_loop/encrypted/mod.rs:147-160` (skip; not passed to `crate::wire_log::log_inbound`, which runs at `:184`)
  - `docs/protocol/message-dispatch-table.md:168` ("Session ticket (20-byte ticket; server ignores per C++ line 131)")
  - `docs/drafts/spec/mercury-wire-format.md` §2.5.2 (claim; derivation "not pinned")
  - Memory: an idle client sends about 6 AUTHENTICATE packets per second (measured 2026-09-19)
- Related/duplicates: #477 (anti-replay), #434

### Action text

Owner question (post as a comment):
> This needs an owner decision. Is 0x01 `authenticate` a static session ticket (canonical dispatch table plus legacy C++) or a per-tick rotating token (draft spec §2.5.2, which admits the derivation is unknown)? Suggested evidence route: log the 0x01 payload (or a hash of it) through `wire_log` for one session and compare it against the issued ticket across ticks. It is currently skipped before `log_inbound` at `encrypted/mod.rs:147-160`. If it is static, I recommend closing this as not planned (a static check gives no replay protection beyond the #738 IP binding) and relying on #477's reliable-seq dedup. If it rotates, re-scope this issue to the RE-derived check.

---

## #532 — Harden minigame SFS server against DoS (no connection cap / read timeout) + ticket TTL/replay

- Verdict: REWRITE
- Priority: P2 (port 30000 is published on the colo)
- Labels: no change
- Summary: Partly done. Ticket TTL (item 3) landed in #652 (CA04): `PENDING_SESSION_TTL` is 180 s, a sweep task runs, and connected sessions are exempt. Item 4 is only half done. `authenticate_and_claim` sets `connected = true` atomically, but it does **not** reject a session that is already connected, so a second connection with the same ticket still authenticates while the first is live. The DoS items are unchanged: the accept loop spawns a task per connection with no cap, per-IP limit or rate limit, and the handshake/game reads have no timeout. The cross-domain policy is still `domain='*'`. The `Unknown SFS message type` warn still has no `peer` field and is still WARN level.
- Evidence:
  - `crates/services/src/minigame/server/mod.rs:56` (sweep), `:59-64` (`tokio::spawn(handle_connection(..))`, unbounded)
  - `crates/services/src/minigame/server/handshake.rs:30-32` (`allow-access-from domain='*'`), `:79-82` (`authenticate_and_claim`)
  - `crates/services/src/minigame/session.rs:36` (`PENDING_SESSION_TTL = 180s`), `:187-210` (`get_mut` → ticket and game-name check → `connected = true`, with no `if session.connected { return None }`), `:299-322` (expiry)
  - `crates/services/src/minigame/protocol.rs:193` (`warn!(msg_type, body_action, "Unknown SFS message type")`, no peer)
  - `docker/compose.yml:65` publishes `30000:30000`
  - PR #652
- Related/duplicates: #470 CAT-K-03 (done), CAT-K-07 (ticket binding, the same code). Recommend that #532 own all SFS transport and ticket hardening and that #470 point here.

### Action text

Comment:
> Re-checked on main @ 059d6038. The ticket TTL landed in #652 (180 s pending TTL plus sweep). Consume-on-auth is half done: `authenticate_and_claim` marks the session connected but never refuses one that is already connected (`session.rs:187-210`), so a concurrent second login with the same ticket still succeeds. The connection cap, read timeouts, the `domain='*'` policy and the noisy peer-less warn are unchanged. I rewrote the checklist and folded in CAT-K-07 (ticket binding) from #470, so all SFS transport and ticket hardening is tracked here.

#### New body

## Context

The minigame SmartFoxServer-compatible TCP server (`crates/services/src/minigame/`) is published on the colo (`docker/compose.yml`, `30000/tcp`) and receives internet probes. It is a safe-Rust reimplementation: there is no JVM deserialization, `quick-xml` is non-validating (no XXE or billion-laughs), messages are bounded at `MAX_MESSAGE_LEN`, and a panic is isolated to its per-connection task. Game logic is gated behind a 256-bit CSPRNG ticket. What remains is resource exhaustion and ticket-reuse hardening.

## Done

- [x] Pending-session TTL: `PENDING_SESSION_TTL = 180s` plus a periodic sweep; connected sessions are exempt (`session.rs:36`, `:299-322`; #652)
- [x] Atomic validate-and-claim (`authenticate_and_claim`, `session.rs:187`)

## Remaining

- [ ] **Read/idle timeout** on the handshake (`verChk` → login) and on the game read loop (`server/handshake.rs`, `server/mod.rs`). Drop peers that don't complete the handshake within N seconds or that go idle.
- [ ] **Connection cap**: a `tokio::Semaphore` around the `tokio::spawn(handle_connection(..))` at `server/mod.rs:59-64`, plus an optional per-IP concurrent limit and accept-rate limit.
- [ ] **Reject re-auth of a connected session**: `authenticate_and_claim` should return `None` when `session.connected` is already true, so a ticket authenticates at most one live connection (CAT-K-07). Optionally bind the ticket to the player's session IP.
- [ ] Tighten the Flash cross-domain policy (`server/handshake.rs:32`, `domain='*'`) if the client's policy fetch allows it.
- [ ] Noise: add `peer` to the `Unknown SFS message type` warn (`protocol.rs:193`) and downgrade it to DEBUG or rate-limit it, so probes don't flood the Discord errors channel.

## Acceptance

- Tests: a connection that sends nothing is closed after the timeout; the N+1th concurrent connection is refused; a second login with an already-claimed ticket gets `loginFailed`. Each guard fails when its fix is reverted.

Client impact: none.

---

## #469 — [security-audit] CAT-J — Mission / Dialog / Interaction (9 findings)

- Verdict: REWRITE
- Priority: P2
- Labels: no change
- Summary: Three of the nine findings are closed, including the Critical one. CAT-J-01 was fixed by PR #513 (#479) and hardened by PR #770 (offered-dialog set, take-on-choice, one-shot). CAT-J-03 was fixed by PR #741 (dead or missing actor rejected before interact dispatch). CAT-J-08 is closed because the Mission*/Debug* GM names are SGWGmPlayer methods at index 109 or above, which `requires_gm` gates, and several of them are now implemented as GM handlers. Still open: CAT-J-02 (`initialResponse` searches **every** NPC's `available_interactions` for the set-map id instead of only the pinned `last_interaction_target`'s entry), CAT-J-04 (content `accept_or_advance` checks the offer guard but not level/faction prereqs; lower risk now that dialog choices are gated), CAT-J-05 (`chosenRewards` is still an UNIMPLEMENTED stub), CAT-J-06 (`shareMission`/`shareMissionResponse` stubs), CAT-J-07 (`abandon_mission` removes the mission in memory and sends the client update but never persists a delete or status, so the mission can come back on relog), and CAT-J-09 (not re-verified; unchanged).
- Evidence:
  - `crates/services/src/cell/cell_methods/player/interaction/dialog.rs:26-57` (offered-dialog gate), PRs #513 and #770
  - `crates/services/src/cell/cell_methods/player/interaction/interact.rs:23-25` (dead/missing actor gate), PR #741
  - `crates/services/src/cell/dispatch/gm_gate.rs:128-139` (range rule ≥109); `crates/services/src/cell/cell_methods/gm/mod.rs:61-75,221-233` (GM mission handlers)
  - `crates/services/src/cell/interactions/dispatch/initial_response.rs:38-47` (search over all `available_interactions.values()`), `:88-102` (the NPC comes from the pin only for the portrait)
  - `crates/services/src/cell/content/executor/mission.rs:15-60` (offer guard only)
  - `crates/services/src/cell/cell_methods/player/world/mod.rs:136-137` (`UNIMPLEMENTED: chosenRewards`)
  - `crates/services/src/cell/cell_methods/missionary.rs:43-56` (share stubs)
  - `crates/services/src/cell/missions/lifecycle.rs:186-222` (abandon: no persist message)
- Related/duplicates: #479 (closed), #459 umbrella

### Action text

Comment:
> Status re-check on main @ 059d6038. Closed: CAT-J-01 (PRs #513/#770, offered-dialog gate), CAT-J-03 (PR #741) and CAT-J-08 (GM mission names sit on the GM-gated SGWGmPlayer tail at index ≥109, via #475/#518). The remaining open findings are listed in the rewritten body. The highest-value remaining item is CAT-J-07: an abandoned mission is never persisted, so it reappears after relog.

#### New body

Part of the server-authority audit (umbrella #459). Findings file: `docs/security-audit/2026-05-31-server-authority/findings/CAT-J-mission-dialog.md` (on `main`).

## Status (re-verified on main @ 059d6038)

- [x] **Critical** CAT-J-01: DialogButtonChoice with no open-dialog check. Fixed by PR #513 (#479) and PR #770 (offered-dialog set, one-shot take).
- [ ] **Medium** CAT-J-02: `initialResponse` matches the set-map id against *every* NPC's `available_interactions` (`cell/interactions/dispatch/initial_response.rs:38-47`). Scope the lookup to the entry for `last_interaction_target`, and re-check the interact distance.
- [x] **Low** CAT-J-03: interact from a dead or missing actor. Fixed by PR #741.
- [ ] **High→Medium** CAT-J-04: content `accept_or_advance` (`cell/content/executor/mission.rs:15`) enforces only the offer guard (already active, repeat cap). There are no level, faction or prior-mission prereqs. Risk is lower now that dialog choices are gated, but chains authored without conditions still grant any mission.
- [ ] **High** CAT-J-05: `chosenRewards` (CM 87) is an UNIMPLEMENTED stub (`cell/cell_methods/player/world/mod.rs:136`). When it is implemented, validate the choice against the completed mission's reward options, one time only.
- [ ] **Medium** CAT-J-06: `shareMission`/`shareMissionResponse` stubs (`cell/cell_methods/missionary.rs:43-56`). They need a pending-share correlation when implemented.
- [ ] **Low** CAT-J-07: `abandon_mission` (`cell/missions/lifecycle.rs:186`) never persists the removal, so the mission comes back on relog. It also has no check that the mission is abandonable.
- [x] **Medium** CAT-J-08: Mission*/Debug* GM names. They are SGWGmPlayer methods at index ≥109, gated by `requires_gm` (`cell/dispatch/gm_gate.rs:128-139`, #475/#518).
- [ ] **Low** CAT-J-09: chain context player_id vs entity linkage (not re-verified; unchanged).

Client impact: none (all server-side).

---

## #470 — [security-audit] CAT-K — Minigame (9 findings)

- Verdict: REWRITE
- Priority: P2
- Labels: add `ready-for-human` (CAT-K-01 needs an owner decision)
- Summary: CAT-K-02 is closed by PR #609: the debug quartet at CM 20–23 was added to `requires_gm`. This is newer than the findings file's 2026-07-25 re-verification block, which still calls it open. CAT-K-03 is closed by #652 (180 s pending TTL plus sweep). CAT-K-07 is still open: there is no IP binding, and a claimed ticket can authenticate a second concurrent connection. CAT-K-08 is open, and it compounds with K-07 because two sessions could each report victory. CAT-K-01 is unchanged: `PlaceholderGame` still grants an instant victory on `cmd=victory` for Hack, Activate, Analyze, Bypass, Converse and ConverseBasicHumanoid. Those SWFs are shells with nothing the server could validate, so this is a design and scope question, not a bug fix. CAT-K-04/05/06/09 are still UNIMPLEMENTED stubs (latent).
- Evidence:
  - `crates/services/src/cell/dispatch/gm_gate.rs:128-139` (`CM_MG_DEBUG_START..INSTANCE` gated), PR #609
  - `crates/services/src/minigame/session.rs:36,187-210,299-322`, PR #652
  - `crates/services/src/minigame/games/placeholder.rs:39-49` (victory → `GameOutput::Victory`)
  - `crates/services/src/cell/cell_methods/minigame.rs:63-150` (START/END_CURRENT/SPECTATE/REGISTER_HELP/START_CANCEL/CALL_* all UNIMPLEMENTED)
  - The findings file status block (`CAT-K-minigame.md:3-35`) is stale on K-02 and K-03
- Related/duplicates: #532 (transport and ticket hardening; recommend moving K-07 there), #475 (closed)

### Action text

Comment:
> Status re-check on main @ 059d6038. CAT-K-02 is closed by PR #609 (the debug quartet at CM 20–23 is now in `requires_gm`). CAT-K-03 is closed by #652 (180 s pending-session TTL). The 2026-07-25 status block in the findings file predates both and should be updated. CAT-K-07 (ticket reuse and binding) moves to #532, which already owns SFS hardening. Owner question for CAT-K-01: the placeholder minigames (Hack, Bypass, Analyze, Activate, Converse) are SWF shells whose only client message is `victory`, so the server has nothing to validate. Do we accept "possession of a valid ticket = win" as the design for these game types, or block the missions that depend on them until real game logic exists?

#### New body

Part of the server-authority audit (umbrella #459). Findings file: `docs/security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md` (on `main`).

## Status (re-verified on main @ 059d6038)

- [ ] **Critical** CAT-K-01: `PlaceholderGame` (`crates/services/src/minigame/games/placeholder.rs`) awards an instant victory on client `cmd=victory` for Hack/Activate/Analyze/Bypass/Converse/ConverseBasicHumanoid. **Owner decision needed**: accept this as the design for shell SWFs (the ticket is the only gate), or add a server-side minimum (for example, a minimum session duration) or content gating.
- [x] **High** CAT-K-02: `debug*Minigame` at CM 20–23 reachable by players. Fixed by PR #609 (`cell/dispatch/gm_gate.rs`).
- [x] **Medium** CAT-K-03: no session TTL. Fixed by #652 (`PENDING_SESSION_TTL` = 180 s plus sweep).
- [ ] **Medium** CAT-K-04: `minigameStartCancel` / `endCurrentMinigame` are stubs. When implemented, act only on the caller's own session.
- [ ] **Medium** CAT-K-05: `spectateMinigame(playerId)` stub. Needs a perception or permission check when implemented.
- [ ] **Medium** CAT-K-06: `minigameCallRequest` stub. Needs a contact and rate gate when implemented.
- [ ] **Low** CAT-K-07: ticket reuse and binding. **Tracked in #532.**
- [ ] **Low** CAT-K-08: `MinigameResult` handler has no idempotency or mission-context check. This compounds with K-07 (two connections, two victories).
- [ ] **Low** CAT-K-09: `registerToMinigameHelp` stubs. Needs a skill check when implemented.

Client impact: none.

---

## #471 — [security-audit] CAT-L — Chat / Contact list / Communication (9 findings)

- Verdict: REWRITE
- Priority: P2 (CAT-L-01 is live on the colo)
- Labels: no change
- Summary: CAT-L-02 is closed by PR #739 (DND stored bounded to 128 characters). CAT-L-04 is mostly obsolete: the contact list is now implemented in `crates/services/src/base/contact_list/` (#579, owner-confirmed working) with DB ownership checks on every op and a 100-member per-request cap. There is still no self-add guard and no cap on total lists or members. CAT-L-01 is **still live**: `sendPlayerCommunication` forwards any-length text to the cell for AoI broadcast, with no rate limit and no ignore filter. Open PR #585 (ignore enforcement) addresses part of it; its mergeable state is UNKNOWN and it was last updated in June. CAT-L-03 is not re-verified: the channel byte is still forwarded unvalidated. CAT-L-05 to L-09 are still unimplemented stubs or fall-throughs (MinimapPing, GMShout, Petition/Who, channel admin, chatJoin/Leave auto-ack). They are latent.
- Evidence:
  - `crates/services/src/base/dispatch/chat.rs:20` (`MAX_DND_MESSAGE_CHARS = 128`, PR #739), `:26-111` (send path: no length cap, no rate limit, no ignore check), `:113-133` (chatJoin/Leave acks)
  - `crates/services/src/base/contact_list/handlers/member_ops.rs:47-56,104-109`, `header_ops.rs:129,216,306` (ownership); `wire.rs:88` (`MAX_MEMBERS_PER_REQUEST = 100`)
  - Open PR #585 (`feat/ignore-enforcement`)
- Related/duplicates: #572 (contact list), #65 (speaker flags, PR #425)

### Action text

Comment:
> Status re-check on main @ 059d6038. CAT-L-02 is fixed by PR #739. CAT-L-04 is largely superseded by the implemented contact list (ownership checks and a per-request cap in `base/contact_list/`). Only a self-add guard and total-size caps remain. CAT-L-01 is still live (no rate limit, length cap or ignore filter on `sendPlayerCommunication`), and PR #585 covers the ignore part. I rewrote the body with the current checklist.

#### New body

Part of the server-authority audit (umbrella #459). Findings file: `docs/security-audit/2026-05-31-server-authority/findings/CAT-L-chat-contact.md` (on `main`).

## Status (re-verified on main @ 059d6038)

- [ ] **Medium (live)** CAT-L-01: `sendPlayerCommunication` (`crates/services/src/base/dispatch/chat.rs:26-111`) broadcasts to the AoI with no per-sender rate limit, no text length cap and no ignore filter. Ignore enforcement is in open PR #585.
- [x] **Low** CAT-L-02: DND message length. Fixed by PR #739 (128 characters).
- [ ] **Low** CAT-L-03: the `channel` byte is forwarded without an allowlist.
- [ ] **High→Low** CAT-L-04: contact list is implemented with DB ownership checks and `MAX_MEMBERS_PER_REQUEST = 100`. Remaining: a self-add guard and caps on total lists and members per player.
- [ ] **High (latent)** CAT-L-05: `BroadcastMinimapPing` has no handler. When implemented, validate org membership and server-side position.
- [ ] **High (latent)** CAT-L-06: `SendGMShout` has no handler. When implemented, it must be GM-gated using the server-side `access_level`, which is now available in the cell via #475.
- [ ] **Medium (latent)** CAT-L-07: `Petition`/`Who`/`chatFriend`/`chatIgnore` have no handlers.
- [ ] **Medium (latent)** CAT-L-08: channel admin commands (`chatOp`/`Mute`/`Kick`/`Ban`/`Password`) have no handlers. Op-bit tracking is needed.
- [ ] **Low (latent)** CAT-L-09: `chatJoin`/`chatLeave` are auto-acknowledged, so there is no password gate.

Client impact: none.

---

## #472 — [security-audit] CAT-M — Organization / Squad / Duel (18 findings)

- Verdict: CLOSE (not planned), with its checklist folded into #568 and #569
- Priority: P3
- Labels: n/a
- Summary: Still entirely pre-implementation on main. `cell/cell_methods/organization.rs` has 12 UNIMPLEMENTED arms and `player/social.rs` has 9. There are no base arms for organizationInvite, Kick, RankChange or sendDuelChallenge. None of the 18 findings is exploitable today, because every handler is a no-op. They are design requirements for the org/squad system (#568, open PR #584, CONFLICTING since June) and the duel system (#569). Keeping them in a separate security issue splits the acceptance criteria away from the implementation tickets. Recommend adding the findings-file link as acceptance criteria on #568 and #569, then closing.
- Evidence:
  - `grep -c UNIMPLEMENTED`: `crates/services/src/cell/cell_methods/organization.rs` = 12, `crates/services/src/cell/cell_methods/player/social.rs` = 9
  - No organization/duel arms under `crates/services/src/base/dispatch/`
  - Open PR #584 `feat/org-system-568` (CONFLICTING, 2026-06-21)
- Related/duplicates: #568 (org/squad), #569 (duel)

### Action text

Closing comment (reason: not planned):
> Every CAT-M surface is still a no-op stub on main @ 059d6038 (12 UNIMPLEMENTED arms in `organization.rs` and 9 in `player/social.rs`), so nothing here is exploitable today. The 18 findings are acceptance criteria for work that hasn't been built yet. I'm moving them onto the implementation tickets so they get checked when the handlers are written: CAT-M-01 to M-11, M-16, M-17 and M-18 go to #568 (organization, squad, strike team), and CAT-M-12 to M-15 go to #569 (duel). Findings file: `docs/security-audit/2026-05-31-server-authority/findings/CAT-M-org-squad-duel.md`.

Also post on #568:
> Security acceptance criteria from the server-authority audit (closed #472): implement against CAT-M-01 to M-11, M-16, M-17 and M-18 in `docs/security-audit/2026-05-31-server-authority/findings/CAT-M-org-squad-duel.md`. Key invariants: membership and rank checks on invite, kick, rank change and permission changes (no promoting above your own rank); a rank-bounded guild-bank withdraw cap (M-09, Critical); caps on text length (MOTD, notes, rank names); pending-invite and pending-request correlation for every `*Response` method; squad loot mode restricted to the leader, with an enum range check.

Also post on #569:
> Security acceptance criteria from the server-authority audit (closed #472): CAT-M-12 to M-15 in `docs/security-audit/2026-05-31-server-authority/findings/CAT-M-org-squad-duel.md`. Challenge target must be online, in the same space and within range, with a cooldown. `sendDuelResponse` needs a pending-challenge correlation. `duelForfeit` is valid only for a current participant. A disconnect during a duel auto-forfeits.

---

## #473 — [security-audit] CAT-N — GM / Debug / Cheat commands (40 findings)

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: The systemic findings are closed. CAT-N-03 (`access_level` in cell dispatch) is fixed by PR #512 (#475): `enforce_gm_gate` reads the server-side `CellEntity::access_level` and fails closed. CAT-N-04 (GM class) is fixed by PR #518: GMs enter as SGWGmPlayer 0x03. CAT-N-01 is fixed by #475 (CM 92 gated). CAT-N-30 is fixed by #475 (CM 2/3/6 gated). Every `gm*` / `Event_NetOut_*` debug method from CAT-N-05 to N-24, N-27, N-28 and N-31 to N-38 is on the SGWGmPlayer tail at index ≥109. The range rule gates that whole tail, including indices with no handler. About 40 handlers are now implemented natively (`cell/cell_methods/gm/`), with bounds (for example, `gmGiveItem` quantity clamped to [1, 1000], non-finite `gotoXYZ` rejected). The `.`-console (#523) gates on `access_level >= GameMaster`. That leaves a short residual list. CAT-N-02 (`resetMyAbilities`, CM 72) was deliberately **not** gated: `gm_gate.rs` documents that it is a player-facing trainer respec, and it needs cost, cooldown and trainer-proximity checks when the stub is implemented. CAT-N-26: player `setMovementType` still accepts the full 0–6 AI enum. CAT-N-39: `setAutoCycle` inherits `setTargetID`'s missing perception check (CAT-C territory). CAT-N-25 (perfStats) is now a DEBUG-only log sink, and CAT-N-29 is benign because the entity id comes from the session.
- Evidence:
  - `crates/services/src/cell/dispatch/gm_gate.rs:1-19,53-62,98-139` (the gate, the ≥109 range rule, and the CM 72 rationale), PRs #512, #518, #609
  - `crates/services/src/cell/cell_methods/gm/mod.rs:61-190,207-268` (implemented GM handlers); `gm/give.rs:20,136-143`; `gm/travel.rs:43,145`
  - `crates/services/src/cell/console/mod.rs:21-24,104-105`, `console/dispatch.rs:22` (the `.`-console GM gate, #523)
  - `crates/services/src/cell/cell_methods/player/combat/mod.rs:172` (`UNIMPLEMENTED: resetMyAbilities`)
  - `crates/services/src/base/dispatch/diagnostics.rs:40-55` (perfStats sink)
  - The findings file status block (`CAT-N-gm-commands.md:3-30`) predates #518 and #609 and still lists CAT-N-02 as a gating item
- Related/duplicates: #475 (closed), #518/#523 (GM consoles), #462 (CAT-C setTargetID perception)

### Action text

Comment:
> Status re-check on main @ 059d6038. The core of CAT-N is resolved. #475 / PR #512 added the server-side `access_level` gate, which fails closed. PR #518 made GMs enter as SGWGmPlayer. `requires_gm` gates CM 2/3/6/20–23/92 and the whole SGWGmPlayer tail at index ≥109, which covers every `gm*` wire surface in findings N-05 to N-38, including methods with no handler yet. The native GM handlers in `cell/cell_methods/gm/` apply their own bounds. The `.`-console is GM-gated (#523). I rewrote the body to the short residual list. Note that CAT-N-02 is intentionally not GM-gated (see the `gm_gate.rs` docs: it's the player-facing trainer respec), so it now reads as "harden the handler when implemented".

#### New body

Part of the server-authority audit (umbrella #459). Findings file: `docs/security-audit/2026-05-31-server-authority/findings/CAT-N-gm-commands.md` (on `main`).

## Resolved

- CAT-N-03: `access_level` in cell dispatch. Fixed by PR #512 (#475): `crates/services/src/cell/dispatch/gm_gate.rs` reads the server-side `CellEntity::access_level` and fails closed.
- CAT-N-04: GMs forced to class 0x02. Fixed by PR #518 (SGWGmPlayer 0x03).
- CAT-N-01 (CM 92) and CAT-N-30 (CM 2/3/6): named in `requires_gm`.
- CAT-N-05 to N-24, N-27, N-28, N-31 to N-38, N-40: all are SGWGmPlayer methods at flattened index ≥109. They are gated by the range rule (`SGWGMPLAYER_CELL_METHOD_BASE`), including indices with no handler. The implemented native GM handlers (`cell/cell_methods/gm/`) apply per-command bounds (for example, `gmGiveItem` quantity [1, 1000] and non-finite `gotoXYZ` rejected). The `.`-console (#523) requires `access_level >= GameMaster`.
- CAT-N-25: `perfStats` is a DEBUG-only log sink (`base/dispatch/diagnostics.rs`).
- CAT-N-29: `requestReload` uses the session-substituted entity id; benign.

## Remaining

- [ ] CAT-N-02: `resetMyAbilities` (CM 72) is a player-facing trainer respec and **intentionally not** GM-gated (`gm_gate.rs` docs). When the stub (`cell/cell_methods/player/combat/mod.rs:172`) is implemented: require an active trainer within `MAX_INTERACT_DISTANCE`, charge `sgw_player.cash`, and enforce a per-character cooldown.
- [ ] CAT-N-26: player `setMovementType` (CM 1) accepts the full `EMobMovementType` 0–6. Restrict it to the values a player can legitimately send.
- [ ] CAT-N-39: `setAutoCycle` (CM 83) fires at `current_target_id`, which `setTargetID` sets without a perception check. Fix in CAT-C (#462).
- [ ] Update the status block in the findings file (it predates #518 and #609).

Client impact: none.

---

## #474 — [security-audit] CAT-O — World / Space / Gate / Ring (6 findings)

- Verdict: REWRITE
- Priority: P3
- Labels: remove `ready-for-human` after the rewrite (the reconciliation it asked for is done)
- Summary: CAT-O-01 is resolved: `player_knows_stargate` is checked before arming the dial (confirmed by the premise review on 2026-09-19). CAT-O-02 is resolved by PR #663 (the 4-second dial timer, with cancellations, is in `gate_travel/tick.rs`). CAT-O-04 is resolved by #475 (CM 92 in `requires_gm`). CAT-O-03 is largely resolved by #682: `triggerClientHintedGenericRegion` ignores the client xyz, the region must be registered, and entry is checked against the **server-known position**. The residual is that exit events skip the containment test, and there is no rate limit on enter/exit toggling. CAT-O-05 (`cancelMovie`) and CAT-O-06 (`updateSystemOptions`) are Low and not re-verified; unchanged.
- Evidence:
  - `crates/services/src/cell/gate_travel/mod.rs:49,129`, `address_book.rs:70` (address check); `docs/gameplay/gate-travel.md:84`
  - `crates/services/src/cell/gate_travel/tick.rs:19`, `tests/dial_timer.rs` (PR #663)
  - `crates/services/src/cell/dispatch/gm_gate.rs:128-139` (CM 92)
  - `crates/services/src/cell/cell_methods/player/world/mod.rs:56-68` (client xyz deliberately unread), `:180-290` (`resolve_hinted_region`: registration check plus server-known containment "on entry only")
- Related/duplicates: #461 (CAT-B-02/03/04 position side), #475

### Action text

Comment:
> Reconciliation done against main @ 059d6038. Closed: CAT-O-01 (address-book check before dialing), CAT-O-02 (4-second dial timer, PR #663), CAT-O-04 (CM 92 GM-gated, #475). CAT-O-03 is mostly closed by #682: region hints are resolved against the server-known position and the registered-region set, but only on entry. The body now tracks the small remainder.

#### New body

Part of the server-authority audit (umbrella #459). Findings file: `docs/security-audit/2026-05-31-server-authority/findings/CAT-O-world-space-gate.md` (on `main`).

## Status (re-verified on main @ 059d6038)

- [x] **High** CAT-O-01: `onDialGate` ignores known addresses. `player_knows_stargate` runs before arming (`cell/gate_travel/address_book.rs:70`).
- [x] **Medium** CAT-O-02: no dial timer. The 4-second timer with cancellations landed in PR #663 (`cell/gate_travel/tick.rs`).
- [ ] **High→Low** CAT-O-03: `triggerClientHintedGenericRegion` requires a registered region plus server-known containment **on entry** (#682; `cell/cell_methods/player/world/mod.rs` `resolve_hinted_region`). Remaining: exit events skip the containment check, and there is no per-region rate limit on enter/exit toggling (chain spam).
- [x] **High** CAT-O-04: `onWorldInstanceReset` on the player. GM-gated (`cell/dispatch/gm_gate.rs`, #475).
- [ ] **Low** CAT-O-05: `cancelMovie` sets `cinematic_spam_cancel` with no movie-id binding.
- [ ] **Low** CAT-O-06: `updateSystemOptions` rides a name/value WSTRING wire; keep the allowlist when adding options.

Client impact: none.

---

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 439 | REWRITE | P0 | PR #724 made the binary bind loopback, but the colo image binds 0.0.0.0 and compose publishes 8443 on all interfaces. No JWT, CORS `Any`. |
| 447 | KEEP | P3 | Still latent. PR #604 fixes it (mergeable, CI green): merge it. |
| 448 | KEEP | P3 | Still falls open (now in `bandolier/ammo_change.rs`). PR #602 conflicts after the file split and needs a rebase. |
| 443 | CLOSE (completed) | P3 | Duplicate of #478, done in PR #522. The residual warn-only speed layer belongs to #63/#461. |
| 434 | REWRITE | P2 | Server halves of Phases 0–3 merged (#566/#577). All client patches, colo TLS enablement and live validation remain. |
| 476 | CLOSE (completed) | P2 | Server TLS done (#566/#577). Client shim is tracked in #434. Duplicate. |
| 477 | REWRITE | P2 | Zero IV fixed in v2 (moves to #434). Inbound dedup still unwired. `updateId` semantics need RE first. |
| 294 | NEEDS-OWNER | P3 | Static-ticket vs rotating-token premise is unresolved. If static, close in favour of #477. |
| 532 | REWRITE | P2 | TTL done (#652). Connection cap, read timeout, reject of an already-connected ticket, `domain='*'` and the noisy warn remain. |
| 469 | REWRITE | P2 | J-01/J-03/J-08 done (#513/#770/#741/#475). J-02, J-04, J-05, J-06, J-07 (abandon not persisted) and J-09 open. |
| 470 | REWRITE | P2 | K-02 done (#609) and K-03 done (#652). K-01 placeholder instant-win needs an owner decision. K-07 moves to #532. |
| 471 | REWRITE | P2 | L-02 done (#739). L-04 mostly superseded by the implemented contact list. L-01 chat rate/length/ignore still live (PR #585). |
| 472 | CLOSE (not planned) | P3 | All 18 are pre-implementation stubs. Fold them into #568 (org/squad) and #569 (duel) as acceptance criteria. |
| 473 | REWRITE | P3 | The GM gate (#475), SGWGmPlayer (#518) and the ≥109 range rule close almost all 40. Residual: N-02 (respec hardening), N-26, N-39. |
| 474 | REWRITE | P3 | O-01, O-02 and O-04 done. O-03 mostly done (entry containment). O-03 exit-edge/rate, O-05 and O-06 remain. |

### Cross-batch observations

- The audit findings files are now on `main` (`docs/security-audit/2026-05-31-server-authority/`), but every category issue (#460–#474) still links the old `worktree-server-authority-audit` branch. The K and N "status re-verification (2026-07-25)" blocks are stale: they predate #609 (K-02 fixed) and #652 (K-03 fixed). The umbrella #459 (another batch) should be updated with the per-category status above.
- Consolidation: #476 → #434; #477 item 1 → #434; #443 → #478 (already closed); #470 CAT-K-07 → #532; #472 → #568/#569; #294 → #477 if the ticket turns out to be static. The residual speed-cap enforcement belongs with #63 / #461 (CAT-B).
- Surprise: #439 is still effectively P0 on the colo despite PR #724. The fix needs launcher telemetry split onto its own listener, because the shared listener is the reason 8443 is still published.
- Two open PRs are ready or near-ready for batch issues: #604 (#447, mergeable and green) and #602 (#448, needs a rebase). PR #585 (ignore enforcement, CAT-L-01) and PR #584 (org system, CAT-M) have been stale since June.
