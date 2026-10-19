# UB-02c review pass 2

Reviewer: pass 2 over coder ba46b5a21 and pass-1 fixer cf7df8999. Verdict: ship.

Pass-1 findings re-verified
- walks_to chained legs (asgard.rs): fixed. Cap 8, restart from last waypoint, needs Ok within 2 m, no-progress (<0.5 m) refuses. Revert proof rerun: MAX_WALK_LEGS=1 fails `beta_walk_helper_chains_long_routes_and_refuses_islands` at the ring-148 assertion.
- Footer revert proof: `beta_asgard_seed_files_carry_their_own_sequence_footers` rerun with the point_set_points footer altered; it fails naming the file. Restored.
- Anchors, ids 23060-23071 / sets 23030-23031 / points 23100-23101, CRLF, database.sql \ir order, templates 2305 and 2328-2334 present: re-read against the seal and spec, all match. Satterfield (23002) is 4.7 m outside FireSite horizontally.

New findings (both fixed)
- asgard.rs island control: `pocket` negative control would also pass if the pocket were off-mesh. Added `standable(&exterior, pocket, ...)`.
- asgard.rs pad test: deck pad lower bound was 1.45 m against the spec's 1.5 m (0.05 m slack). Tightened to 1.499 (pair is 1.49999 m in f32).

Checks (worktree ub02c)
- fmt: status=ok exit=0
- clippy -p cimmeria-cell-world --all-targets -D warnings: status=ok exit=0
- live-db beta_e1: status=ok, 23 passed; seeded_sequences: status=ok, 1 passed

Not changed: stack-overlap textual conflicts with 02b/02d (mod.rs, database.sql, 23199 footers) are for the merge; keep both sides.