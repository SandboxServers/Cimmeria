# LT-01 / LT-02 security review (network-security-auth)

Branch `launcher-telemetry/server` against `origin/main`. Read-only. Verdict: **fix-notes** (no Critical or High; one Medium, four Low).

Checked and found sound:
- `valid_host` accepts only `[A-Za-z0-9.-]` names (at most 253 chars) or a bracketed IPv6 literal, with an optional 1 to 5 digit port. A path, userinfo, space or query cannot reach the derived URL. Its tests cover this.
- The https branch needs `x-forwarded-proto` to equal exactly `https`. A list such as `https, http`, `HTTP`, or a missing header all fail closed. The plain-http branch needs a loopback peer (canonicalised, so IPv4-mapped works) and a loopback URL. A local reverse proxy that forwards a public Host without the header gets 503.
- `endpoint_for` runs before any signing or quota charge, and in `refresh` after the token verifies. The mint INFO carries `upload_endpoint_source` and `upload_endpoint_https`, never the URL or the peer.
- `warn_if_unreachable` prints no URL and no peer.
- The three `refusal_log.rs` rows no longer write `peer`. The `admin.request` row drops it on `/api/telemetry*` and `/api/auth/dev-session*`. The login-port router does not layer `admin_request_row` and `request_span` records the path alone, so no peer row exists there.
- `AuthError::NoUploadEndpoint` maps to 503 in `token.rs:161` and is listed in `is_credential_refusal` as not a credential refusal. That match has no wildcard, so it is exhaustive.
- Accepted risk, no change: `Host` and `x-forwarded-proto` are caller-controlled when the server is reached directly. The worst a caller can do is steer its own token to a host of its own choosing. The response goes only to that caller and a POST is not cacheable.

## Fix notes

**A1 (Medium). The boot log can print credentials embedded in the env URL.**
`crates/admin-api/src/routes/dev_session/upload_endpoint.rs`, `printable` (about lines 563-567) and `log_boot`. `printable` only checks that the host is a DNS name. `host_of` strips `user:pass@`, so `https://user:secret@play.example/api/telemetry?token=x` passes and is logged whole in all four boot lines.
Change: make `printable` return `None` when the authority contains `@`, or when the URL contains `?` or `#`. Better, print only `scheme://host[:port]`. Add a test, `log_boot(Some("https://user:secret@play.example/x?k=v"))`, and assert the captured rows contain neither `secret` nor `k=v`.

**A2 (Low). The unauthenticated 503 body names the operator's env var.**
`crates/admin-api/src/routes/dev_session/token.rs:146-150`. `to_response` sends `self.to_string()` to any caller on the public login port. Shorten the `#[error(...)]` text to `no_upload_endpoint: no https telemetry upload endpoint is available`. Keep the `no_upload_endpoint` prefix, because the launcher and the tests key on it. The operator hint is already in the boot WARN (`log_boot`). Update the `starts_with` assertion in `upload_endpoint_tests.rs:295` only if the prefix changes.

**A3 (Low). The 503 test does not prove "no token, no quota spent".**
`upload_endpoint_tests.rs:280-300`. Moving `endpoint_for` after `launcher_kind::charge_ip` or `mint_ip.check_and_record` (handlers.rs:312-329) would still pass. Change the test: use `policy(1, 1, 1)`, refuse once with the Host and no `x-forwarded-proto`, then mint again with `headers(Some("play.example"), Some("https"))`. Assert the second mint succeeds, which shows the refusal charged nothing. Also assert that the refusal wrote no `Minted dev-session telemetry token` row, by wrapping the first call in `capture`.

**A4 (Low). Boot warnings miss a non-lowercase scheme or a scheme-less env.**
`upload_endpoint.rs`, `boot_warning` and `is_https` (`starts_with("http://")` and `starts_with("https://")` are case-sensitive). `HTTP://play.example`, or `play.example` with no scheme, gets no WARN. Compare case-insensitively with `eq_ignore_ascii_case` on the first 8 bytes. Add a catch-all WARN with `reason = "upload_endpoint_not_https"` for any env that does not start with `https://`, so a mistyped override cannot hide. Add a `boot_warning` case for each.

**A5 (Low, cosmetic). `IngestError::reason()` maps `NoUploadEndpoint` to `bad_token`.**
`crates/admin-api/src/routes/telemetry/dto.rs:159`. Ingest never raises this variant today, so the row is unreachable. The wildcard would still file a future occurrence as a credential failure. Add `IngestError::Auth(AuthError::NoUploadEndpoint) => "no_upload_endpoint"` before the wildcard.

## Out of scope but flagged (owner directive 6)

`dev_session/handlers.rs:505-556` `log_refusal` still writes `peer = %peer` on all seven arms. This is LT-38's remit (work-packets.md:510). Directive 6 is not met for dev-session refusals until LT-38 merges. LT-38 must also add a `NoUploadEndpoint` arm. Today `_ => {}` swallows it, so the 503 is silent (the `// LT-38 logs this refusal` comment is there).

## Revert-sensitivity of the tests

- The peer-free ingest rows are guarded by `assert_no_peer_address` for all three rows. Re-adding `peer = %who.peer` fails.
- `telemetry_request_row_has_no_peer` fails if `is_telemetry_path` is reverted. The `/api/auth/dev-session` side is covered only by the pure-function test. That is acceptable, but a real-router dev-session request test would be stronger (optional).
- `a_mint_logs_no_peer_ip_and_no_host_ip` fails if the URL returns to the INFO row.
- The `resolve` tests fail if the old http Host derivation returns.
