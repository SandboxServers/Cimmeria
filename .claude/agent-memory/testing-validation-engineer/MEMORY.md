# Memory Index

- [workflow_revert_audit.md](workflow_revert_audit.md) — Per-PR revert+run audit workflow: Edit prod code to revert, run test, confirm failure shape, Edit to restore (NOT git stash)
- [feedback_revert_to_verify_regression_guards.md](feedback_revert_to_verify_regression_guards.md) — Always revert the fix locally + rerun the test before accepting a regression-guard claim; theatre is invisible without this step
- [finding_external_junction.md](finding_external_junction.md) — PowerShell `New-Item -ItemType Junction` to link `external/` into fresh audit worktrees
- [finding_self_skipping_asset_tests.md](finding_self_skipping_asset_tests.md) — Cooked-asset-dependent tests silently pass when assets absent — flag as CI coverage gap
- [project_dispatcher_layering_gotcha.md](project_dispatcher_layering_gotcha.md) — Tests calling inner submodule dispatch bypass outer router; check arg count (5 vs 6 args)
- [project_social_arm_shadow.md](project_social_arm_shadow.md) — social.rs has a SPEND_APPLIED_SCIENCE_POINTS arm shadowing crafting's — bool routing assertions are blind to this
- [reference_logcapture_helper.md](reference_logcapture_helper.md) — `crate::test_support::LogCapture` for asserting which tracing event fired; required for routing tests where bool returns are ambiguous
- [finding_seed_null_masks_livedb_assertion.md](finding_seed_null_masks_livedb_assertion.md) — "forced to None regardless of the row" passes for free when the picked seed row is already NULL; filter the fixture row or drop the assert
- [finding_livedb_self_skip_masks_revert_verify.md](finding_livedb_self_skip_masks_revert_verify.md) — require_db_or_skip! self-skips (still prints "ok") on connect failure — a live-DB revert-verify can silently run a skipped test; grep -i skip first
- [project_legacy_command_parity_review_pattern.md](project_legacy_command_parity_review_pattern.md) — full-workspace build/clippy is the coordinator's gate, not the worker's (workers may still run crate-scoped fmt/clippy — verify, don't assume "Not run"); acceptance lines often list multiple response classes, only some tested
- [Seed SQL line-scan loses rows](finding_seed_sql_line_scan_loses_rows.md) — Dialogs seeds have multi-line literals, chain seeds have apostrophes in `--` comments; pin parsed row counts
- [finding_sentinel_and_method_idx_sweeps.md](finding_sentinel_and_method_idx_sweeps.md) — 2026-09-25: 9 dup sentinel consts (4 real) + decimal-offset spill; method_idx vs .def flattening has 0 mismatches; names.rs is a conformance hook
- [project_live_db_marker_convention.md](project_live_db_marker_convention.md) — live-DB tests need `live_db` in fn/module name; N-way parallel on per-slot DB clones; guards in test-support live_db_group
- [finding_tolua_native_arity_stubs.md](finding_tolua_native_arity_stubs.md) — SGW Lua natives are tolua shims that raise on extra args (0x00403280=isnoobj); permissive UAT stubs hid #1213 getAbilityList(2)
- [finding_source_scan_lexer_masking.md](finding_source_scan_lexer_masking.md) — NT-03 scan masks literals/comments before parsing (load-bearing); Git Bash `sed -i` turns CRLF docs LF
- [finding_station_bucket_hides_intra_zone_assist.md](finding_station_bucket_hides_intra_zone_assist.md) — tag-prefix station buckets skip intra-zone assist pairs; NID Guard 24 assist 26 u; seed y != runtime y (navmesh snap)
