---
name: logcapture-helper-location
description: The LogCapture test helper for asserting which tracing event fired lives in crates/services/src/test_support.rs
metadata:
  type: reference
---

`crate::test_support::LogCapture::install()` returns a `LogCaptureGuard` that captures every `tracing::event!` call on the current thread. Drop the guard to restore the previous subscriber.

API surface:
- `install()` — returns the guard. **Must be called from a `#[tokio::test]` (current-thread) runtime, not multi_thread.** It panics if called inside a multi-thread runtime because `set_default` is thread-local.
- `guard.find_event(level, message_substr, reason_value)` — finds the first event matching all three filters. Use this when the negative log carries a `reason: "..."` field (the project's convention per `docs/architecture/negative-logging-convention.md`).
- `guard.find_message(level, message_substr)` — finds the first event matching level + substring, ignoring fields. Use when the log doesn't have a `reason` field.
- `guard.all()` — returns every captured event (useful inside `assert!` failure messages to debug what *did* fire).

Examples in the codebase:
- `crates/services/src/base/character/delete_live_db_tests.rs` — `use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};`
- `crates/services/src/base/helpers/tests.rs` lines 195, 235, 272, 331, 362, 393, 447 — many use cases for negative-log assertions.
- `crates/services/src/cell/cell_methods/player/dispatch.rs` (after commit `fc821bd6`) — example of using `find_message` to distinguish which dispatcher branch fired.

Self-tests live at `crates/services/src/test_support.rs::log_capture_tests` if the helper's behavior is in doubt.

Reference field names per the negative-logging-convention doc: `reason`, `entity_id`, `player_id`, etc. Stable across the test suite; renaming requires touching every guard.
