---
name: Signed rollback fixture
description: NativeHost rollback verification and fixture close semantics
type: reference
---

2026-10-04: `shell/src/host/game_update/integration_tests.rs` under
`crates/launcher/desktop/` uses two valid signed inert ZIP releases. The earlier
padded catalog fixture cannot execute a rollback download. The portable engine
fixture appends `previous.txt` to distinguish a valid previous release.

Rollback creates a new evidence identity even when its manifest digest matches
the original release. Assert the current identity equals the rollback plan target,
and separately compare the original digest. Do not equate the evidence UUIDs.

Terminal operation observation can precede async observer teardown. Fixture reopen
must drop the host and wait for retained store references to drain before opening
the same state directory. This models process quiescence and avoids false Busy
failures; it does not relax the production state lock.

Host tests verify immutable owner bytes, independent retained original/current
backups, no local modification merge, duplicate rollback suppression, partial
preparation cancellation/discard and lost handoff abandonment/discard after reopen.
The mounted Effect UAT executes rollback and lost-reply inspection across reopen.
Native Windows, real Wine helpers/client and visual webview remain separate gates.
