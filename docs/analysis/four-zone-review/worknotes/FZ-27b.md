# FZ-27b worknote

## Done
- Praxis offer (6511/6512) stands down while mission 1664 is on step 5226, and returns after the debrief. Guard rows added to db/resources/Content/Seed/harset_opcore_chains.sql.
- Tests: harset_1664_lifecycle.rs and mod.rs updated; new mission_1664_debrief_flow.rs; mission_1361 disjointness.rs and handoff.rs updated.
- Revert proofs: dropping the 5226 rows fails the Marsh disjointness and debrief-flow offer tests. Dropping the 6511 mission_completed 1664 row fails the handoff trigger-count assert first, then the debrief-flow test.

## Left
- Nothing in code. Committed and merged in `77f4188b7`; nothing is pushed.

## Next step
- Review the commit on four-zone-review/FZ-27-1664-debrief.

## Last check
- status=ok (cargo fmt, clippy, and the test set are recorded in the packet report)
