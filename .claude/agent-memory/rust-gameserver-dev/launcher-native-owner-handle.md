---
name: Launcher native ownership reads
description: Launch preparation must read installation identity through its retained lock handle.
type: project
---

2026-10-04: `crates/launcher/desktop/engine/src/storage/launch/worker.rs`
acquires an exclusive file lock before reading `.cimmeria-install.json`.
Use `storage::read_open(&owner)` rather than reopening the path: Windows byte
range locks exclude a second handle even in the same process. Keep the owner
handle in the returned `Ownership` until guest supervision completes.

`native_preparation_reads_locked_owner_and_retains_exclusion` exercises actual
client preparation with an inert PE32 fixture and checks competing lock exclusion
and release. Unix can validate preparation and lock lifetime, but only native
Windows can establish the read-through-lock regression and its revert proof.
