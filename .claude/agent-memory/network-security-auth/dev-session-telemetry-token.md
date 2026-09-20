---
name: dev-session-telemetry-token
description: Non-obvious facts about the /api/auth/dev-session HMAC token — where the real harm is, why a mint quota does not fix it, the launcher's missing re-mint path, and the refresh-storm math that any exp-clamp triggers
metadata:
  type: project
---

Verified 2026-09-19 against the `fix/441-dev-session-mint-quota` worktree (base `beaf7947`).
Mint/refresh live in `crates/admin-api/src/routes/dev_session/` (promoted from `dev_session.rs`
during issue #441); ingest in `crates/admin-api/src/routes/telemetry/`.

**The harm is on ingest, not mint.** Mint being unauthenticated is the headline of issue #441,
but `verify_bearer` (`telemetry/handlers.rs`) checks signature + `exp` and nothing else — no
rate limit, no `scope` check (the `scope` claim is signed but never enforced). `telemetry/mod.rs`
applies only body-size caps. **One token buys 8h of unlimited log injection**, so a mint quota
bounds distinct `(sid, sub)` label cardinality and stops a mint loop — it does *not* bound
injected volume. Don't let a PR claim it "fixes SigNoz poisoning."

**Escalation ceiling is lower than it looks.** Replayed events emit at `info!`/`debug!` only, and
the Discord layer harvests WARN/ERROR only (`crates/discord/src/layer.rs` `on_event`), so there is
no injection path to Discord. There *is* one to `/ws/logs`, which is itself unauthenticated — this
compounds finding #2 of [[security-audit-2026-05-31]] rather than standing alone. Also: mint hard-
fails 500 when `CIMMERIA_TELEMETRY_HMAC_SECRET` is unset, so telemetry-less deployments aren't exposed.

**No reverse proxy fronts the admin port.** `docker/compose.yml` publishes `8443:8443` directly.
The Cloudflare Tunnel in `docs/operations/signoz-remote-access.md` is scoped to `frontend:3301`
(the SigNoz UI). So **never read `X-Forwarded-For` on admin routes** — no trusted proxy means the
header is attacker-controlled. Workspace precedent is peer-address-only: `services/src/auth/
handlers.rs` extracts `ConnectInfo<SocketAddr>` and uses `addr.ip()`, wired by
`into_make_service_with_connect_info::<SocketAddr>()` in `services/src/auth/service.rs` (including
the TLS listener, via a `tap_io` no-op that makes the blanket `Connected` impl apply).

**Unmeasured: does the container see real client IPs?** Published ports traverse `docker-proxy`;
with `userland-proxy=true` the container may observe the bridge gateway (`172.x.0.1`) for all
callers, which collapses any per-IP limiter into a global cap and locks out every launcher at once.
This cannot be settled by reading code — it needs a connection from off-box with the handler
logging the peer. Until measured, per-IP quotas should default to disabled/generous, and every
quota should be env-configurable with `0` = disabled so an operator has a runtime escape hatch.

**The launcher never re-mints.** `telemetry::start_session` runs once per process.
`ChunkError::TokenRejected` / `BundleError::TokenRejected` are defined but matched *nowhere* in
`launcher/src/telemetry/runner.rs` — `tick_once` swallows refresh failures with a `warn!`. Once a
token expires the session is dead for the rest of the process, events pile into the 100 MiB disk
queue until compaction drops them, and `refresh_if_due` 401-storms every flush tick (2s). This is a
pre-existing bug worth its own issue; it also means **any server-side session cap strands real users.**

**Clamping `exp` on refresh: the storm depends on the cap:TTL ratio, and at 3:1 it is a non-issue.**
`should_refresh` (`launcher/src/telemetry/auth.rs`) fires at 25% remaining, and after each refresh
the launcher re-stamps a *local* `issued_at_ms = now`. So `total = exp - now` and each successive
window is ¼ of the previous one *once the clamp binds*. The trap is assuming it binds immediately:
with TTL 8h and cap 24h it does not bind until 16h of session age, and the series then converges in
~8 refreshes over the final 8 hours — negligible. It only turns pathological as cap approaches TTL.
I filed this as "cut the clamp" on the early-binding assumption and had to walk it back; **run the
ratio before calling it.** Note `should_refresh` uses the local timestamp, **not** the token's `iat`.

**The cap check protects the clamp — don't file the unfloored `min()` as a bug.** In `refresh_inner`
the `elapsed >= max_session_secs` guard runs *before* `new_exp = (now + TTL).min(session_deadline)`,
which forces `new_exp > now_unix` on both branches (`now >= iat`: `elapsed < cap` implies
`now < iat + cap`; `now < iat` from backwards clock skew: `elapsed` floors to 0 and the deadline
still exceeds `now`). So refresh cannot emit an already-expired token even though nothing re-checks
the *outgoing* expiry. I filed this as a MUST-FIX and was wrong. The real residual is
`max_session_secs <= 0` refusing every refresh, which contradicts `0 = disabled` elsewhere.

**Fixed-size quota tables: reset-on-tag-mismatch is a bypass, not a safety valve.** A slot that
stores the key's tag and resets when a *different* key lands on it looks fail-open toward the
victim — but the attacker controls a key too, so two colliding keys alternated (A,B,A,B,…) reset
each other forever and the counter never reaches the limit. With an unseeded FNV-1a the colliding
pair is computable offline. Correct shape: colliding keys **share** one counter (fail-closed for
whoever collides) plus a per-process random hash seed so collisions cannot be aimed or precomputed;
add a `with_seed()` test seam or the collision tests can't be written deterministically.
**The tell:** the suite contained `slot_collision_resets_rather_than_denies`, which asserted the
vulnerable behaviour *as an invariant* and passed. A green test encoding the bug is how it survived
review — when auditing a limiter, read its tests as claims to attack, not as evidence of safety.

**`iat` has zero verifiers.** Nothing in `crates/` reads `TokenClaims.iat` — mint/refresh/ingest all
read `exp` only, and `decode_token` reads neither. (`client-telemetry/src/hooks/iat_hooks.rs` is the
PE Import Address Table, unrelated — it pollutes every `grep iat`.) So preserving `iat` across refresh
is behaviourally invisible; the only casualty is the `exp − iat = 8 hours` line in
`docs/architecture/dev-session-telemetry.md`, and there is no external verifier left to break since
the Functions/Cosmos path is decommissioned. The module docstring still claims "the Functions side
verifies the token independently" — stale, contradicts the constant block below it.

**The launcher honours `Retry-After` only on 503.** A 429 falls through to a generic status error, so
a `Retry-After` on a 429 is decorative for this client (emit it anyway — correct HTTP). A 429 on
*mint* degrades gracefully: `start_session` fails and the game launches without telemetry. A 429 on
*refresh* does not — see the storm above. Refresh is signature-authenticated; the HMAC is the
authorization, so quota belongs on mint only.

See also [[admin-api-jsonwebtoken-unused]] (the token is hand-rolled HMAC, not a JWT).
