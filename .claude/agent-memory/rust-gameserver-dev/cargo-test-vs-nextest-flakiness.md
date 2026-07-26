---
name: cargo-test-vs-nextest-flakiness
description: Full-suite `cargo test -p cimmeria-services` has pre-existing order-dependent failures; nextest is the source of truth — don't assume you broke it
metadata:
  type: project
---

A full-suite `cargo test -p cimmeria-services --lib` run fails 1-2 tests
that pass in isolation. `cargo nextest run --profile=ci -p cimmeria-services`
passes all of them. This is **pre-existing**, not something your change
caused.

Observed failing test (there may be others): `cell::service::tests::npc_ai::
state_machine::stationary_no_los_or_range_emits_structured_decision_log`.
The failure count varies run to run (saw 1, 1, 2 across three consecutive
clean-tree runs), which is the tell that it's scheduling-dependent.

**Why:** `cargo test` runs all tests as threads in one process. Tests using
`crate::test_support::LogCapture` install a *thread-local* subscriber via
`set_default`, so events bleed across tests sharing a thread — the capture
comes back populated with events from unrelated modules and the expected
event is missing. nextest runs each test in its own process, so no bleed.
CI runs nextest, which is why this never shows up in the pipeline.

**How to apply:** Two things follow.

1. **Don't diagnose a full-`cargo test` failure as yours without checking
   the clean tree the same way.** The trap: `git stash` then run the
   *filtered* single test — it passes, and you wrongly conclude your change
   broke it. Adding any tests shifts the thread partition, so the flake
   moves. Compare like with like: stash and run the **full suite**, several
   times.
2. **Validate with nextest before reporting done.** `cargo nextest run
   --profile=ci -p cimmeria-services` is what CI gates on. Use `cargo test`
   only for fast filtered iteration on your own tests.

Related: [[db-test-revert-verification]] for the other case where the local
runner can't prove what CI proves.
