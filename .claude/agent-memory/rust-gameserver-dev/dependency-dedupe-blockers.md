---
name: dependency-dedupe-blockers
description: Which duplicate crate versions in the CI-gated workspace are pinned by upstream crates we must not bump (sqlx, axum ws, reqwest, rmcp, notify, utoipa, lzokay-native), plus the cargo-machete false positives and the live-db-test toolchain trap. Read before any dependency dedupe or unused-dependency pass.
metadata:
  type: project
---

Snapshot from the 2026-09-26 dedupe pass (branch `build/dep-dedupe`). Re-check
with `cargo tree -i <crate>@<ver> -e normal,build --workspace <gated excludes>`
before trusting any row: upstream releases move these.

## Families that are NOT ours to fix

| Family | Old version pinned by | Why it stays |
|---|---|---|
| digest/sha2/sha1/block-buffer/crypto-common/cpufeatures 0.10-gen | sqlx-core + sqlx-macros-core (sha2 0.10), axum `ws` -> tungstenite 0.29 (sha1 0.10), rust-embed-utils | sqlx 0.9 and axum 0.8 are frozen. Our Mercury crypto is already on the 0.11 generation; never move it backwards. |
| rand/rand_chacha/rand_core 0.9, getrandom 0.3 | tungstenite 0.29 (axum 0.8 pins `^0.29`) and opentelemetry_sdk (0.33 is still rand 0.9) | Needs an axum release that moves to tungstenite 0.30. |
| hashbrown 0.16 | sqlx-core + hashlink 0.11 | sqlx. |
| base64 0.22 vs 0.23 | 0.22: sqlx, axum, hyper-util, pem, tonic; 0.23: rmcp (+ reqwest 0.13.5) | Both sides are third-party; moving admin-api does not remove either. |
| tower-http 0.6 | reqwest 0.13.x (`^0.6.8`, follow-redirect) | Moving our crates DOWN to 0.6 is a behaviour change: 0.7.0 relaxed CORS `Vary` defaults and re-parented trace events. Wait for reqwest. |
| windows-sys 0.60 | notify 8.2 (9.0 is RC only) | |
| thiserror 1 | lzokay-native 0.1.0 (its only release, used by cimmeria-upk) | |
| zip 3 (+ rust-embed x3) | utoipa-swagger-ui 9 build script | Fixed by utoipa 6 + utoipa-swagger-ui 10 (-4 crates), but that also swaps the served Swagger UI 5.17.14 -> 5.32.6 and was days old; left for the dependabot PR. |
| syn 2 vs 3 | ecosystem split (serde_derive/thiserror-impl on 3; sqlx-macros, tokio-macros, tracing-attributes, zerovec-derive... on 2) | |

What WAS fixed: argon2 0.5 -> 0.6 (only consumer of rand_core 0.6 / getrandom
0.2 / password-hash 0.5 / blake2 0.10), jobserver 0.1.35 (getrandom 0.4), and
unused-dep removal (jsonwebtoken alone was 8 crates; opentelemetry_sdk/otlp
in cimmeria-observability kept tonic/prost in cimmeria-services' graph).

## argon2 0.6 API (password-hash 0.6)

`SaltString::generate(&mut OsRng)` + `hash_password(pw, &salt)` is now just
`hash_password(pw)` (16-byte getrandom salt); `PasswordHash`, `PasswordHasher`,
`PasswordVerifier` are re-exported at the argon2 crate root; the `std` feature
is gone. 0.5 and 0.6 derive byte-identical PHC strings; the guard is
`verify_argon2id_accepts_hash_minted_by_argon2_0_5` in `auth/credentials.rs`.

## cargo-machete false positives in this repo

- `md-5` in cimmeria-mercury: the lib is named `md5`. Ignored via
  `[package.metadata.cargo-machete]`.
- `cimmeria-common` in cimmeria-defs: only the build-script output pulled in
  with `include!` uses it. Ignored the same way.
- machete does not flag dev-dependencies here, and it counts a crate name
  in a comment as a use. A comment-stripping grep found the dev-only
  `tempfile` (discord) and `tokio` (observability) that machete missed.
- The Tauri apps (`cimmeria-app`, content/scene editors) still carry flagged
  deps: they need a built frontend `dist/` to compile, so the removals can't
  be verified cheaply. They are candidates, not confirmed.

## Tooling trap: live-db-test.sh uses the DEFAULT toolchain

`live-db-test.sh` runs plain `cargo nextest`, i.e. the machine default (1.94),
not `+1.98.1`, which rebuilds every dependency into a second artifact set.
Prefix `RUSTUP_TOOLCHAIN=1.98.1` so it reuses the `+1.98.1` build. See
[[lane-sh-masks-cargo-exit-code]] and [[tooling-filter-and-path-traps]].
