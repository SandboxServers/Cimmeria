---
name: project-launcher-summary-observability
description: "Launcher-summary observability (track O) built 2026-10-04: anonymous strict ingest (no mint or scope, 12 requests a minute per address), public login-port mount approved, inert by design (no endpoint), where the hooks live"
metadata:
  type: project
---

The consented launcher-summary flow was implemented on 2026-10-04 on branch
`launcher/observability-implementation` (base `ba4b20b6b` of
`prototype/launcher-packaging-proof`). Handoff:
`docs/analysis/playtests/2026-10-03-macos-wine/worknotes/observability.md`.
Design: `docs/architecture/launcher-summary-telemetry.md`.

**Status, later on 2026-10-04:** the owner decided the upload is anonymous.
That replaced the first design, a `launcher_summary` session kind with its own
scope and mint allowance, which no longer exists in the code. The
install-admission gap this note used to list as open is closed (see below).

- **The upload is anonymous.** One `POST /api/telemetry/launcher-summary` of
  the exact schema-1 payload: no mint, no token, no scope and no
  `Authorization` header. The server refuses anything else whole and limits
  each peer address (`CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`, default 12 per
  minute, charged before the body is read). The window is a fixed 60 s of the
  route's own; the mint and refresh quotas keep theirs
  (`CIMMERIA_TELEMETRY_QUOTA_WINDOW_SECS`, an hour). A dev-session mint for kind
  `launcher_summary` is refused as an unknown kind. Rows are self-reported, so
  never alert on them or read a success rate from them.
- **It is inert in every build.** The shell passes `endpoint: None`
  (`crates/launcher/desktop/shell/src/host/summary.rs`), so nothing is tracked,
  queued or sent. Tests inject a loopback endpoint.
- **Owner decisions, 2026-10-04:** the route may be served on the public login
  port (8081), which extends the four-route decision of 2026-09-29 to it as a
  fifth, and its rate limit is 12 requests per minute per peer address.
- **Still open, as a later and separately decided rollout:** a production
  endpoint, the consent copy change and the frontend UAT. The exporter accepts
  only `https` (bundled webpki roots, so a publicly trusted certificate) or
  loopback `http`, so the plain-HTTP login port cannot be the launcher's
  endpoint.
- **The only producer hook is the journal.** `FileJournal::commit` in
  `engine/src/storage/mod.rs` calls the observer; rows are finalized lazily at
  `operations_mut()`, `save_preferences_with`, the exporter's batch take and
  the start of `admit_install_with`, and right after an install terminal in
  `install_worker::publish`. A new operation kind needs no hook of its own.
- **Closed 2026-10-04: the install-admission gap.** `admit_install_with`
  commits without `operations_mut()`, so a failed launch's row finalized after
  an install admission used to lose its error code. It now calls
  `finalize_summaries()` first (`storage/install_intent/mod.rs`). Any other
  path that commits on the journal without `operations_mut()` needs the same
  call.
- **A 429 ends the exporter's cycle with no retry.** A retry would spend more
  of the address's allowance; the rows wait for the next trigger.
- **Launch rows deliberately carry no `running` phase and no total duration**
  (that would be play-session length). The rule is in
  `launcher_summary/attempt.rs`.
- **The wire contract is the golden files** under
  `engine/src/storage/launcher_summary/fixtures/`: three request bodies and one
  response, no mint fixture. Both workspaces test against them; a change must
  pass the engine suite and the admin-api suite.

**Why:** the assignment required an inert, consent-gated flow with the
activation left to the maintainer.

**How to apply:** before changing any of this, read "Decisions and what is
still open" in the design doc and do not add an endpoint, an env override or a
frontend copy change without the rollout decision. Related:
[[reference-desktop-engine-test-seams]],
[[reference-loopback-exporter-test-seams]].
