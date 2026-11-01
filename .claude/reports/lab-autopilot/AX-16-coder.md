# AX-16 coder report

Done. `resume_from` for lab_uat_run (C10).

- New `uat/runner/resume.rs` (validate, start_index, order, skip_count) + `resume_tests.rs`.
- `RunRequest.resume_from` and `UatRunArgs.resume_from`; `run_all` validates first, then skips the leading rows (no run, no re-grade, manifest entries kept).
- Other `RunRequest` literals all use `..Default::default()`; none needed edits.
- Line counts: `uat/runner/mod.rs` 557, `server/uat.rs` 645 (+4).
- Tests: auto_starts_at_first_missing_row, auto_with_all_done_runs_nothing, explicit_row_and_unknown_row, resume_needs_run_dir_and_excludes_rows (C10 strings), a_resumed_run_skips_earlier_rows (fails if the skip in run_all is removed: row 1 would re-type `.one`). No revert proof run.
- No new tool; policy/claims lists unchanged.
- Checks: L1 clean, L2 1645 passed.
