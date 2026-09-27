---
name: project-na39-channel-tx-window-ordering
description: NA39 (2026-09-26) found and fixed a real cross-task ordering bug, but it's in Channel (mercury crate), not the AoI/witness layer
metadata:
  type: project
---

NA39 (PR #830, branch `npcai/na39-cross-task-ordering`) chased NA38's named
next candidate for the "A sees B, B doesn't see A" one-way visibility
report: cross-task send ordering to a single client's Mercury channel.

**Verdict: real hazard confirmed, but one layer below AoI/witness-fanout.**
[[reference_witness_fanout_helper]]'s three helpers
(`send_to_witness`/`send_to_witness_reliable`/`send_bundle_to_witness_reliable`,
now `crates/base-session/src/base/helpers/mod.rs` post-#825 split, was
`crates/services/src/base/helpers/mod.rs`) reserve a sequence via
`ConnectedClientState::next_seq.fetch_add` under one lock, then await the
socket send, then call `Channel::register_sent_packet`
(`crates/mercury/src/channel/channel_core.rs`) under a *different* lock.
That gap lets a second concurrent task on the same witness (Base's
recv-loop, a `tokio::spawn`ed fan-out) register a later-allocated
sequence first, leaving `tx_window` non-monotonic. `process_acks`'s
cumulative-ACK drain is front-only by design, so an out-of-order higher
sequence parked at the front blocks an already-acked lower sequence
behind it from *ever* draining — the client's 32-slot TX window
degrades toward full under routine concurrent traffic, starving new
reliable sends (including future `CREATE_ENTITY`s) to that one client.

**AoI/witness layer itself was cleared, not implicated:** CellService's
`run_cell_loop` is single-task (confirmed via grep, no internal
`tokio::spawn` under any post-split `cell-*` crate — `cell`, `cell-world`,
`cell-combat`, `cell-console`, `cell-content`, `cell-cover`,
`cell-interactions`, `cell-methods` — the only spawn in that family is
`crates/cell/src/cell/service/startup.rs`'s top-level launch of the loop
itself). Base's `cell_rx` drain is likewise single-task, fully awaited
per message, so `entered_aoi`'s own create-then-cascade ordering
(`crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi.rs:84`)
can't race with itself. Base-originated fan-out to *other* clients always
proxies through Cell (`broadcast_to_witnesses` →
`BaseToCellMsg::BroadcastToWitnesses` →
`CellToBaseMsg::WitnessEntityMethod`,
`crates/base-session/src/base/helpers/witness_broadcast.rs`), never sends
directly.

**Fix:** `register_sent_packet` now inserts into `tx_window` and
`unsent_packets` at the sorted position (`insert_tx_entry_sorted`,
reusing `process_acks`'s 28-bit modular comparator as `seq_mod_leq`)
instead of `push_back`. Allocation itself untouched. Three revert-proven
unit tests in `crates/mercury/src/channel/tests/channel_lifecycle.rs`.

**Open follow-ups (not blocking, noted in the PR):**
- No live two-task concurrency test proving the OS scheduler actually
  interleaves in the adversarial order — tests pin the `Channel`-level
  invariant directly.
- No SigNoz corroboration yet (`mercury.backpressure` WARN clustering).
- NA38's other two candidates remain open: AoI radius/space-id mismatch,
  and a create emitted without its appearance/cascade follow-up
  (`aoi.create_emit`/`create_send_failed`, see
  [[reference_periodic_aoi_tick_covers_midsession_spawn]]).

**Post-#825 crate-split gotcha:** worknotes/work-packets written before
the split will have stale `crates/services/src/...` paths. When resuming
paused NA-series work after a restructuring, grep the referenced paths
first — `crates/services` is now a thin facade re-exporting split crates
at their old *module* paths, but the *file* paths moved (e.g. `base/` →
`crates/base`, `base/helpers` → `crates/base-session`, `base/world_entry`
→ `crates/base-world-entry`, `cell/` → `crates/cell` + 7 sibling
`cell-*` crates per `crates/README.md`).
