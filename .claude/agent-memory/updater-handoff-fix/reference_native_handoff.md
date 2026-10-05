---
name: Native updater successful handoff signal
description: Post-spawn persistence failure must not strand the old process lock.
type: reference
---

2026-10-04: `engine/src/storage/updater/apply.rs` records durable handoff intent
before spawning. That intent cannot prove spawn succeeded. The native callback
emits only after successful spawn; `shell/src/host/updater/mod.rs` retains the
signal through an error return and invokes shutdown once. Installation completion
still belongs to compiled-version startup reconciliation.

Paths above are relative to `crates/launcher/desktop/`. See its `docs/updater.md`
and the engine-to-host fault regression in `shell/src/host/updater/handoff_tests.rs`.
