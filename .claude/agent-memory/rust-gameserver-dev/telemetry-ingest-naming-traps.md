---
name: telemetry-ingest-naming-traps
description: Traps when adding fields to the client telemetry replay (admin-api routes/telemetry) — tracing's 32-field cap, no router state on the upload routes, docs/ absent from the container, ms-truncated row times in tests, lane log-dir prune race
metadata:
  type: project
---

Learned on NT-40 (2026-10-04), naming client.native rows at ingest.

- **`tracing` caps one event at 32 fields, message included.** The `client.native` replay outgrew it,
  so `replay_native.rs` has two shapes: rows with game IDs carry the ID/name pairs, other rows carry
  the status keys. Adding a field means picking a shape and recounting.
- **Never put public-ingest work on the base->cell gameplay channel** (security review of #1215).
  The ingest has its own 4-slot `EntityLabelsRequest` channel, read by the cell loop only under
  `if rx.is_empty()`, one request in flight (`Semaphore::try_acquire`), budget gate before naming,
  cell skips `reply_tx.is_closed()`, departed rings indexed by entity ID. Names are client-claimed.
- **The upload routes have no router state.** They are mounted on the admin listener and on the
  public login port (`login_port_telemetry_router`), so cell access is the process-global label
  link from `connect_entity_labels`, set in `main.rs` after `start_all`. Tests pass a sender explicitly
  (`replay_ndjson_named`).
- **`docs/` is in `docker/Dockerfile.dockerignore`.** Anything `include_str!`'d must live under
  `crates/` (or entities/tools) — the symbol table is `routes/telemetry/client_symbols.tsv`.
- **Row server times are whole milliseconds.** A test that destroys and recreates a slot and then
  replays "now" in the same millisecond names the old occupant; sleep a few ms first.
- **Lane log-dir race:** another lane's `prune_logs` deletes a worktree's still-empty log dir while
  the job waits for a slot, so the job dies with "No such file" on its log. A `.keep` file in
  `%LOCALAPPDATA%\cimmeria-build\logs\<worktree>\` works around it.

Related: [[observability-test-and-throttle-traps]], [[tracing-span-fields-not-on-log-records]].
