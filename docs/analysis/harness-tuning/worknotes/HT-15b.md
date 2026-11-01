# HT-15b worknote

Folders: `.claude/agent-memory/main-session/` and the 18 orphan folders the lint reported.

## Lint (whole-repo summary line)

- Before: `memory_lint files=777 no_type=68 state_no_invalidated=127 index_over=1 index_missing=1 unindexed=203 orphan_folders=18 ip_like=4`
- After review pass 1: `memory_lint files=766 no_type=32 state_no_invalidated=119 index_over=0 index_missing=0 unindexed=199 orphan_folders=0 ip_like=1`
- No flag is left for `main-session/` or any orphan folder. The remaining counts belong to other folders (HT-15a, HT-15c).

## Counts for main-session (review pass 1)

Pass 1 settled all 48 files that carried `checked: unverified` (the 45 flagged at the base plus the 3 folded files the packet created). Each was checked with at most 3 Grep or Read calls at the worktree HEAD.

- (a) verified, stamped `checked: 2026-11-01`: 27.
- (b) rewritten as method, or trailing stamp lines removed from a method or trap file: 19.
- (c) deleted: 2.
- (d) left unverified: 0.

Pass 2 found two (a)-stamped files that were still false (reference_custom_debug_map, project_four_zone_planning_2026_10_13) and corrected both; 9 of the 27 (a) stamps were spot-checked. N4 then swept the seven named status-word files and corrected stale status sentences in reference_worldforge_shell_2026_10_06, reference_custom_map_first_client_result_2026_10_06 and reference_cellblock_praxis_start_2026_10_13. Pass 3 found three more stale status sentences in stamped files (reference_ancient_asset_inventory_2026_10_06, reference_castle_letter_bridge_2026_10_14, project_cellblock_aftermath_dwell_2026_10_13) and corrected them. Ruling 1 traced the status sentences in five more stamped files (reference_dev_workbench_causal_trace_2026_10_16, reference_dev_workbench_dossier_2026_10_15, reference_dev_workbench_slash_3a_2026_10_16, reference_launcher_embedded_payload, reference_owner_lock_release_2026_10_04): 5 held, 0 rewritten, 0 marked checked: unverified. Three stamped files with no status words (reference_dev_workbench_progression_metadata_2026_10_16, reference_dev_workbench_slash_2b_2026_10_16, reference_janus_worldforge_design_2026_10_06) were accepted as is.

Deleted in pass 1 (with their index lines): `project_cellblock_autoplay_campaign.md` (N2: contradicted by `crates/lab/src/supervisor/world/` and the D-AP1 VERIFIED row) and `project_lab_uat_reliability_2026_10_13.md` (the ledger `docs/analysis/lab-uat-reliability/README.md` carries its status).

False claims found: `project_enemy_combat_runtime_blockers` blockers 3 (`EF_DONT_USE_QR` is 16) and 4 (DoT kill goes through `dot_kill_credit`) were fixed and are removed; blockers 1 and 2 still hold. `project_launcher_summary_observability` said the exporter is inert in every build; `summary.rs` now resolves an endpoint, so that bullet was rewritten. `project_harset_continuation` was a stale progress narrative and is now a pointer. `reference_launcher_desktop_native_traps` lost two point-in-time sentences (updater Disabled, egui source gone).

Other fixes: N5 16 placeholder descriptions replaced; N6 two duplicate index pairs and N7 the dangling updater-handoff link removed (index now 165 lines); N8 ip-like prose reworded; `reference_lab_uat_capture_ids` path corrected to `uat/runner/drive.rs`. `git grep -c 'checked: unverified'` for main-session returns nothing.

## Orphans

All 18 orphan folders were removed. Their content was folded into three new main-session files:
`reference_launcher_desktop_native_traps.md`, `reference_wine_guest_app_identity.md`, `reference_lab_route_and_livewire_traps.md` (trap files; only reference_wine_guest_app_identity carries a stamp, checked: 2026-11-01, and it is counted in the 27).

Deleted paths (folders, 28 files): `.claude/agent-memory/{adoption-ui,cellblock_route,effective-settings,launcher-game-update,launcher-launch-engine,launcher-mac-graphics-fix,launcher-migration,launcher-migration-ui,launcher-play-integration,launcher-signed-updater,launcher-update-uat,launcher-updater-minimum,livewire-atlas,native-adoption,published-adoption,updater-apply,updater-handoff-fix,wine-computer-use}/`.

## Index

`main-session/MEMORY.md`: 165 lines, one line per file, every link target resolves.

## Next

Nothing left in these folders for pass 1.

Ruling 1 lint: files=766 no_type=32 state_no_invalidated=119 index_over=0 index_missing=0 unindexed=199 orphan_folders=0 ip_like=1; no main-session flag. Unit tests OK.
