---
name: password-storage-argon2id
description: Phase 2a argon2id password storage — dual-column schema, TLS-gated plaintext, opportunistic on-login migration (server half implemented)
metadata:
  type: project
---

# Argon2id password storage (Phase 2a of #434)

Server-half implemented on branch `feat/434-encryption-foundation` (part of
PR #566).

**Why:** the 2009 stack stored unsalted SHA-1 password hashes. Phase 1 wrapped
auth transport in TLS; Phase 2 lets the patched client send plaintext over that
TLS so the server can do real (argon2id) hashing. Dual-column + opportunistic
migration avoids a flag day.

**How to apply:** when auditing/extending login credential handling, the logic
now lives in a dedicated module, not inline in handlers.

## Key files

- `crates/services/src/auth/credentials.rs` — `ClientCredential` enum
  (LegacySha1Hex / Plaintext), `classify_credential` (the TLS gate, unit-
  testable), `validate_credentials` (branches on `(password_algo, credential)`),
  argon2id hash/verify helpers, `migrate_to_argon2id`. Live-DB tests in
  `0x7000_1B00` sentinel window.
- `crates/services/src/auth/handlers.rs` — `handle_user_auth` extracts
  `over_tls: Option<Extension<TlsConn>>`, calls `classify_credential`, passes
  classification to `validate_credentials`. Plaintext over plain HTTP → error 2.
- `crates/services/src/auth/mod.rs` — `pub(super) struct TlsConn` marker.
- `crates/services/src/auth/tls.rs` — `tls_marker_layer` middleware (inserts
  TlsConn). Layered ONLY onto the `tls_app` clone in `service.rs` (~line 231),
  never the plain-HTTP `app`. This is the security boundary.

## Schema (db/sgw/Accounts/Tables/account.sql)

- `password varchar(64)` — now NULLable (was NOT NULL). Legacy SHA-1, NULLed on
  migration.
- `password_hash_v2 text` — argon2id PHC string, NULL until migrated.
- `password_algo smallint DEFAULT 1 NOT NULL` — 1=sha1_legacy, 2=argon2id.
- Seed accounts (`Seed/account.sql`) omit the new columns → start algo 1.

## Behaviour matrix (validate_credentials)

- (algo 1, LegacySha1Hex) → uppercase-hex compare; NO migration.
- (algo 1, Plaintext) → recompute SHA-1, compare; on match migrate to argon2id
  (UPDATE password_hash_v2, password_algo=2, password=NULL). Migration failure
  logged (negative-logging) but does NOT fail the already-verified login.
- (algo 2, Plaintext) → argon2id verify against password_hash_v2.
- (algo 2, LegacySha1Hex) → InvalidCredentials (can't verify argon2id from a
  hash; unpatched client locked out of migrated accounts).
- Missing row / disabled → same InvalidCredentials / AccountDisabled as before
  (existence never revealed).

## argon2id params

`Params::new(65536 /*64 MiB*/, 3, 1, None)`, `Algorithm::Argon2id`,
`Version::V0x13`, random per-hash salt via `SaltString::generate(&mut OsRng)`.
Deps: `argon2` 0.5 (std feature) + `sha1` 0.10, added as workspace deps.

## Still pending (NOT in this slice)

- Client-side plaintext patch (Phase 2b): LoginReplyHandler ctor `0x00DDED60`.
- Dropping the legacy `password` column once migration covers the player base.
