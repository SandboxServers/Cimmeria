---
name: lab-runner-paused-time-poll-tests
description: cimmeria-lab runner poll/timeout loops are tested with #[tokio::test(start_paused = true)]; loops must use tokio::time::Instant, not std
metadata:
  type: project
---

`cimmeria-lab` has `tokio` `test-util` in its dev-dependencies, so a runner test can use
`#[tokio::test(start_paused = true)]` to run a multi-second poll loop (SV-07 `@stand`: 250 ms
polls, 5 s timeout) in milliseconds. The fake client and `FakeServer` never await real I/O, so
the paused clock auto-advances through each `sleep`.

**Why:** before SV-07 no runner test drove time; a real 5 s timeout test would slow the suite.

**How to apply:** a runner loop with a deadline must take it from `tokio::time::Instant::now()`
(a `std::time::Instant` deadline never expires under a paused clock and the test hangs). Script
the fake's replies with `.answer(tool, vec![..])`: the last reply repeats, so "never arrives" is
one reply. See [[testing-patterns-index]].
