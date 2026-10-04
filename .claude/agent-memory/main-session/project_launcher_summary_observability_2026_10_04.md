---
name: project-launcher-summary-observability
description: "Launcher-summary observability (track O) built 2026-10-04 on launcher/observability-implementation: inert by design, two open maintainer gates, where the hooks live"
metadata:
  type: project
---

The consented launcher-summary flow was implemented on 2026-10-04 on branch
`launcher/observability-implementation` (base `ba4b20b6b` of
`prototype/launcher-packaging-proof`). Handoff:
`docs/analysis/playtests/2026-10-03-macos-wine/worknotes/observability.md`.
Design: `docs/architecture/launcher-summary-telemetry.md`.

- **It is inert in every build.** The shell passes `endpoint: None`
  (`crates/launcher/desktop/shell/src/host/summary.rs`), so nothing is tracked,
  queued or sent. Tests inject a loopback endpoint.
- **Two maintainer decisions are open:** the public login-listener mount (its
  own commit, do not squash) and any production endpoint. The exporter refuses
  plain `http` except to loopback, so the 8081 login port cannot be the
  endpoint.
- **The only producer hook is the journal.** `FileJournal::commit` in
  `engine/src/storage/mod.rs` calls the observer; rows are finalized lazily at
  `operations_mut()`, `save_preferences_with` and the exporter's batch take,
  and right after an install terminal in `install_worker::publish`. A new
  operation kind needs no hook of its own.
- **Known gap:** `admit_install_with` commits without `operations_mut()`, so a
  launch row finalized after an install admission loses its error code if the
  exporter is dead. One `finalize_summaries()` call there fixes it.
- **Launch rows deliberately carry no `running` phase and no total duration**
  (that would be play-session length). The rule is in
  `launcher_summary/attempt.rs`.
- **The wire contract is the golden files** under
  `engine/src/storage/launcher_summary/fixtures/`. Both workspaces test against
  them; a change must pass the engine suite and the admin-api suite.

**Why:** the assignment required an inert, consent-gated flow with the public
activation left to the maintainer.

**How to apply:** before changing any of this, read the worknote's "Remaining
public-activation gate" and do not add an endpoint, an env override or a
frontend copy change without that decision. Related:
[[reference-desktop-engine-test-seams]].
