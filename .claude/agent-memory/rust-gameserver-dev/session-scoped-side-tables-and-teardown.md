---
name: session-scoped-side-tables-and-teardown
description: Where to clear per-address side tables (lab chaos etc.) on session end, and why a relaunched client cannot inherit them; logOff keeps the address
metadata:
  type: project
---

A side table keyed by client `SocketAddr` (lab chaos, CH-04, 2026-10-11) is cleared in
`teardown_session` (`crates/base-session/src/base/helpers/session_teardown.rs`) after the
owner check. That is enough for a relaunch takeover: `evict_prior_sessions`
(`crates/base/src/base/login/eviction.rs`) calls the un-owned `destroy_client_entities` on the
old session before the caller inserts the new one. The old tick loop's late
`destroy_owned_client_entities` finds a different `cancelled` Arc and returns before the clear,
so it cannot wipe state set on the new session.

Gaps the teardown clear cannot see: `logOff` (`crates/base/src/base/dispatch/session.rs`) keeps
the session and address but ends the player entity (no teardown), and a resolve-then-set
caller can race a teardown. Tag entries with the player entity id and drop stale ones where the
current entity is known (`LabChaos::clear_if_not`).

**Why:** the reviewers asked for "a replacement session never inherits chaos"; tracing the
eviction order showed the teardown hook alone covers takeover.
**How to apply:** for any new per-address state, hook teardown after the owner check, and
decide separately what a logOff to character select should do. Source-scan guards in
`crates/server/tests/` build their needles with `concat!` so the test file never matches itself;
see [[testing-patterns-index]].
