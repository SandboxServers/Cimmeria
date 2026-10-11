---
name: uat-runner-grade-neutral-capture
description: UAT runner grade reasons name step action indices; any capture that records actions mid-row must defer them or the reasons change
metadata:
  type: project
---

`grade()` (crates/lab/src/uat/grade.rs) writes "step action {index} (...) failed",
and `ActionRecord.index` is `ctx.actions.len()` when the action ran. Anything
that pushes a record before or between steps (a tap start, a snapshot) moves
every later step's index, so the reasons differ even though the verdict does not.

GD-06 fingerprint mode (runner/fingerprint.rs) keeps its server calls on
`FpCapture.actions` and appends them, re-indexed, in `fp_write` at row end. It
also never sets `${player_entity_id}` (a clause could read it) and never
late-binds a row with a packet clause (its clauses grade the tap).

**Why:** the coordinator's rule for capture modes is "grade unchanged"; the
`fingerprint_mode_does_not_change_the_grade` guard fails on in-line recording.

**How to apply:** for any new opt-in capture in the runner (chaos, record),
record its actions the same deferred way and compare result + reasons + step
actions with the mode on and off. See [[lab-uat-runner-in-process-tool-calls]].
