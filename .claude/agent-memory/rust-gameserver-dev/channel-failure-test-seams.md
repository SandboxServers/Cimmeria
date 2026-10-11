---
name: channel-failure-test-seams
description: How to make a tokio mpsc send fail mid-loop, and a DB query fail, in a unit test with no live database (CH-13 review, 2026-10-11)
metadata:
  type: reference
---

- A `tokio::mpsc::Sender::send` never errors on a **full** channel, it waits. To fail a send part-way through a loop: a one-slot channel and a spawned reader that takes k messages then drops the receiver (current-thread test runtime). The exact number delivered depends on tokio's permit hand-off (a waiter already granted a permit may "succeed" into the closed channel), so assert on ranges (some delivered, some refused), not an exact count.
- A dropped receiver before the call makes **every** send fail: good for "nothing delivered", useless for "partial".
- DB query failure without a live DB: `PgPoolOptions::new().acquire_timeout(2s).connect_lazy("postgres://u:p@[::1]:9/x")`; the first query errors in about 2 s. Use `[::1]:9`, not 127.0.0.1 (see [[testing-patterns-index]]).
- Testing a 1 Hz tick is wired into the cell loop: run `run_cell_loop` in a spawned task under `#[tokio::test(start_paused = true)]`, keep the base channel's sender and the cell→base receiver alive, wait on the reply with `tokio::time::timeout`; abort the task at the end (`service/tests/spawn_batch_loop.rs`).
