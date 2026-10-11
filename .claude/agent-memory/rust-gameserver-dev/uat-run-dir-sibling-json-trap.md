---
name: uat-run-dir-sibling-json-trap
description: RunDir::rows parses every *.json in rows/<section>/ as RowEvidence; a new per-row file there must be skipped or every run reader breaks
metadata:
  type: project
---

`crates/lab/src/uat/evidence.rs` `RunDir::rows` reads every `*.json` directly
under `rows/<section>/` as a `RowEvidence`. FX-10 (2026-10-11) put
`<row>.snapshot.json` there and had to skip the `.snapshot.json` suffix; any
new per-row file beside the row JSON needs the same, or the ledger render,
`lab_uat_attest` and `lab_uat_report` all fail on a parse error. Per-row
attachments belong in `attach_dir` (`rows/<section>/<row>/`) instead.

**Why:** row ids never contain `.` (`path_safe`), so a suffix filter is safe.
**How to apply:** when adding row-level output files, prefer `attach_dir`;
if the spec puts a file beside the row JSON, extend the skip in `rows()`.

Also: `lab_uat_attest` re-grades from the row record with `grade()` only, so
anything the runner applies after `grade` (FX-10 `apply_parity`) must be
re-applied there or an attestation silently drops it. See [[uat-runner-grade-neutral-capture]].
