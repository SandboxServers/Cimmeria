---
name: tracing-capture-single-dispatcher-trap
description: A hand-rolled tracing capture via dispatcher::set_default sees nothing under `cargo test` (passes under nextest) unless a second Dispatch is kept alive; plus process-wide test-coupling from static async locks.
metadata:
  type: reference
---

Found 2026-10-10 in LX-08a (desktop launcher telemetry transport tests,
`crates/launcher/desktop/engine/src/storage/game_telemetry/transport/tests.rs`).

- **Single-dispatcher interest trap.** With exactly one live `Dispatch`,
  tracing-core sets `has_just_one` and computes a *new* callsite's interest
  from the default of whichever thread hits it first. Under `cargo test`
  (threads in one process) that is often another test's thread with no
  subscriber, so the callsite is cached as never-enabled and a scoped
  capture on your thread records nothing. Nextest (one process per test)
  hides it. Fix: keep a second `tracing::Dispatch::new(...)` alive for the
  test; registering it also rebuilds stale interest.
- The desktop engine has no `tracing-subscriber`; a ~30-line `Subscriber`
  impl with a `Visit` that writes `name=value` is enough, filtered to
  targets starting with `cimmeria` so hyper/reqwest traces stay out.
- **Static async locks couple tests.** A process-wide `tokio::sync::Mutex`
  around flushes made a Retry-After-1s test flake under `cargo test` while
  another test held the lock for a second. Key such locks by the resource
  (queue directory) instead.
- Desktop engine `nextest run` without `--lib` fails at list time: its
  `harness = false` integration tests print `PASS ...` lines nextest cannot
  parse. Use `--lib`, or `cargo test` for D3.

Related: [[testing-patterns-index]], [[observability-test-and-throttle-traps]].
