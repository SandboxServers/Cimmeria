# BV-10a Worknotes

> Type: reference. Audience: the Bank and Vault coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [bv-04.md](bv-04.md), [bv-07.md](bv-07.md), [the debug hub doc](../../../content/debug-hub.md).

## Contract

- **Packet:** BV-10a, Team and Command Banker NPCs in the stasis-room debug hub, so the org vaults (BV-07) can be tested in game. Split out of BV-10 by the coordinator; the UAT steps below are drafts for BV-10 to put in the ledger.
- **Decisions in force:** D-BV09 (`INT_Banker` plus `entity_templates.vault_scope`), D-BV12 (every member may open; deposit by default, withdraw opt-in), D-BV13 (no disband while the vault holds items), D-BV14 (Team 40, Command 100), D-BV19 (telemetry), and the organizations ledger's D-ORG21 (default ranks hold `DepositBank`, only the Leader `WithdrawBank`).
- **Base:** first cut on `origin/main` @ `13643442d` (BV-07b #949); rebased onto `fe49730b1` (#923 docs-regen, #956 unified UAT guide, #958) before the push. Branch `bank/bv10a-org-bankers`, worktree `.claude/worktrees/bank-bv10a`, test database `sgw_bank_bv10a`.
- **Owned paths:**
  - seeds: `db/resources/Entities/Seed/entity_templates.sql` (templates 371 and 372), `db/resources/Worlds/Seed/spawnlist.sql` (spawns 471 and 472)
  - `crates/cell-catalog/src/cell/spawner/tests/{live_db_debug_org_bankers.rs,mod.rs}`
  - `crates/cell-methods/src/cell/cell_methods/player/interaction/debug_hub_dispatch_tests.rs` (count and click rows)
  - `crates/cell-world/src/cell/spawner_tests/live_db_vault_scope.rs` (one test's pinned list)
  - `crates/services/src/{bank_org_round_trip_tests.rs,lib.rs}`
  - docs: `docs/content/debug-hub.md`, `docs/guides/unified-uat.md` (the hub table and the bank status line), this file
  - agent memory: `.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`

## Read set

[bv-04.md](bv-04.md) and its seed rows (template 370, spawn 470) and guards (`live_db_debug_banker.rs`, `debug_hub_banker_opens_the_personal_vault`); [bv-07.md](bv-07.md) (the open round trip and the telemetry catalog); `docs/content/debug-hub.md`; `.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`; every world-12 spawn and its template (bodies); the ORG-05 registrar rows 330/331 and 430/431; SS-U3's mail clerk 390/490; `texts.sql` for banker monikers; `org_vault/open.rs`, `org_vault/tests/{mod.rs,open.rs}`, `bank/org_open.rs`, the cell's `base_messages/bank.rs`, `base-world-entry` `cell_dispatch/{mod.rs,bank_dispatch.rs}`; the organizations handlers (`kick.rs`, `disband.rs`, `telemetry.rs`), `docs/commands.md` (the `.org_*` rows), `docs/gameplay/inventory-system.md` (the org move reasons), `crates/services/src/{mission_round_trip_tests.rs,gate_round_trip_tests/}`; the ledger's [UAT checklist](../handoffs/session-resume.md#uat-checklist).

## Evidence

- **Room geometry.** Region1 corners A(-347.17, -230.14), B(-327.67, -240.70), C(-318.89, -224.51), D(-338.39, -213.94); respawner 8 at (-334.23, -228.03). In wall coordinates (`u` along B to C, `v` in from the B-C wall), the B-C wall line (`v` = 3) holds 450 (`u` 3.0, `v` 2.1), 430 (6.1), 470 (9.2) and 490 (13.0). The only slot left on it is `u` 16, 2.4 from the C-D wall. 431 is the one NPC already in a second row (`u` 6.1, `v` 6.0). The A-B line is full; crafting holds the D-A wall.
- **Name.** No `DN_npc_` moniker names a Team or Command banker. The only Bank or Storage NPC monikers are 29462 `DN_npc_OmegaSite_Banker` ('Storage Officer') and 29463 `DN_npc_Harset_Banker` ('Storage Lotaur'); the Team/Command moniker 29068 is the registrar's. A new moniker cannot render.
- **The `name` column never reaches the client.** The spawner loaders select `name_id`, not `name` (`cell-catalog` `spawner/npcs.rs:36`, `templates.rs:80`). A distinct `name` is for the seed reader only; bodies are what tell the Bankers apart in game.
- **A kick sends the client nothing to close the vault window.** `OrgMembershipEnded` ends the cell's session (`vault_session_closed reason=org_left`, `cell-world` `vault_session_end.rs`) and the organizations handler sends `onOrganizationLeft`, but no bank method goes to the client. Whether the client's vault window closes by itself on `onOrganizationLeft` is unknown. The next drag is refused either way (no session).
- **Nothing in production sets `bound = true`.** `sgw_inventory.bound` defaults false and no grant, vendor or equip path writes it (a grep of `crates` finds only tests). A bound-item UAT step needs a database edit on a local server.

## Design decisions

- **Templates 371 and 372** copy 370's role columns: `interaction_type = 2` (`INT_Banker`) only, `vault_scope` `team` / `command` written explicitly, faction 1, class `mob`, event set 570, level 1, no ability set, trainer list, vendor list, loot table, chain or dialog, `name_id` 29462. `name` is 'Team Banker' / 'Command Banker'.
- **Bodies**, both already spawned in world 12, both human male so neither is mistaken for the Storage Officer (a woman in SGC uniform):
  - 371: template 15's SGC Cellblock Guard uniform (spawn 20), without its pistol. Template 330 (the Team registrar) wears a similar SGC uniform, but its head differs (`BS_HM_Head_04` there, `BS_HM_Head_01` here) and it stands two slots away.
  - 372: template 314's plain crew look (`CraftHub_Supplies`, spawn 414), on the other side of the room.
- **Placement: a second row, staggered.** The wall line has one slot left, 2.4 units from the C-D wall, and two Bankers need two. Following ORG-05's precedent (431 is a second-row NPC), both stand 6 units in from the B-C wall, each in the gap between two wall NPCs so that none stands in front of another as seen from the room centre, where a player clicks from:
  - 471 at `u` 11.1, `v` 6.0: (-327.65, 73.472, -228.09), heading -1.4294.
  - 472 at `u` 14.9, `v` 6.0: (-325.84, 73.472, -224.74), heading -1.9148.
  - Headings face the room centre (-333.03, -227.32), yaw = atan2(dx, dz), as the other hub NPCs.
- **Clearances (XZ).** 471: 3.55 to 470, 3.55 to 490, 3.8 to 472, 5.0 to 431, 5.8 to 430, 6.6 to the respawner, 8.2 to 404; 11.1 from the A-B wall. 472: 3.55 to 490, 3.8 to 471, 6.4 to 470, 8.8 to 431, 9.0 to the respawner; 3.5 from the C-D wall. Every `CraftHub_*` spawn is 10+ away (they stand at `v` 19.2). The seed guards check 2.5 to every other spawn in the room and 5.0 to the respawner. Both new spawns move no other spawn's clearance below its own guard: 470 and 490 are 3.55 from the nearest new one.
- **Tags** `DebugHub_TeamBanker` and `DebugHub_CommandBanker`, so the hub guards count them: `staged_hub` goes from 10 to 12.
- **The end-to-end smoke is in `cimmeria-services`.** The org open is a round trip. The cell half is in `cimmeria-cell-methods` / `cimmeria-cell-interactions` and the base half in `cimmeria-base-world-entry`, and neither track can depend on the other. So a smoke that proves "a member gets 107/108, a non-member the refusal" from the real seed rows has to be in the facade, beside the gate and mission round trips. BV-04's `debug_hub_banker_opens_the_personal_vault` harness (in `cell-methods`) is reused for the cell half only: the two new click rows in `debug_hub_npcs_answer_a_click_with_their_own_interaction` pin the `OrgVaultOpen` each Banker sends.
- **The round trip routes through the production bridges.** The cell's `OrgVaultOpen` goes into the base's real `handle_cell_message` (so the `Bank` arm and `bank_dispatch::route` run), and the base's `OrgVaultGranted` is fed to `grant_org_vault` with the arguments `CellService`'s `base_messages::bank::handle` passes (that fn is `pub(super)` in `cimmeria-cell`).
- **`seeded_spawns_load_vault_scope` (BV-02) is updated, not deleted.** It pinned "no seeded template sets a non-personal vault_scope yet", which these rows make false. It now pins exactly `[(471, Team), (472, Command)]`, a stronger form of the same guard: a loader that stops selecting the column loads both as `Personal` and fails.

## Telemetry (D-BV19)

This packet adds no event, span, target or filter row: the Bankers exercise BV-07's catalog. Every row a click on 471 or 472 produces carries `account_id`, `player_id`, `entity_id` and `banker_id` (see [bv-07.md § Telemetry](bv-07.md#telemetry-d-bv19)). The round-trip smoke pins one of them on the real seed rows: `org_vault_open_rejected` WARN with `reason=not_in_org` and `banker_id` equal to the clicked Banker (`debug_hub_org_bankers_open_the_members_vault_and_refuse_others`).

## Commands run

All from the worktree root. `live-db-test.sh` reloads `sgw_bank_bv10a` from `db/database.sql` first.

| Command | Exit | Result |
|---|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-services -p cimmeria-cell-methods -p cimmeria-cell-catalog --all-targets` | 0 | clean |
| `bash tools/build-lane/live-db-test.sh -E 'test(/debug_hub\|debug_banker\|org_banker\|mail_clerk\|registrar/)'` | 0 | 44 passed, 0 skipped among them |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | 0 | clean |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-cell-catalog -p cimmeria-cell-methods -p cimmeria-services --all-targets -- -D warnings` | 0 | clean |
| `bash tools/build-lane/live-db-test.sh ::` (first cut) | 100 | 5333 run, 5332 passed, 1 failed: `cell-world` `live_db_vault_scope::seeded_spawns_load_vault_scope`, which pinned "no non-personal scope yet" (see Design); updated |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings` | 0 | clean |
| `bash tools/build-lane/live-db-test.sh -E 'test(/live_db_vault_scope/)'` | 0 | 4 passed |
| **After the rebase onto `fe49730b1`:** fmt check, clippy on `cimmeria-cell-catalog`, `cimmeria-cell-methods`, `cimmeria-cell-world`, `cimmeria-services`, then `bash tools/build-lane/live-db-test.sh ::` | 0 | fmt and clippy clean; 5333 passed, 0 skipped, against `sgw_bank_bv10a` |

## Tests added or changed

| Test | Type | Guards |
|---|---|---|
| `cimmeria-cell-catalog spawner::tests::live_db_debug_org_bankers::org_banker_templates_carry_their_role_fields` | live-DB seed guard | 371 and 372: exactly `INT_BANKER`, `vault_scope` `team` / `command`, class `mob`, no pet flag, name 29462 renders, no trainer list, vendor lists, loot table or ability set, not faction 10; the trainer-list loader maps neither |
| `...live_db_debug_org_bankers::org_banker_spawns_sit_in_the_stasis_room_hub` | live-DB seed guard | 471 and 472 are the only placements of their templates, with their tags, in world 12, loading `INT_BANKER` with scope `Team` / `Command`, inside Region1, on the floor, 5+ from the respawner, 2.5+ from every other spawn in the room (470 must be among them), no respawn, not stationary, no aggression override |
| `cimmeria-services bank_org_round_trip_tests::debug_hub_org_bankers_open_the_members_vault_and_refuse_others` | live-DB smoke (cross-track) | from the real seed rows, with the real content engine: each Banker spawns as its scope; a member gets exactly one `OrgVaultOpen`, one `OrgVaultGranted` and one client method (107 Team, 108 Command) and a session pinned to the Banker naming the org; a character in no org gets no grant, no method, no session, one WARN `org_vault_open_rejected reason=not_in_org` with the `banker_id`, and the line; a Team-only character opens the Team vault and is refused by the Command Banker |
| `cimmeria-cell-methods ...debug_hub_dispatch_tests::debug_hub_npcs_answer_a_click_with_their_own_interaction` (extended) and `staged_hub` (count 10 to 12) | live-DB smoke | each org Banker's click sends one `OrgVaultOpen` with its scope, its own entity id and the player id, and no client method |
| `cimmeria-cell-world spawner_tests::live_db_vault_scope::seeded_spawns_load_vault_scope` (changed) | live-DB | exactly 471 `Team` and 472 `Command` load a non-personal scope |

Sentinels: account `0x7000_BA00`, players `0x7000_BA01`-`0x7000_BA03`, entity `0x7000_BA20`, organizations named `Bv10a Team`, `Bv10a Command`, `Bv10a Solo` and cleaned up by exact name key, port 40910. The `0x7000_BA` block was unused before this packet.

## Regression proof

Code committed first (`8ba7ab648`, pre-rebase). A scratch script (`revert.py`, not committed) applied each revert set; `git checkout HEAD -- db` plus `touch` restored the seeds; `git status` was clean after each.

| Revert set | Command | Result |
|---|---|---|
| `rows`: the 371, 372, 471 and 472 `INSERT` rows removed | `bash tools/build-lane/live-db-test.sh --no-fail-fast -E 'test(/org_banker\|debug_hub_npcs\|bank_org_round_trip\|debug_banker/)'` | exit 100; 9 run, 4 FAIL: both `live_db_debug_org_bankers` tests, `debug_hub_npcs_answer_a_click_with_their_own_interaction` (the count), `debug_hub_org_bankers_open_the_members_vault_and_refuse_others`. BV-04's two `live_db_debug_banker` tests and BV-07's three cell tests still pass |
| `scope`: 372's `vault_scope` set to `'team'` | same | exit 100; the same 4 FAIL (the template guard on the column, the spawn guard on the loaded scope, the click row on the `OrgVaultOpen` scope, the round trip on the spawned interaction) |

The `seeded_spawns_load_vault_scope` change was proven by the first full run: before the edit it failed on these seed rows; it now passes with them and would fail on the `rows` revert (the list would be empty).

## Known gaps

- **No distinct names.** All three Bankers show "Storage Officer": no shipped moniker fits and a new one cannot render. The bodies differ. A tester who clicks the wrong one gets a line saying they are in no Team (or Command), or the personal vault.
- **Placement is unchecked in the client**, like the rest of the hub (no navmesh or occluder data for the room). Check that 471 and 472 stand clear of the stasis pods and do not block the walk to 470 and 490.
- **A kick does not visibly close the vault window** (see Evidence). The server ends the session, and the next drag is refused with a line. UAT step 20 below checks what the client shows.
- **No in-game way to make a bound item** (see Evidence). UAT step 19 needs a database edit, so it is for a local server, not the colo.
- **No Squad Banker.** Squad vaults (18) are not built; no template has `vault_scope` squad (the CHECK does not allow it).
- **Files near the caps:** `debug_hub_dispatch_tests.rs` is 545 lines (was 521). Its tests share `staged_hub` and `click`; a split would move the org Banker rows and the Banker test into a sibling with those helpers made `pub(super)`. Left as is.

## Draft UAT steps for the ledger (org vaults)

These continue the [UAT checklist](../handoffs/session-resume.md#uat-checklist) at 15. Two characters, A (a GM) and B, both online. Filter SigNoz by `target=bank` (or `target=org`) and the tester's `player_id`. The events are [bv-07.md § Telemetry](bv-07.md#telemetry-d-bv19)'s. The org vault success rows are DEBUG (`bank=debug` in `OTEL_FILTER`).

Setup: A types `.org_create team Vault Testers` (A leads it), `.org_info` to read the org id and rank 2's mask, then `.org_join <orgId> <B>` (B joins at Team Member, rank 2). For the Command steps, the same with `.org_create command ...`; B joins at Initiate.

15. **Open as a member.** A right-clicks the Team Banker (spawn 471, "Storage Officer" in the Cellblock guard uniform, second row off the B-C wall). The Team vault opens on the first click, 40 slots. Repeat at the Command Banker (472, plain crew clothes): the Command vault opens, 100 slots.
    - SigNoz: `target=bank event=org_vault_open_requested scope=team` (cell, with `banker_id`, `space_id`, `distance` at most 5), then `target=bank event=org_vault_opened org_type=team rank=8 can_deposit=true can_withdraw=true vault_slots=40` (span `bank.org_vault_open`), then `target=bank event=vault_session_opened scope=team` with `org_id` (span `bank.org_vault_grant`). The Command run shows `scope=command org_type=command`.
16. **Open as a non-member.** A character in no Team right-clicks the Team Banker. Nothing opens; chat says "You are not in a Team, so there is no Team vault to open." The same at the Command Banker says "Command".
    - SigNoz: `target=bank event=org_vault_open_rejected reason=not_in_org scope=team` (WARN), with `banker_id`. No `org_vault_opened` and no `vault_session_opened` follow.
17. **Deposit.** B opens the Team vault and drags a stackable item (a crafting supply from the hub vendor) from a bag into a vault slot. It stays; close and reopen, still there.
    - SigNoz: `target=bank event=org_move_accepted direction=deposit perm=DepositBank org_type=team rank=2`, with `item_id`, `type_id`, `quantity`, `target_container_id=19`, `target_slot_id` below `vault_slots`, and the stack sizes before and after, in the span `bank.org_move_item`.
18. **Withdraw without and with `WithdrawBank`.** B drags the item back to a bag. It is refused with a line and snaps back into the vault. A then types `.org_set_perms <orgId> 2 <mask>` with 0x20000 added to the mask from `.org_info`. B drags again: the item lands in the bag.
    - SigNoz: first `target=bank event=org_move_rejected reason=missing_permission perm=WithdrawBank` (WARN) with `vault_end=source`; then `target=org event=org.gm_action action=gm_org_set_perms outcome=ok` with the `org_id` and `rank`; then `target=bank event=org_move_accepted direction=withdraw perm=WithdrawBank source_container_id=19`, with the same `item_id` as step 17.
19. **A bound item is refused (local server only).** Nothing in game binds an item, so on a local server set one of B's carried items bound: `UPDATE sgw_inventory SET bound = true WHERE item_id = <id>`, then relog B. B drags it into the Team vault: refused with a line, and it stays in the bag.
    - SigNoz: `target=bank event=org_move_rejected reason=bound_item_not_org_storable vault_end=target`, with the `item_id` and `type_id`.
20. **Kick closes the session.** With B's Team vault window open, A removes B from the Team (the Team window's roster). Record whether B's vault window closes. Either way, B's next drag in that window is refused with a line and snaps back.
    - SigNoz: `target=org event=org.kick outcome=ok` and `target=org event=member_left reason=kicked`, then `target=bank event=vault_session_closed reason=org_left scope=team` with B's `player_id` and the `org_id`. B's drag: `target=bank event=org_move_rejected` with `reason=no_vault_session` (the session is gone).
21. **Disband is blocked while the vault holds items.** With an item in the Team vault, A types `.org_disband <orgId>`. It is refused with a line; `.org_list` still shows the Team. A withdraws the item and disbands again: it succeeds.
    - SigNoz: `target=org event=org.disband outcome=rejected reason=vault_not_empty`, then after the withdrawal (`target=bank event=org_move_accepted direction=withdraw`) `target=org event=org.disband outcome=ok`.
22. **A second online member sees the move.** A and B both in the Team, both with the Team vault open at the Team Banker. A deposits an item: it appears in B's window without B reopening. A withdraws it: it leaves B's window.
    - SigNoz: `target=bank event=org_move_accepted` for A, then `target=bank event=org_vault_fanout` with `updated_recipients=1` for the deposit and `removed_ids=1 removed_recipients=1` for the withdrawal. A `target=bank event=org_move_resync_failed` row means the fan-out did not go out.

## Integration edits for the coordinator

- **work-packets.md:** BV-10a status to Review. BV-10's UAT extension can take steps 15-22 above.
- **UAT checklist** (`handoffs/session-resume.md`): replace "The org-vault steps are added by BV-10." with steps 15-22. Carry the kick-window and bound-item gaps into "Known gaps".
- **Unified UAT guide:** this PR adds the two Bankers to its hub table and points the bank status line at the debug hub doc. The org vault steps themselves are still BV-10's.
- **BV-08 (org cash):** no overlap. This packet touches no cash code and no org handler.
