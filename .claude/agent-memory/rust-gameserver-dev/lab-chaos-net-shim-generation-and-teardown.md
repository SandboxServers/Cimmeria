---
name: lab-chaos-net-shim-generation-and-teardown
description: Lab chaos net shim (CH-06): per-set generation for replay, discard (never release) held/delayed sends at teardown, bounded delay line, paused-clock test recipe
metadata:
  type: project
---

The per-session net shim (`crates/base/src/base/lab_chaos_transport.rs`) keys
its state on `SessionChaos::generation` (new on every `LabChaos::set`, read
through `LabChaos::net_entry_for`), not on the knobs: comparing knobs let an
identical re-set keep the old RNG stream, so a runner row did not replay.
Comparing `expires_at` would not work either: tests set with a fixed `now`.

**Why:** a client address can belong to a new session right after teardown.
Releasing a dead peer's reorder bucket, or draining its delay line, sent the
old session's packets to the new client. `SendShaper` flows
(`lossy_transport/filter.rs`) are generation-tagged; `discard` drops the flow,
and dropping the `SendLine` drops a oneshot sender the line task selects on
(`biased`), so queued items are thrown away, not sent.

**How to apply:**
- Peer (re)insert re-reads the registry under the peers lock, and a send
  re-checks its generation after `deliver_as`: that closes the "read Some
  before teardown, insert after the sweep" race. Lock order: peers, then
  registry, then shaper flows.
- The delay line is a bounded `mpsc::channel(MAX_DELAYED_PER_DEST)` with
  `try_send`; Full = shed (counted), the shim WARNs at most every 10 s.
- Testing delay lines: `#[tokio::test(start_paused = true)]` on the
  current-thread runtime. The spawned line task never runs while the test
  only awaits non-pending futures, so `cap + 100` sends shed exactly 100;
  then `tokio::time::sleep` auto-advances past the latency. `cimmeria-base`
  dev-deps carry tokio `test-util` for this.
- Mutation-check the sweep and the cancel with `a_dead_peer_is_swept_by_any_send`.

Related: [[lab-uat-runner-in-process-tool-calls]].
