---
name: lab-record-tap-fixture-and-recorder-seams
description: lab-record tap-row fixtures need a cell msg_id and method_index for decode_call; the recorder's test seams (Recorder struct, permit-checking fake probe)
metadata:
  type: project
---

Tap-row fixtures for the lab recorder must look like real cell calls:
`decode_call` (crates/lab/src/uat/record/decode.rs) reads a row by
`method_index`, and any `msg_id` >= 0xC0 is a base message (`Call::Other`).
A fixture with `msg_id: 200` and no `method_index` names nothing; use
`msg_id: 0x80, method_index: 0` for `setTargetID` (LR-07 fixed the budget
tests this way).

**Why:** LR-07 switched `named_target` and the live anchor count to
`decode_call`; four sampler budget tests broke on name-only fixtures.

**How to apply:** when writing tap rows for any record/golden test, copy a
row from `crates/wireclient/tests/fixtures/praxis_start_tap.json` or give
both fields. To test the record tools offline, inject through
`server/record/run.rs`'s `Recorder` (fake `ServerInvoker`, a `ClientProbe`
whose reads call `lease::permit::ensure` like `Supervisor::bridge_call`, an
atomic clock, a 10 ms tick); see `server/record/fakes.rs`. Related:
[[lab-daemon-lease-gate]].
