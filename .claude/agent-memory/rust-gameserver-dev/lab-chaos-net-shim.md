---
name: lab-chaos-net-shim
description: Lab-chaos net shim (CH-06) layout - shared seeded filter in mercury lossy_transport::filter, ChaosTransport in base, has_net fast path, receive side drop-only, test seam
metadata:
  type: project
---

Lab chaos `net` (CH-06, 2026-10-11, branch `lab-chaos/ch06-net`):

- Seeded filter logic is shared, not copied: `cimmeria_mercury::lossy_transport::filter`
  (`roll_send` draw order drop -> dup-if-survived -> latency is the replay
  contract; `FlowFilter` = config + separate send/recv ChaCha20 streams;
  `SendShaper` = per-destination delay lines + reorder buckets, `release(addr)`).
  `LossyTransport` and the lab shim both call it.
- `crates/base/src/base/lab_chaos_transport.rs` is the only base file allowed
  to call `lab_chaos::current()` (`crates/server/tests/lab_chaos_callers.rs`);
  no `use ...lab_chaos::{..}` grouped imports anywhere outside the module.
- Receive side is drop-only with no consecutive-drop cap: a sleep or an error
  in `recv_from` hits every session (one receive task).
- `LabChaos::has_net()` is kept in step by a `SessionsGuard` Drop that
  recounts under the lock; add new map writers through `sessions()`.
- Base's `async-trait` is a dev-dep normally; the shim needs it as an optional
  dep enabled by `lab-chaos` (`dep:async-trait`); hakari stays clean.
- Test seam: a recording fake `BidirectionalTransport` + a leaked
  `LabChaos::new()` (never the process-wide registry). Registry expiry uses
  `std::time::Instant`, so expiry tests need a real ~1.1 s sleep.

Related: [[lab-daemon-lease-gate]], [[cell-systems-index]].
