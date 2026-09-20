---
name: reference-dev-session-telemetry-trust-boundary
description: Dev-session mint/refresh + telemetry ingest trust boundary — unauth mint, decorative scope/iss claims, install_id published at info, and why per-identity quotas are victim-targeted
metadata:
  type: reference
---

# `/api/auth/dev-session` + `/api/telemetry/*` trust boundary

From the #441 review (2026-09-19). Durable shapes, not a diff summary.

## Decorative claims: `scope` and `iss` are never verified
`TokenClaims.scope` is written at mint, copied verbatim through refresh,
and read by **no verifier** — `verify_bearer` in
`crates/admin-api/src/routes/telemetry/handlers.rs` checks signature +
`exp` only. Same for `iss`. Harmless today (only one minter, one scope),
but the dev_session module doc asserts the scope *is* the containment
("the worst an attacker can do is upload garbage telemetry") — the
threat model leans on a check that doesn't exist. Latent confused-deputy
the day a second endpoint signs with the same
`CIMMERIA_TELEMETRY_HMAC_SECRET`.

## `install_id` is not a secret — it is published at `info`
The ingest handlers log `install_id = %claims.sub` at **info** on every
replayed event, so it reaches `logs/*.log`, SigNoz, and the `/ws/logs`
admin WebSocket — which is on the same unauthenticated 0.0.0.0 port.
**Consequence for any future design:** a per-`install_id` rate limit,
ban list, or quota is a *victim-targeted* denial tool, because the
identity is harvestable by anyone who can reach the admin port. Per-IP
must be the enforcing key; per-identity buckets are observe-only or
gated on an attacker signature (one IP presenting many identities).

## Quota-map-as-amplifier (general shape, not specific to this handler)
`/api/auth/dev-session` has no `DefaultBodyLimit` override, so axum's
2 MiB default applies and a single client string field can be ~2 MiB.
Keying a server-side window map on a raw client String turns the
mitigation into a memory amplifier. Rule: **cap + charset-validate +
reject, then key on a fixed-width digest**, and check the cap before
the claim is built and before any map touch. Corollary: bound the map's
*entry count* too, and on saturation return 429 rather than evicting —
an LRU-evicting quota map is bypassable by flooding fresh keys to evict
your own throttled bucket.

## `iat`-as-chain-start is safe here
The launcher tracks `issued_at_ms` locally
(`crates/launcher/src/telemetry/mod.rs`) rather than reading the token's
`iat`, so repurposing `iat` as a refresh-chain start does not perturb
the client's refresh cadence. Clamp `new_exp` to `iat + MAX`; a bare
refusal at the boundary leaves the last token valid a full TTL past it.
A max-chain-lifetime bound is hygiene, not an attacker control — mint is
unauthenticated, so an attacker just starts a fresh chain.

Related: [[exploit-admin-api-unauth]], [[reference-auth-exploit-classes]].
