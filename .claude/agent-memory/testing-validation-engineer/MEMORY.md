# Memory Index

- [workflow_revert_audit.md](workflow_revert_audit.md) — Per-PR revert+run audit workflow: Edit prod code to revert, run test, confirm failure shape, Edit to restore (NOT git stash)
- [feedback_revert_to_verify_regression_guards.md](feedback_revert_to_verify_regression_guards.md) — Always revert the fix locally + rerun the test before accepting a regression-guard claim; theatre is invisible without this step
- [finding_external_junction.md](finding_external_junction.md) — PowerShell `New-Item -ItemType Junction` to link `external/` into fresh audit worktrees
- [finding_self_skipping_asset_tests.md](finding_self_skipping_asset_tests.md) — Cooked-asset-dependent tests silently pass when assets absent — flag as CI coverage gap
- [project_dispatcher_layering_gotcha.md](project_dispatcher_layering_gotcha.md) — Tests calling inner submodule dispatch bypass outer router; check arg count (5 vs 6 args)
- [project_social_arm_shadow.md](project_social_arm_shadow.md) — social.rs has a SPEND_APPLIED_SCIENCE_POINTS arm shadowing crafting's — bool routing assertions are blind to this
- [reference_logcapture_helper.md](reference_logcapture_helper.md) — `crate::test_support::LogCapture` for asserting which tracing event fired; required for routing tests where bool returns are ambiguous
- [finding_livedb_self_skip_masks_revert_verify.md](finding_livedb_self_skip_masks_revert_verify.md) — require_db_or_skip! self-skips (still prints "ok") on connect failure — a live-DB revert-verify can silently run a skipped test; grep -i skip first
