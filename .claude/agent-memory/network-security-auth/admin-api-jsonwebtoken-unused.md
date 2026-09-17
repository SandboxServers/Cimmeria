---
name: admin-api-jsonwebtoken-unused
description: jsonwebtoken is a declared-but-never-called dependency of cimmeria-admin-api; the real dev-session token is hand-rolled HMAC-SHA256, so JWT-library CVEs/bumps carry no validation-semantics risk
metadata:
  type: project
---

`jsonwebtoken` is declared in the root `Cargo.toml` (`[workspace.dependencies]`) and in
`crates/admin-api/Cargo.toml`, but **has zero call sites anywhere in the workspace**.
Verified 2026-09-17 (PR #626, v10 -> v11 bump): no `use jsonwebtoken`, no `jsonwebtoken::`
path, no `extern crate`, no `package = "jsonwebtoken"` rename alias. Its only mention in
Rust source is a TODO comment at `crates/admin-api/src/middleware.rs:24`.
`docs/tools/admin-api.md:434` independently confirms the JWT auth middleware is still a TODO.

The token that actually exists is **not a JWT**. `/auth/dev-session` mints a hand-rolled
`base64url(payload_json).base64url(HMAC-SHA256(payload_b64))` token in
`crates/admin-api/src/routes/dev_session.rs` (`encode_token` / `decode_token`, ~lines 307-342)
using `hmac` + `sha2` + `base64`. It verifies with `mac.verify_slice()` (constant-time), and
`exp` is enforced by the *callers*, not by `decode_token` — at `dev_session.rs:218` (refresh)
and `telemetry/handlers.rs:274` (ingest). `decode_token` itself only checks length cap,
separator, signature, and JSON shape.

**Why:** a security reviewer seeing `jsonwebtoken` in the manifest will assume JWT validation
semantics (Validation defaults, algorithm allow-lists, `alg: none` confusion, clock skew) are
in play. None of that surface is reachable. Conversely, any real weakness lives in the
hand-rolled HMAC path, which no JWT-library audit or advisory will ever cover.

**How to apply:** when triaging a `jsonwebtoken` advisory or version bump, confirm the call-site
count is still zero and it is a no-op — do not re-derive the whole validation analysis. When
auditing admin-API auth for real, audit `dev_session.rs`, not the JWT crate. If the
`middleware.rs:24` TODO is ever implemented, this memory goes stale — re-verify then, and see
[[security-audit-2026-05-31]] (finding 3: dev-session mints tokens to any caller, `sub` is
attacker-controlled).

**Low-severity standing item:** an unused crypto dependency is needless supply-chain surface
and audit noise. Either implement the middleware or drop the dep from both manifests.
