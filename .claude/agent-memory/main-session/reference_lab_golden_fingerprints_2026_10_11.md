---
name: reference-lab-golden-fingerprints
description: Lab golden fingerprint rules (lab-golden, code-complete 2026-10-11) - offline from the bundle, tap names, entity labels, key-arg allowlist, five passing runs agree, diff under the golden's variance, rebless needs fresh runs, the grade-neutral capture trap
metadata:
  type: reference
---

Lab golden runs (campaign `docs/analysis/lab-golden/`, code merged by
2026-10-11; GD-10, the first golden, waits on lab-spec-vocab SV-13 and SV-15).
Guide: `docs/guides/automated-uat.md#golden-runs`.

- **Offline from the bundle.** Fingerprint mode (`lab_uat_run fingerprint:
  true`) only captures: a tap on every in-world row, entity snapshots
  (`entities.json`) and window samples (`windows.json`). `uat::golden`
  computes the fingerprint from the run folder by pure code, so an old
  bundle can be re-fingerprinted (D-GD1).
- **Tap names, not wireclient.** Events use the tap's `msg_name` and
  `decoded` JSON (D-GD2); the lab crate never depends on
  `cimmeria-wireclient`.
- **Entity labels.** `self`, `p2`, `player`; NPCs `tag:`, else `spawn:`,
  else `tpl:` from two snapshots (bind and finish); unknown ids `?<n>` by
  first appearance per row; `0` is `none`. Run id and character names become
  `${run_id}`, `${character}`, `${p2_character}`.
- **Key-arg allowlist** (`golden/key_args.rs`). Listed messages keep listed
  fields only; anything else compares by name and target. Five inbound calls
  decode raw bytes (interact 74, dialogButtonChoice 75, moveItem 38,
  gmGotoXYZ 163, triggerClientHintedGenericRegion 85). `avatarUpdate*`,
  `perfStats`, `requestEntityUpdate` are always ignored.
- **Agreement.** At least five runs launched by `lab golden record` itself,
  every row PASS in every run, same spec hash (equal to the spec file's
  current hash), server build and SGW.exe hash, fingerprints exactly equal,
  no tap fault or `unavailable` tap (D-GD6, D-GD8).
- **Diff under the golden's variance.** The golden stores each row's resolved
  variance; `diff_run_dir` fingerprints the run under it, so a spec variance
  edit changes nothing until a re-bless (D-GD5). Divergence exits 5, never
  changes a grade (D-GD7).
- **Rebless needs fresh runs.** A run id already in the old golden's
  evidence, or dropping a row the old golden held, is `cannot_record`.
- **Grade-neutral capture trap.** Fingerprint mode's own server calls (tap
  start, entity get/query) must not shift step indices in reasons: they go
  to `actions[]` as setup only when the row ends, re-indexed. A row with a
  packet clause keeps `tap_start` and is never late-bound; the capture never
  sets `${player_entity_id}`. Any new capture call must follow the same rule
  or golden mode changes grades.
- Calling `lab_timeline` on the lab character while recording drains the tap
  and makes the runs unrecordable.

Related: [[reference-first-session-lab-calibration]].
