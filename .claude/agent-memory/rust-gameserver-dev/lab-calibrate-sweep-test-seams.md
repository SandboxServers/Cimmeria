---
name: lab-calibrate-sweep-test-seams
description: lab_uat_calibrate (SV-10) test seams - runner Fake/FakeServer are private to runner, use a geometry fake; paused tokio time makes each stand_at cost one 250 ms poll
metadata:
  type: reference
---

- `uat::runner::tests::Fake` and `runner::test_server::FakeServer` are `pub(super)`: code outside `runner` (e.g. `uat/calibrate/drive_tests.rs`) cannot use them. Write a small fake there; a geometry fake (`/gmgotoxyz` via the chat macro moves the player, hover answers by where it stands) proves re-stands better than canned queues.
- Under `#[tokio::test(start_paused = true)]` with `no_settle: true`, `Runner::stand_at` costs exactly one `STAND_POLL` (250 ms) of virtual time when the player arrives at once; camera and hover calls cost none. Budget tests lean on that.
- The lab crate is a binary: a `pub use` re-export nobody outside tests uses fails clippy (`unused_imports`); use `pub(super)` items and import them in the test by path.
- Runner items for other uat modules are re-exported from `runner/mod.rs` as `pub(crate) use players::Who` / `targets::ResolvedTag`.

Related: [[lab-uat-runner-in-process-tool-calls]], [[lab-daemon-lease-gate]].
