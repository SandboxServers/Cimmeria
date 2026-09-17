---
name: project_legacy_command_parity_review_pattern
description: Recurring review checklist for legacy-command-parity work packets (P01+) — full-workspace build/clippy/nextest is the coordinator's serialized gate, not the worker's, and acceptance criteria often list multiple response classes where only one gets tested
metadata:
  type: project
---

The "legacy-command-parity" campaign (docs/analysis/legacy-command-parity/) runs each
work packet (P01, P02, ...) in its own worktree/branch, with a coordinator integrating
one packet at a time onto `legacy-command-parity`. The *full workspace* `cargo build`/
`cargo nextest run`/`cargo clippy --workspace` pass is the coordinator's serialized
pre-PR gate (README.md), not something every worker runs — but workers are not barred
from running crate-scoped `cargo fmt --all -- --check` / `cargo clippy -p <crate>
--all-targets -- -D warnings` themselves, and P02's worker did exactly that, with its
handoff correctly reporting both as passing (not "Not run"). Don't assume every packet's
handoff will say "Not run" for these — check what the worker actually claims to have run
before treating "Not run" as the default.

**Why this matters for review:** a worker's own "ran it, it's clean" claim still needs
independent verification. P01's handoff (docs/analysis/legacy-command-parity/handoffs/p01.md)
initially claimed complete regression proof and passing tests, but `cargo fmt --all --
--check` still found real diffs in both new test files. Always run fmt + clippy yourself
during review regardless of what the worker's handoff claims — a worker-reported "clean"
result is a claim to verify, not a fact to trust, and cheaper to catch at packet-review
time than at final-integration time.

**How to apply:** For every future packet in this campaign (P02+), independently:

1. Revert each of the worker's claimed fix points and confirm the cited test(s) fail with
   the cited message — see [feedback_revert_to_verify_regression_guards](feedback_revert_to_verify_regression_guards.md)
   and [workflow_revert_audit](workflow_revert_audit.md) for the general procedure.
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
