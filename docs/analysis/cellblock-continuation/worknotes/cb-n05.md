# CB-N05 integration notes (W3-INT)

> **Status:** Integrated (W3-INT, 2026-10-17). UATPending on owner-run CB-U09 and CB-U10.
> **Seal:** [cb-n05-seal.md](cb-n05-seal.md). **Packets:** [w3-packets.md](../w3-packets.md).

## Commits integrated

| Packet | Landed as (`content/cellblock-campaign`) | Source commit |
|---|---|---|
| CB-N05-1 mission catalog | 5b63bed37 | dc921f274 |
| CB-N05-2 dialog 60150 | b62ee7cca | 84f7958af |
| CB-N05-3 chains 11000-11005 | 390b724ef | 5836675d5 |
| CB-N05-3f linter allowlist | c5ee47b31 | e9e931749 |
| W3a-INT fix (once-chain set) | 12fcfb5db | (W3a-INT-fix) |
| CB-N05-4 cooked-sync transfer | deb648154 | 3c3d61933 |
| CB-N05-5 dispatch negatives | 90bf4c125 | fcaa879eb |
| CB-N05-5 fix (step roster) | cd1a8db93 | 8f4f2dcf1 |
| CB-N05-6 persistence | 9da88c183 | 656e6fafc |
| CB-N05-7 executor replay | e3d5935ff | de443957a |

Review verdict for CB-N05-4, -5 (after fix), -6 and -7: ship (W3b-review1, W3b-review2).

## Tests per packet

- CB-N05-4: `remaining_pods_mission_reaches_a_stock_client`, `remaining_pods_dialog_reaches_a_stock_client` (no DB).
- CB-N05-5: seven `live_db_mission_62050_dispatch_*` tests (in-range accept, far click, wrong and missing entity, cross-space, forged dialog choice, stale initial response, Marsh far click).
- CB-N05-6: `live_db_mission_62050_progress_survives_relog_per_character`, `live_db_mission_62050_abandon_then_reaccept`.
- CB-N05-7: five `live_db_mission_62050_replay_*` tests (offer, accept and report; Marsh beside archetypes 1 and 8; duplicate and skip; abandon; relog restores offer once).
- W3a-INT fix: `live_db_once_chains_match_the_seeded_set`.

## Revert proofs (recorded by each packet, checked in review)

- CB-N05-4: removing 62050 from `MISSION_ADDITIONS` fails `heads.len()==1`; removing the 60150 override fails `stale.remove`; a one-byte edit to the EXPECTED literal fails the body comparison.
- CB-N05-5 fix: injecting `complete_objective(62055)` passes the old test and fails the fixed one at `mission_62050_tests.rs:339`.
- CB-N05-6: removing the 62050 `mission_steps` rows fails `reload-db` (seed required, no assertion revert).
- CB-N05-7: removing the 11004 `remove_dialog_set` fails the slot-17 assertion near `executor.rs:262`.
- W3a-INT fix: removing 11010 from the expected set fails with `left` holding six IDs and `right` five.

## Proven statically and by live-DB tests versus owner-only

- Proven here: catalog row and dialog bytes against the frozen literals (w3-frozen.md); range, cross-space, entity, forged-choice and stale-response gates; per-character persistence and relog; executor replay of 11000-11005 including `once`; seed linters; `reload-db` with 361 content chains.
- Only CB-U10 can prove (owner, real client): the journal renders 62050, dialog 60150 renders, no crash, first-press feedback, advance and relog, and the core Marsh briefing is unchanged.
- Only CB-U09 can prove: pacing of the 10-minute beat.

## Live-DB results at integration

- `live-db-test.ps1 mission_62050`: 22 passed, 8099 skipped.
- `live-db-test.ps1 chain_replay_tests`: 577 passed, 7544 skipped.

## Follow-ups

From W3a-INT notes:
- CB-N05-3 added `chain_replay_tests/mission_62050/rows.rs` (255 lines), outside its Writes list. Accepted by review 1 and the architect as a file-cap split of `resolve.rs`.
- `crates/resources/src/base/resources/tests/dialog_overrides.rs:88`: message says "the two mission-622 corpses" (double space) after 60150 was added. Cosmetic.
- `mission_62050/mod.rs` has a ragged doc-comment wrap. Cosmetic.
- `crates/content-engine/tests/it/interact_tag_linter.rs` is 754 lines, over the 700 cap from before this campaign (CB-N05-3f added one line by exception). Issue #97's entity-template walker or a per-player-bind exemption would shrink the allowlist.
- A live `cimmeria-server` on `sgw_zone_cellblock` blocks the live-DB clone. Close it before live-DB runs.

From W3-INT:
- CB-N05-7 Marsh click at step A in `duplicate_and_skip` cannot detect removal of chain 11005's `step_status 62054` gate (low). Step A holds only 62052 active, so a missing gate still leaves 62055 uncompleted. Staging 62055 as active too would detect it.
- CB-N05-6 "player B has no row" assertions cannot fail (low). Nothing writes to B. A second manager at another step, asserting A's row is unchanged, would bite.
- Measurement sections: W3b review 1 lists reports 4, 5 and 6 as lacking `## Measurement` (the brief named 4 and 6; the review is the source for 5 too). Report 7 has one without wall-clock times. Low, checked against the review text, not the reports.
- CB-N05-7 spec test 3, third bullet: the test never binds first, so it proves only that nothing binds. A binding by chain 11000 before 641 is accepted survives until relog (indicator only, the click is refused). Not in F4; architect to decide.
- CB-N05-4 coder ran `cargo fmt -p`; W3-INT ran `cargo fmt --all -- --check` (clean).
