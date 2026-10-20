# CB-N06 integration (W5-INT)

Date: 2026-10-17. Branch `content/cellblock-campaign`. Feature: *Before the Patrol Returns*, mission 62060, new Cimmeria content (D-CC18, `CIMMERIA_AUTHORING`). Seal: [cb-n06-seal.md](cb-n06-seal.md). Packets: [w5-packets.md](../w5-packets.md), literals in [w5-frozen.md](../w5-frozen.md).

## Commits integrated

| Packet | Commits on the branch | Review |
|---|---|---|
| CB-N06s seal | `259127199` (from `bbdfd2fe9`) | seal |
| CB-N06m items sign-off | `fedc51359` (from `6fc43280c`) | approve |
| CB-N06-1 catalog entry | `03dc96faf` | ship (W5a) |
| CB-N06-2 dialog 60151 | `050f67f2f`, fix `f893ea0c5` | ship (W5a) |
| CB-N06-3 chains 11030-11041 | `de581adf9`, fix `a83509799` | ship (W5a) |
| CB-N06-4 resolve matrix | `0fe86243b` (from `2c3141cd3`) | ship (W5b-review1) |
| CB-N06-5 executor replay | `d12b03b5b` (from `c10160cce`) | ship (W5b-review1) |
| CB-N06-6 cooked-sync transfer | `8aacbde8a` (from `21c21c8b9`) | ship (W5b-review1) |
| CB-N06-7 services round trip | `04ea037c1` (from `2d3dd71c0`) | ship (W5b-review1) |
| Stale allows removed | `6e275de22` | - |

Sibling recheck (W5-INT step 2): no hits for 62060, 60151, 96111 on `origin/main`, `content/castle-campaign`, `content/harset-campaign` or `content/tollana-campaign` under `crates` or `db/resources`; no chain VALUES `(110[34][0-9],` on any of them. The same grep finds 62060 on this branch, so the search works.

## Tests per packet

- CB-N06-4, `crates/cell-content/src/cell/content/chain_replay_tests/mission_62060/resolve.rs`: `live_db_mission_62060_offer_gates`, `_accept_is_exclusive`, `_follow_up`, `_one_branch_per_click`, `_panel_stock_chains_disjoint`, `_restores_and_withdrawals`, `_no_dialog_path_moves_the_mission`.
- CB-N06-5, `mission_62060/executor.rs`: `live_db_mission_62060_replay_offer_accept_shows_the_choice_archetype_1`, `_archetype_8`, `_replay_aid_grants_once`, `_replay_sabotage_grants_once`, `_replay_abandon_and_641`, `_replay_relog_keeps_the_outcome`.
- CB-N06-6, `crates/base-session/src/base/cooked_sync/tests/resync_cellblock_62060.rs`: `prisoner_choice_mission_reaches_a_stock_client`, `prisoner_choice_dialog_reaches_a_stock_client`.
- CB-N06-7, `crates/services/src/mission_62060_round_trip_tests.rs`: `live_db_mission_62060_round_trip_aid_then_panel_grants_two_once`, `_far_click_refused`, `_relog_keeps_the_outcome_per_character`.
- CB-N06-1 to -3: the catalog, dialog and chain tests in the cell-content, content-engine and resources crates, including `dialog_button_linter` and the chain linters.

## Revert proofs

- CB-N06-4: deleting 11033's `mission_status 62060 not_active` row makes the "62060 active" refusal pass the gate; `accept_is_exclusive` and `one_branch_per_click` fail (review, W5b).
- CB-N06-5: removing 11038's step condition makes `sabotage_grants_once` fail at "no second grant".
- CB-N06-6: flipping one byte of the expected transfer makes the comparison fail; removing 62060 from `MISSION_ADDITIONS` or 60151 from the overrides fails the production-side asserts.
- CB-N06-7: removing 11037's `step_status` condition row makes tests 1 and 3 fail on a second grant or second row. The post-restore rerun was refused by the lane disk guard; the integrated tree is rerun in W5-INT.
- Seed: changing 11032's trigger `once` to `false` fails `rows_load_as_sealed` and `once_chains_match_the_seeded_set` (CB-N06-3 proof).

## Proven statically versus only by CB-U12

Proven by the tests above: the chain rows and their gates, the exclusive accept, one branch per click, the executor replay of the choice, the cooked-sync transfer of mission 62060 and dialog 60151, and the grant (item 2893 x2, once) with its persistence through the services round trip against a live database.

Only CB-U12 (owner, native client) proves: the client renders the journal entry and dialog 60151, first-press feedback on both branches, the two Health Slappacks arrive, and relog in the real client.

## Follow-ups

- `resolve.rs` is 620 lines (soft cap 500), accepted. Cheapest cut later: move `panel_stock_chains_disjoint`, `restores_and_withdrawals` and `no_dialog_path_moves_the_mission` to a `resolve/` sibling.
- `panel_stock_chains_disjoint`: its 1017 and 1028 halves cannot fail on an InteractTag event. Kept, but it is not protection against that.
- Chain 11036 is never run through the executor. Covered only by the resolve matrix and the seed rows.
- `relog_keeps_the_outcome_per_character` (services) does not prove isolation: player B is only seeded, never staged or clicked. Strengthen if wanted (review W5b-review1 item 1).
- Coders used `LANE_MIN_FREE_GB=0` (and 5 on one run) while B: was low. Disk is now about 63 GB free.
- Report line counts are stale: `executor.rs` is 458 lines, not the figure in the CB-N06-5 report.
- From W5a-INT: `crates/content-engine/tests/it/interact_tag_linter.rs` is 755 lines, over the 700 hard cap. Pre-existing, not split here.
- The stock client, the lab and the owner rows are not run by this packet. See [owner-lab-checklist.md](../owner-lab-checklist.md#cb-u12--before-the-patrol-returns-in-the-native-client-after-w5-int).
