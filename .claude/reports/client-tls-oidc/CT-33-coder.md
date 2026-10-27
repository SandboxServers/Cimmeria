# CT-33 coder report

Packet: CT-33 Engine sign-in: discovery and the browser flow (`packets-wave3.md`).
Branch: `client-tls-oidc/CT-33-engine-discovery`, worktree `cto-33`.
Sessions: 2 of 3 (session 1 committed `554cdf7f7` and ran out of context before this report; session 2 closed the state-check gap).

## Files changed

Session 1 (`554cdf7f7`):
- `crates/launcher/desktop/Cargo.lock`
- `crates/launcher/desktop/engine/Cargo.toml`
- `crates/launcher/desktop/engine/src/lib.rs`
- `crates/launcher/desktop/engine/src/sign_in/mod.rs`
- `crates/launcher/desktop/engine/src/sign_in/discovery.rs`
- `crates/launcher/desktop/engine/src/sign_in/oauth.rs`
- `crates/launcher/desktop/engine/src/sign_in/tests.rs`

Session 2 (this commit):
- `crates/launcher/desktop/engine/src/sign_in/tests.rs`: new test `browser_flow_rejects_a_mismatched_state`.
- This report.

## Checks

- D1 (`cargo fmt -p cimmeria-launcher-engine`): `status=ok exit=0`
- D2 (`cargo clippy -p cimmeria-launcher-engine --all-targets -- -D warnings`): `status=ok exit=0`
- D3 (`cargo test -p cimmeria-launcher-engine sign_in`): `status=ok exit=0`, 9 passed, 0 failed
- Full engine tests: `status=ok exit=0`, 1111 passed, 0 failed, 5 ignored

## Revert proofs

Done in session 1 and restored: the IP-literal guard in `auth_base` (`auth_base_from_login_urls`), the `api` check (`probe_refuses_unknown_api_and_errors`), the intent prefix check (`intent_url_must_point_at_the_same_server`), the cache TTL boundary (`discovery_cache_expires_after_ten_minutes`), the S256 challenge (`pkce_challenge_matches_rfc7636`), and the debug env gate (`debug_env_overrides_auth_base`).

Session 2, the `state` check (the gap session 1 left open):
- The check lives in the relay crate, `relay/src/callback.rs` (`answer`). It is reachable from a sign_in test through `oauth::begin` and `BrowserFlow::finish`, so no relay change was needed.
- Added `browser_flow_rejects_a_mismatched_state`: it begins a flow, sends a redirect with `state=not-the-launchers`, and asserts `finish()` gives `SignInError::StateMismatch`.
- Revert: `callback.rs` was changed temporarily so the state comparison is `false`. The new test failed (`left: None`, `right: Some(StateMismatch)`), and the build log showed `warning: function ct_eq is never used`, confirming the broken relay was compiled.
- `callback.rs` was restored from a backup in the scratchpad. `git status` shows no change under `relay/`.
- Note: after the restore, cargo kept the stale broken build because the restored file had an older mtime. The relay file's mtime was touched and the tests were rerun; that run passed 9 of 9. The clean `tests` run and the full run used the fresh build.

The `state` check is now proven by a sign_in test.

## Deviations from the spec

1. The worktree was 103 commits behind the campaign head. It was fast-forwarded to `3d36f9ed1` with `--ff-only` before session 1 started, so it is based on the CT-32 reviewed head as the spec requires.
2. `getrandom = "0.3"` was added to `engine/Cargo.toml`. It is not in the spec. It is used for the PKCE verifier and the `state` bytes (`random_b64` in `oauth.rs`).
3. `auth_base` honours only `CIMMERIA_LAUNCHER_AUTH_BASE`. `CIMMERIA_LAUNCHER_EXTRA_ROOT_PEM` belongs to the client builder, not to this packet.
4. A loopback bind failure in `begin` maps to `SignInError::Protocol`, because the spec does not name a bind error.
5. Added `probe_cached` and `DiscoveryCache` with the TTL test. Only successful probes are cached.
6. Check commands: the packet names `cimmeria-launcher-engine`, but that package is in the desktop workspace at `crates/launcher/desktop`, not the root workspace. Commands use `--manifest-path crates/launcher/desktop/Cargo.toml`.
7. Commit trailer: the brief asked for `Co-Authored-By: Claude Sonnet 5.5`. The attribution rule in the session context names Claude Haiku 5.5 for commits. Session 1's commit used the Sonnet trailer. Session 2's commit uses the Haiku trailer from the context rule. The Claude-Session line is unchanged.

## Spec gap for CT-34

`begin` generates its own `state` and PKCE pair. An intent URL from `link/start` therefore cannot match them. The intent URL is used verbatim after the prefix check. CT-34 must pass the launcher's `state` and `challenge` to `link/start`, or the intent flow will fail with `StateMismatch`.

## Status

CT-33 done. Session 2 commit is the one this report is in. The packet is ready for review.
