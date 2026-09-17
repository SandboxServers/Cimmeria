---
name: project_legacy_command_parity_review_pattern
description: Recurring review checklist for legacy-command-parity work packets (P01+) — workers skip fmt/clippy by protocol, and acceptance criteria often list multiple response classes where only one gets tested
metadata:
  type: project
---

The "legacy-command-parity" campaign (docs/analysis/legacy-command-parity/) runs each
work packet (P01, P02, ...) in its own worktree/branch, with a coordinator integrating
one packet at a time onto `legacy-command-parity`. Workers explicitly skip
`cargo fmt --check`/`clippy`/full workspace build per the campaign protocol (README.md's
"coordinator's serialized pre-PR gate") — their handoffs say "Not run" for these.

**Why this matters for review:** "not run by design" does not mean "known-clean." P01's
handoff (docs/analysis/legacy-command-parity/handoffs/p01.md) claimed complete regression
proof and passing tests, but `cargo fmt --all -- --check` still found real diffs in both
new test files. Always run fmt + clippy yourself during review even when the worker's
protocol says the coordinator will gate it later — catching it at packet-review time is
cheaper than at final-integration time.

**How to apply:** For every future packet in this campaign (P02+), independently:
1. Revert each of the worker's claimed fix points and confirm the cited test(s) fail with
   the cited message — see [[feedback_revert_to_verify_regression_guards]] and
   [[workflow_revert_audit]] for the general procedure.
2. Run `cargo fmt --all -- --check` and `cargo clippy -p cimmeria-services --lib --tests -- -D warnings`
   against the worktree yourself, even though the worker's handoff says these are
   "Not run" by protocol.
3. Cross-check the packet's `work-packets.md` **Acceptance** line against the actual test
   file — acceptance lines often list multiple response classes (e.g. "empty/error
   responses") where the worker's tests cover only one (e.g. empty, not error/no-DB). This
   isn't necessarily a blocker if the untested code path is pre-existing and untouched by
   the packet, but it's worth flagging explicitly rather than assuming full coverage from
   the presence of a live-DB test file.
4. Verify sentinel base non-collision by grepping the crate for neighbouring `0x7000_xxxx`
   reservations — packets self-report their neighbour but don't always grep to confirm.
