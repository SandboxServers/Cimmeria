# ORG-05 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [org-02.md](org-02.md).

## Contract

- **Packet:** ORG-05, creation and the registrar NPCs.
- **Decisions in force:** D-ORG04 (ORG-LOCK, the org → `sgw_player` lock order, pending creations are TTL state), D-ORG10 and D-ORG23 (name rules), D-ORG13 (GM commands, access re-read on the base), D-ORG15 (free creation through constants that are 0), D-ORG18 (one Team and one Command per player).
- **Coordinator notes applied:** duplicate names must not burn org ids (pre-check before the id is drawn, plus the 3-attempt cap); the cost debit is a plain `UPDATE sgw_player` after `create_org`, never `SELECT ... FOR UPDATE`; hub templates 330 / 331 and spawns 430 / 431 with a live-DB seed guard; no edits to `gap-analysis.md`, `project-status.md` or test counts; a minimal header and roster push that ORG-06 can later unify; `destroy_client_entities` and `client_ready/` untouched.
- **Audit rows read:** A-02, A-09, A-14, A-17, A-20, A-43; CAT-M-03.
- **Base:** `origin/main` @ `cdbd5ce88` (ORG-02 #881), rebased onto `2076e482` (SS-U3 #934) before coding, and onto `38296335f` (#938, after SS-C4 #937 and CR-11 #909) before the push (see "Commands run"). Branch `org/05-creation`, worktree `.claude/worktrees/org-05`, test DB `sgw_org_05`.
- **ORG-06's push:** ORG-06 (#941) merged before the final rebase, so the founder's state goes through its `push_org_state` (see "Design decisions" 6). An earlier draft of this packet had its own minimal push, `creation::push::founder_push`; it was removed in the swap.

## Owned paths

- Wire: `crates/wire/src/cell/messages/{org_cell_to_base,org_base_to_cell}.rs` (new variants), `crates/wire/src/cell/client_methods/player.rs` (134 `Result` / `RetCode` constants).
- Base: `crates/base-session/src/base/organization/creation/` (new: `mod.rs`, `handler.rs`, `push.rs`, `telemetry.rs`, `tests/{mod,found,handlers,push,seams}.rs`), `organization/mod.rs` (one `pub mod`), `organization/persistence/mod.rs` (`insert_org` pre-checks), `crates/base-world-entry/src/base/world_entry/cell_dispatch/org_dispatch.rs` (three arms) and its `tests_dispatch_arms/org_arms.rs`.
- Cell: `crates/cell-world/src/cell/org_creation/` (new), `cell/mod.rs`, `space_manager/mod.rs` (one field); `crates/cell-interactions/src/cell/interactions/org_registrar.rs` (new), `interactions/mod.rs`, `dispatch/interact.rs` (the call beside `try_open_dhd`, and the too-far line); `crates/cell-methods/src/cell/cell_methods/organization/creation/` (new), `organization/mod.rs`, `player/social.rs` (the CM 94 arm), `player/interaction/interact.rs` (the outer range gate's too-far line), `player/interaction/{registrar_dispatch_tests,debug_hub_dispatch_tests}.rs`, `organization/tests/org_creation.rs`; `crates/cell/src/cell/service/base_messages/{org,lifecycle}.rs` (two arms, one disconnect call) and `tests/org.rs`.
- Console: `crates/cell-console/src/cell/console/org_create.rs`, `registry/commands/org_create.rs` (new), `console/{mod,dispatch}.rs`, `cell/mod.rs`, `tests/org05_org_create.rs`.
- Seeds: `db/resources/Entities/Seed/entity_templates.sql` (330, 331), `db/resources/Worlds/Seed/spawnlist.sql` (430, 431); guard `crates/cell-catalog/src/cell/spawner/tests/live_db_debug_registrars.rs`.
- Docs: `docs/gameplay/organization-system.md` (Creation section, status rows), `docs/content/debug-hub.md`, `docs/commands.md`, `docs/protocol/client-method-dispatch-table.md` (134 / 135 note), `docs/architecture/observability.md` (`org` row), this worknote.

## Evidence

- The type is not on the wire: CM 94 is `onOrganizationCreation(WSTRING)` (`SGWPlayer.def:877-880`, audit A-09), so the server must remember the dialog it opened.
- `EInteractionType.OrganizationCreation = 9` and `INT_Organization = 64` (`enumerations.xml:854,873`). The 2009 server named the registrars' interaction set maps `INTERACTION_OrganizationRegisterTeam = 7447` and `INTERACTION_OrganizationRegisterCommand = 7448` (`deprecated/python/common/Constants.py:47-48`). No seeded template carried either before this packet (`only_the_hub_templates_are_registrars`).
- Moniker 29068 `DN_npc_reg_OmegaSite_TeamCommandRegistrar` ("Organization Registrar") ships in the client's text strings (`db/resources/Texts/Seed/texts.sql`).
- No `Result` / `RetCode` text exists in the client (audit A-14, ORG-E1 Q4). The values are project policy.
- The org id sequence is `NO CYCLE`, and `INSERT ... ON CONFLICT DO NOTHING` draws `nextval` before the conflict check, so ORG-02's `create_org` drew an id for every taken name and for every second-of-type founder (the latter failed only at `insert_member`). Reproduced by the regression proof: the sequence moved from 1 to 2 on a refused name.
- The outer interact dispatcher (`cell_methods/player/interaction/interact.rs`) is the real range gate; `handle_interact`'s own too-far branch sits behind it. Both now answer a registrar click from out of range, as both answer a Banker's.

## Design decisions

1. **Registrar recognition from seed data only:** `INT_Organization` plus exactly one of the two legacy interaction sets in `static_interaction_sets`. This needs no schema column and no `SpawnRecord` field (so none of the ~15 fixture sites a new field would touch), and names the type the way the original data did. Both halves are required; a template with both sets is not a registrar. `spawn.rs` warns that `static_interaction_sets` is not kept in step with the bits elsewhere in the seed, which is why the bit is also required.
2. **The pending creation lives on the cell** (`SpaceManager::org_creations`), as the packet places it, keyed by `player_id`, with the registrar, the space, a 5-minute TTL and 3 attempts. The eligibility check is the base's (the cell does not know a character's Teams and Commands), so the registrar click is a round trip: `RegistrarOpen` → `RegistrarEligible`; then the cell records the offer and sends 135. The base's creation answer comes back as `CreateResult`, so the cell can close the offer or charge the attempt. The contract's `Create` variant is unchanged; ORG-05 adds `RegistrarOpen` and `GmCreate` (cell → base) and `RegistrarEligible` and `CreateResult` (base → cell).
3. **Attempt budget and rate limiting.** A refused name (the cell's text check or any base refusal) costs one attempt. When the three are spent the entry stays until its TTL, and a registrar click in that window is refused `rate_limited` instead of refilling it, so a player gets at most three names per five minutes. Only one name may be in flight at a time (`creation_in_flight`), which also stops one player racing two creates.
4. **No refused creation draws an org id.** `insert_org` now checks, before the insert, that the founder is not already in an organization of the type and that the name key is free. Two transaction-scoped advisory locks, the founder's (`-0x4F524701`, `player_id`) and then the name's (`-0x4F524702`, `hashtext(type:name_key)`), serialise concurrent creations so neither check goes stale. The class keys are negative so they never meet the inventory's `(player_id, container)` keys. Only creation takes them, in one order and before any row lock, so they add no ORG-LOCK cycle. `found_organization_at_cost` also refuses a short purse before `create_org`. The insert keeps its `ON CONFLICT` guard for anything that bypasses the locks (an invite accepted in between).
5. **The D-ORG15 debit** is `UPDATE sgw_player SET naquadah = naquadah - cost WHERE ... AND naquadah >= cost RETURNING naquadah` after `create_org`, in the same transaction (org row first, then `sgw_player`, no `FOR UPDATE`). A cost of 0 skips it entirely. The code path is tested at a non-zero cost through `found_organization_at_cost`; a paid creation also sends `onCashChanged` (75). There is no base-side naquadah cache to refresh.
6. **Founder push:** 134 `(1, 0)` first, so the naming dialog resolves whatever follows; then ORG-06's `push_org_state(ctx, org_id, founder, new_member = true)`, which sends one reliable bundle (35 as Leader, 43, 45, 48, 44, 49, 50 for renamed ranks only, 38, and 37 with the founder's entity id, which is what marks them online per ORG-E1 Q1) and logs `org.state_push`; then `onCashChanged` (75) when a cost was paid; then the line. The bundle's bytes are pinned by ORG-06's `org_state_messages` tests; this packet's handler test pins 134 first, the line last and the `org.state_push` row (`new_member = true`, roster 1, online 1). A failed push is ORG-06's WARN `org.state_push_failed`; the organization is committed and fills in at the next login.
7. **Refusal feedback** is 134 `(0, RetCode)` plus a line on the feedback channel, never `onErrorCode` alone. The registrar's eligibility and too-far refusals send a line only (there is no dialog to answer).
8. **GM `.org_create`** goes through `GmCreate`. The console checks the cell's access level, and the base re-reads it from its own session before acting (D-ORG13). It bypasses the registrar and the offer, and nothing else.
9. **Telemetry** follows the packet line: `org.registrar_open` and `org.create` spans on both sides; exactly one INFO outcome row per action, written by whichever side decides it; `pending_creation_created` / `_consumed` / `_expired` transitions (plus `_attempt_charged`); `org_actions_total{action, outcome, reason}` on both sides (a cell-world and a base-session helper, since the base does not depend on the cell crates, as for squads). Reasons beyond the packet's list, added because the path exists: `rate_limited`, `creation_in_flight`, `not_ready`, `actor_mismatch`, `db_error`, `base_unreachable`, `cell_unreachable`, `usage`, `not_gm`. The full list is in the observability.md `org` row.

## Commands run

All from the worktree root through the lane (`target=B:\targets/org-05`). Every live-DB run reloads `sgw_org_05` from `db/database.sql`; no live-DB test self-skipped (the tier sets `DATABASE_URL`).

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo check -p <crate> --all-targets` for base-session, base-world-entry, cell-interactions, cell-methods, cell, cell-console, cell-catalog | 0 | |
| `live-db-test.sh organization::creation` | 0 | 12 passed (first run) |
| `lane.sh cargo test -p cimmeria-cell-methods org_creation` | 0 | 11 passed |
| `lane.sh cargo test -p cimmeria-cell-console org05` | 0 | 2 passed |
| `lane.sh cargo test -p cimmeria-cell creation_arms` | 0 | 1 passed |
| `lane.sh cargo test -p cimmeria-base-world-entry org_arms` | 0 | 3 passed |
| `lane.sh cargo test -p cimmeria-cell-world -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-wire --lib` | 101 | 2 unrelated `LogCapture` tests failed under threaded `cargo test` (`gate_travel::dial_gate_without_a_gate_region_travels_immediately`, `trainer::outcome_split_player_missing_vs_no_archetype`); both pass under nextest, below |
| `lane.sh cargo nextest run -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-cell-world -p cimmeria-wire -p cimmeria-cell -p cimmeria-base-world-entry -p cimmeria-base-session -p cimmeria-base --lib` | 0 | 2872 passed |
| `live-db-test.sh organization debug_ registrar spawnlist castle_seed npc_spawn` | 0 | 233 passed (after updating `debug_hub_dispatch_tests` for ten hub NPCs) |
| `live-db-test.sh "::"` (whole live-DB tier, before the final rebase) | 0 | 4971 passed, 0 skipped |
| after `git rebase origin/main` (onto `38296335f`, #938; one conflict each in `debug_hub_dispatch_tests.rs` and `debug-hub.md`, both kept both sides): `lane.sh cargo fmt --all -- --check` | 0 | |
| `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-cell-world -p cimmeria-cell-interactions -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-cell -p cimmeria-cell-catalog --all-targets -- -D warnings` | 0 | after replacing one `chunks_exact(4)` with `as_chunks::<4>()` |
| `lane.sh cargo nextest run` (the nine crates above plus `cimmeria-base` and `cimmeria-server`) `--lib --bins` | 0 | 3111 passed |
| `live-db-test.sh "::"` (whole live-DB tier, after the rebase) | 0 | 5033 passed, 0 skipped |
| rebase onto `58aaff010` (ORG-06 #941; conflicts in `organization/mod.rs`, `org_dispatch.rs`, `org_arms.rs`, `org_base_to_cell.rs`, the cell `base_messages/org.rs` and its tests, the console registry and dispatch, three docs; all kept both sides) and the push swap | | see the rows below |
| after the swap: `lane.sh cargo clippy` (same nine crates, `--all-targets -- -D warnings`) and `cargo fmt --all` | 0 | |
| after the swap: `lane.sh cargo nextest run` (eleven crates) `--lib --bins` | 0 | 3130 passed |
| after the swap: `live-db-test.sh "::"` | 0 | 5052 passed, 0 skipped |

## Regression proof

Each mutation applied alone to the committed tree, the named test run, then the file restored with `git checkout HEAD -- <file>` and touched (the seed proof reloads the database on the next run). Every run failed as expected:

| Mutation | Test | Exit | Failure |
|---|---|---|---|
| A. `insert_org`'s `name_taken` early return removed | `create_duplicate_name_costs_nothing` (live) | 100 | "a taken name must not draw an organization id": sequence `(2, true)` vs `(1, true)` |
| B. `insert_org`'s `in_type` early return removed | `create_rejects_a_second_organization_of_the_type` (live) | 100 | "a second Team must not draw an organization id" |
| C. the short-purse pre-check disabled (`if false && ...`) | `create_without_the_cost_founds_nothing` (live) | 100 | "a short purse must not draw an organization id" |
| D. `begin_attempt` returns `Ok(Team)` with no offer | `create_rejects_without_pending_creation` | 101 | assertion at `org_creation.rs:118` (a `Create` reached the base) |
| E. the cell's `org_text::validate` replaced by the raw name | `org_creation::create_rejects_invalid_names` | 101 | assertion at `org_creation.rs:173` (invalid names reached the base) |
| I. the in-flight check disabled | `create_forwards_the_offer_type_and_blocks_a_second_name` | 101 | two `Create`s forwarded |
| G. the `try_open_org_registrar` call disabled in `handle_interact` | `registrar_click_in_range_asks_the_base_with_the_seeded_type` | 101 | no `RegistrarOpen` |
| J. the registrar too-far call removed from the outer range gate | `registrar_click_out_of_range_asks_nothing_and_says_why` | 101 | no feedback line |
| H. the base's GM access check disabled | `gm_create_rechecks_the_access_level_on_the_base` (live) | 100 | a player founded an organization |
| F. template rows 330 / 331 and spawn rows 430 / 431 deleted from the seed | `live_db_debug_registrars` (3 tests, live) | 100 | all three FAILED |
| K. the `push_org_state` call removed from `push_to_founder` (after the swap) | `create_pushes_the_new_organization_and_tells_the_cell` (live) | 100 | assertion at `handlers.rs:160` (no state bundle between 134 and the line) |

## Test catalogue

- `cimmeria-base-session` `organization::creation::tests` (sentinels `0x7000_4E00..=0x7000_4EFF`, blocks of 16, 11 of 16 used; moved off `0x7000_4C00`, which ORG-06 takes with `0x7000_4D00`):
  - `found::create_rejects_invalid_names`, `found::create_duplicate_name_costs_nothing`, `found::create_rejects_a_second_organization_of_the_type`, `found::create_without_the_cost_founds_nothing`, `found::create_debits_in_the_same_transaction_and_free_touches_nothing` (type 3);
  - `handlers::create_pushes_the_new_organization_and_tells_the_cell`, `handlers::create_duplicate_name_is_refused_visibly`, `handlers::registrar_open_checks_eligibility_per_type`, `handlers::gm_create_rechecks_the_access_level_on_the_base`, `handlers::create_for_a_recycled_entity_acts_for_nobody` (types 3, 8 and 12, against `TestTransport`);
  - `seams::a_failing_database_client_and_cell_each_warn` (with the `org_actions_total` count), `seams::a_failing_eligibility_read_warns_and_opens_nothing` (type 12, no database).
- `cimmeria-cell-world` `org_creation::tests`: 8 state-machine unit tests with exact instants.
- `cimmeria-cell-interactions` `org_registrar::tests`: `the_type_needs_the_bit_and_exactly_one_registrar_set`, `registrar_sets_are_the_legacy_interaction_set_maps`.
- `cimmeria-cell-methods` `organization::tests::org_creation`: `registrar_eligible_opens_the_dialog_and_records_the_offer`, `create_rejects_without_pending_creation`, `create_forwards_the_offer_type_and_blocks_a_second_name`, `create_rejects_invalid_names`, `create_result_charges_or_closes_the_offer`, `an_offer_from_another_space_or_too_old_is_expired`, `eligible_for_a_recycled_entity_opens_nothing`, `reopening_an_exhausted_offer_is_rate_limited`, `disconnect_drops_the_offer`, `a_closed_base_channel_warns_and_charges_the_attempt`; `player::interaction::registrar_dispatch_tests` (4, including the closed-channel seam); `player::social::tests::org_creation_routes_to_the_creation_handler` (replaces the ORG-01 "unimplemented" test); `debug_hub_dispatch_tests` extended to the ten hub NPCs.
- `cimmeria-cell-catalog` `live_db_debug_registrars`: `registrar_templates_carry_their_role_fields`, `only_the_hub_templates_are_registrars`, `registrar_spawns_sit_in_the_stasis_room_hub`.
- `cimmeria-cell` `base_messages::tests::org::creation_arms_reach_the_creation_handlers`; `cimmeria-base-world-entry` `org_arms::creation_variants_reach_the_creation_handlers`; `cimmeria-cell-console` `org05_org_create` (2); `cimmeria-wire` `cm134_result_and_ret_codes_are_pinned`.

The CAT-M-03 names from the ledger exist as written: `create_rejects_without_pending_creation` (cell), `create_rejects_invalid_names` (cell and base, the base one the live-DB half), `create_duplicate_name_costs_nothing` (base, live).

## SigNoz filters

Logs, `service.name = 'cimmeria-server'`:

| What | Filter |
|---|---|
| Everything a player did at a registrar | `scope_name = 'org' AND event IN ('org.registrar_open', 'org.create') AND player_id = <id>`, by time |
| Why a creation was refused | `scope_name = 'org' AND event = 'org.create' AND outcome = 'rejected'`, grouped by `reason` |
| Founded organizations | `event = 'org.create' AND outcome = 'ok'` (`org_id`, `org_type`, `name_units`, `cost_before` / `cost_after` when paid) |
| Registrar clicks refused | `event = 'org.registrar_open' AND outcome = 'rejected'` (`not_eligible`, `too_far` with `distance`, `rate_limited`) |
| An offer's life | `event IN ('pending_creation_created', 'pending_creation_attempt_charged', 'pending_creation_consumed', 'pending_creation_expired') AND player_id = <id>` |
| GM creations | `event = 'org.gm_action' AND action = 'gm_org_create'` |
| Broken plumbing | `scope_name = 'org' AND severity_text = 'WARN' AND event IN ('org.actor_mismatch', 'org.send_failed', 'org.cell_send_failed', 'org.create_forward_failed', 'org.registrar_forward_failed', 'org.create_failed', 'org.registrar_open_lookup_failed', 'org.state_push_failed')` |

Metrics: `org_actions_total` by `action` (`registrar_open`, `create`, `gm_org_create`), `outcome` and `reason`.

## Known gaps

- **Client behaviour is unverified.** Whether the client sends `interact` for an `INT_Organization` NPC, and what `CreateTeamWin` / `CreateCommandWin` do on 134 `(0, n)` (stay open or close), was not checked in the client. ORG-UAT step 5 covers it; the feedback line is sent either way.
- The eligibility check at the registrar is a display read, so a player invited into a Team between the click and the name gets the dialog and then `already_in_org_type` at creation. The creation transaction is the authority.
- An invite accepted into a Team between `insert_org`'s pre-check and its insert (ORG-07's `add_member` does not take the creation advisory locks) still draws an id before `AlreadyInType`. Rare, and a typed refusal.
- A player who never answers the dialog keeps an offer until the TTL or logout; it holds no lock and no database state.
- Registrar placement is unchecked in the client, like the rest of the hub.

## Integration edits for the coordinator

- **Contract (work-packets.md § Messages):** record the new variants `OrgCellToBase::{RegistrarOpen, GmCreate}` and `OrgBaseToCell::{RegistrarEligible, CreateResult}`, and that the pending creation lives on the cell (`SpaceManager::org_creations`) while the base keeps none.
- **Contract (Models):** registrar recognition is `INT_Organization` plus static interaction set 7447 (Team) / 7448 (Command); the 134 `Result` / `RetCode` values are in `org_creation_ret_code`.
- **ORG-06:** done in this branch: creation calls `push_org_state` after 134.
- **ORG-07:** `add_member` does not take the creation advisory locks. That is fine for correctness (the `ON CONFLICT` guard stays), but an invite into a Team racing a creation may draw an id.
- **ORG-11:** `docs/gap-analysis.md` §23 and `docs/project-status.md` need the Creation row (not edited here, per the rules).
