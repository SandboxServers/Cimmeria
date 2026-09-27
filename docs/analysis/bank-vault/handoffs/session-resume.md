# Bank and Vault: Session Resume

> Type: how-to. Audience: the owner (UAT) and any later session.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: every packet merged; BV-10 (the org close-out) in review, then release 2

| Packet | Status | PR | Notes |
|---|---|---|---|
| Plan | Done | #860 | The ledger, and two doc bugs fixed (audit §3) |
| BV-E1 | Done | #867 | Client evidence |
| BV-01 | Done | #872 (`09aadf59c`) | Capacity, the allowlist, and #798 (closed) |
| BV-02 | Done | #921 (`6f00cce9d`) | The Banker open path and GM `.bank` |
| BV-03a | Done | #927 (`490fe4200`) | The pure `move_/mod.rs` split, merged ahead of BV-03 |
| BV-03 | Done | #935 (`dfb0b0f8d`) | Deposit and withdraw, and the use and removal gate |
| BV-04 | Done | #931 (`1c6bed3cc`) | The debug-hub Banker and `.bankdump` |
| BV-05 | Done | #947 (`6607fbdc2`) | Expansion. Server side and GM `.bankexpand`; the player-facing Expand dialog is quarantined (D-BV35) |
| BV-06 | Done | #950 (`3684fa7eb`) | Close-out docs and the UAT queries. Release 1 is deployed |
| BV-07a | Done | #948 (`673929c0d`) | Team and Command vault storage, the Banker open round trip, the session |
| BV-07b | Done | #949 (`13643442d`) | Moves into, out of and within 19/20, the bank bits, the vault log, the fan-out |
| BV-10a | Done | #960 (`07f27ce19`) | The Team Banker (371/471) and Command Banker (372/472) in the debug hub |
| BV-08 | Done | #963 (`acb43359c`) | Org cash: treasury deposits and withdrawals, the cash log |
| BV-09 | Done | #966 (`6d7d43149`) | Team vault expansion from the treasury, GM `.orgvaultexpand`, leader only |
| BV-10 | Review | this PR | Org close-out docs and UAT steps 15-25. The coordinator posts `/release` after it merges (release 2) |

Coordinator: session cimmeria-79 (formerly cimmeria-97), worktree `.claude/worktrees/bank-ledger`. Worker rules: `%TEMP%\cimmeria-castle\BANK-WORKER-RULES.md`.

Merge bar: green CI plus the coordinator's review. Copilot review is suspended while its spend is exhausted.

Cross-campaign agreements:

- **cimmeria-1f (organizations, formerly cimmeria-fa).**
  - The ORG-API is described in its ledger.
  - `org_vault_is_empty(tx, org_id)` and `org_vault_is_empty_sql(org_id)` shipped as stubs that returned `true`; BV-07a (#948) replaced them with the real vault and treasury check.
  - D-ORG20 (PR #857): when an org's last member deletes their character and the vault is not empty, the org is kept, memberless.
  - D-ORG21 is our D-BV12.
  - BV-07 built on ORG-02 and ORG-06; BV-08 on ORG-07's CM 19 cell forward. BV-07b added `broadcast_to_org_except` to `organization/handlers/broadcast.rs` for the fan-out, the one edit to an organizations file.
  - The lock order for vault and treasury actions (D-BV39) was approved by cimmeria-1f and is recorded in `organization/api.rs` § "Lock order".
- **cimmeria-23 (crafting, formerly cimmeria-af).** Container 15 stays movable to and from 1. CR-05 may add a post-commit notify in `move_/after_commit.rs`; cimmeria-23 messages before starting it.
- **cimmeria-3d (social, formerly cimmeria-19).**
  - Vault mail aliases (D-SS07, our D-BV29) are ours, in Wave 4 after BV-07. The seam is `resolve_recipient_flags`; until we replace it, `MAIL_ToVault` is refused.
  - Mail attachments come from bags 1 and 15 only (D-BV30); SS-M2 (#912) refuses 16 to 20.
- **cimmeria-11 (black market).** The auction container (18) is theirs. `player_movable` refuses it until their packet wires it.

## Resuming

1. Read the status lines above and in [work-packets.md](../work-packets.md).
2. For any packet in Review, check its PR's CI. If the CI is older than `main`, rebase and re-test before merging.
3. Once BV-10 merges, post `/release` on it from PowerShell (D-BV11), then hand the owner the [UAT checklist](#uat-checklist): steps 1-14 for the personal bank, 15-25 for the org vaults, the treasury and the Team vault expansion.
4. Open work after release 2 is in [Known gaps](#known-gaps-carried-forward): the vault mail aliases (D-BV29), the player-facing Expand button (#943), and the follow-ups the owner's UAT turns up. Retire the campaign's worktrees the day each PR merges.

## Lifting the #943 quarantine

Dialog 60110 ("Expand vault") is quarantined with the debug-hub dialogs until reverse engineering names the field that crashed the client (#943 is the containment PR; no issue tracks the root cause yet). Lift it only after that field is known and 60110 is shown not to carry it, or has been reshaped so it does not. Then, in one PR:

1. Move the 60110 entry from `QUARANTINED_DIALOG_OVERRIDES` to `DIALOG_OVERRIDES` in `crates/resources/src/base/dialog_overrides/mod.rs`.
2. Set `VAULT_EXPAND_DIALOG_SERVED` to `true` in `crates/wire/src/cell/vault.rs`. `the_expand_dialog_flag_matches_the_served_list` (resources) fails if only one of steps 1 and 2 is done.
3. Update `expand_tests::a_quarantined_dialog_is_recorded_but_not_shown` in `cell-interactions`. Its `const` assertion stops compiling once the flag flips, so the test becomes "an offer shows the dialog".
4. Move 60110 in the pinned id lists: `debug_hub_dialogs_are_quarantined` (in `dialog_overrides/mod.rs`), `crates/resources/src/base/resources/tests/dialog_overrides.rs`, and `HUB_DIALOGS` in `dialog_overrides/override_seed_agreement_debug_hub.rs`, which looks the definition up in `QUARANTINED_DIALOG_OVERRIDES`.
5. The category-5 version bump is hashed from `DIALOG_OVERRIDES`, so it changes by itself. Nothing evicts an id a client already cached, so a client that crashed on a bad copy still needs `Cache.en-US\CookedDataDialogs.pak` deleted (see #943).
6. UAT: rewrite steps 11 and 12 back to the Banker button, and confirm that `expand_offered` replaces `expand_offer_suppressed` in SigNoz.

Record the lift as a new decision row that supersedes D-BV35.

## Known gaps (carried forward)

- **Players cannot buy an expansion.** Dialog 60110 is quarantined (D-BV35), so a Banker offers nothing and only a GM can buy, with `.bankexpand` for the personal vault and `.orgvaultexpand` for a Team vault. See [Lifting the #943 quarantine](#lifting-the-943-quarantine).
- **Vault mail aliases are not built** (D-BV29). Every `MAIL_ToVault` bit is still refused with `reason=vault_alias_unsupported`; no packet in this ledger took the `resolve_recipient_flags` seam. It needs its own packet, which also decides what `MAIL_ToCommandRank6` (4092) means.

Org vaults, the treasury and the Team expansion (BV-07 to BV-10a):

- **A kick does not close the client's org vault window.** The server ends the session (`vault_session_closed reason=org_left`) and the organizations handler sends `onOrganizationLeft`, but no bank method goes to the client, so the window may stay open. The next drag is refused (`no_vault_session`) with a line and snaps back. UAT step 20 records what the client shows.
- **Other members' open Team windows keep the old size after an expansion** until they reopen them. They get the new treasury (`onOrganizationCashUpdate`) but no `onBagInfo`, which would have to carry each recipient's own `bank_slots` (BV-09 review N4).
- **The buyer's `onBagInfo` after a Team expansion may carry a stale personal `bank_slots`** if a `.bankexpand` by the same player commits in between. Cosmetic; the next world entry or vault open re-declares it (BV-09 review N3).
- **`price_missing` on the Team expansion is untested on purpose.** The price seed is shared by every live-DB test, so removing a row would break the shared test database. The cash log's `CHECK (amount > 0)` is the backstop against a free step, and the Rust check refuses a missing or zero price.
- **All three hub Bankers show "Storage Officer".** No shipped moniker names a Team or Command banker, and the `name` column never reaches the client. Tell them apart by body: the personal Banker is a woman in SGC uniform (470), the Team Banker wears the Cellblock guard uniform (471), and the Command Banker plain crew clothes (472).
- **The bound-item step needs a local database edit.** Nothing in production sets `bound = true`, so UAT step 19 binds an item by hand and cannot run on the colo.
- **World entry declares 19 and 20 at 100.** The org vault open and the Team purchase re-send `onBagInfo` with the Team's real size, which the client is expected to apply to the live window (BV-E1 Q2, inferred). UAT steps 15 and 24 confirm it. Changing the world-entry declaration would touch BV-05's `bank_slots` path.
- **`onClearOrgVaultInventory` (74) is unused** (D-BV40). A client that left an organization keeps its vault's rows in its cache until relog; they cannot be moved, because every move re-checks membership.
- **The fan-out goes to every online member**, not only those with the window open: the cell holds the sessions, and the rows are harmless in a closed window's cache.
- **No `ViewBankLogs` reader.** The vault log and the cash log are written for one; nothing reads them yet.
- **Org guard audit (BV-10).** Every org catalog event has a `LogCapture` guard on `main`, except the reasons the worknotes exempt because nothing can inject them without dropping the database or the channel mid-transaction:
  - BV-07: `org_vault_open_rejected reason=cell_channel_closed`; `org_move_rejected` with `move_failed`, `move_lock_begin_failed` or `move_lock_failed`; `org_move_resync_failed` (both reasons). The open's lock-time reasons (`not_a_member`, `no_such_org`, `wrong_org_type`, `player_missing`) are pinned by `lock_actor_refuses_non_members_gone_orgs_and_the_wrong_type`, not by a `LogCapture` row.
  - BV-08: `bank_feedback_send_failed` for the treasury sends (the same seam is guarded for BV-05).
  - BV-09: `expand_rejected` with `price_missing` (above), `row_changed` (unreachable under the lock), `no_such_org`, `wrong_org_type`, `player_missing` and the cell's `base_channel_closed`.
- **Files over the soft cap from Wave 4:** `move_/org/mod.rs` (520 lines) and `debug_hub_dispatch_tests.rs` (545).
- **`docs/known-issues.md` § Organizations is stale.** It still lists `organizationTransferCash` and most organization calls as stubbed. It predates the organizations campaign and is for its owner or the docs-regen owner to rewrite.

Personal bank (BV-01 to BV-06):

- **`vault_access` trusts a recycled Banker id.** It checks only that the pinned Banker entity id exists, is in the same space and is in range, not that it is still a Banker. Entity ids can be recycled, so if the Banker despawns and its id is reused by another NPC standing within 5 m, a move or an expansion passes the proximity check against that NPC. The fix is to check the entity's `NpcInteractionType::Banker` (and its scope) inside `vault_move_allowed`. It dates from BV-02 and BV-03, and the BV-05 server-authority review named it as out of scope.
- **Guard audit (BV-06).** Every catalog event and reason from BV-01 to BV-05 has a `LogCapture` guard on `main`, except these:
  - `bank_feedback_send_failed` with `reason` `no_session`, `not_in_world` or `send_error` (BV-05, `bank_expand/sends.rs`): only `no_client_address` is pinned (`a_player_with_no_client_address_logs_the_dropped_sends`, `a_recycled_entity_id_receives_nothing`).
  - `expand_offer_dropped` with `reason` `entity_missing` or `vault_scope_mismatch`: `a_stale_offer_shows_no_dialog` pins the other three reasons.
  - Pinned by a label or classification unit test, not a `LogCapture` row, as the worknotes record: the verdict pass-through labels `banker_other_space`, `vault_session_other_space` and `player_missing` on `move_rejected`, and the same labels plus `banker_gone` on `expand_rejected` (`vault_access_maps_every_verdict`); `vault_scope_mismatch` on both (no org session can open yet); `expand_rejected reason=row_changed` (a race between two statements); the `move_accepted` kinds `within` and `swap` (`move_kind` unit test).
  - Exempt because nothing can inject them (BV-01): `move_resync_skipped reason=resync_read_failed` and `move_rejected reason=move_lock_release_failed`.
- **A backpack merge has no success telemetry.** D-BV25 changed carried-bag drag behaviour (a drop on a same-type stack merges instead of swapping), but `move_accepted` is logged only for moves that touch 17. A committed carried move logs only the unstructured DEBUG "Inventory move persisted", with no `event=` field, no move kind and no stack sizes. UAT step 14 can only be checked in the client. A later packet could add a DEBUG `inventory` row for a committed merge.
- **Files over the hard cap**, pre-existing and not owned by the bank. BV-02 to BV-05 each added a few lines:
  - `crates/entity/src/cell_entity/entity_struct.rs` (775 lines);
  - `crates/cell-methods/src/cell/cell_methods/player/interaction/mod.rs` (734);
  - `crates/wire/src/cell/messages/cell_to_base.rs` (865).
- **Files over the soft cap:** `move_/container_policy.rs` (566), `move_/vault_move_tests.rs` (575), and BV-05's `cell-interactions` `bank/expand_tests.rs` (517). `move_/mod.rs` is already split (#927) and is 464 lines; `base-session` `bank_expand/mod.rs` is 473.
- **BV-01 residual.** The success-path snapshot is read after the lock is released.
- **Non-vault silent rejects are unchanged:** a split onto an occupied slot in the backpack, a bad slot outside 17, and `item_allows_container` outside the vault.
- **Live resize.** Whether an `onBagInfo` sent mid-session resizes an open vault window is inferred only (BV-E1 Q2). UAT step 11 checks it with `.bankexpand` while the window is open; the chat line acknowledges the purchase either way.
- **`price_missing` has no live-DB test**, because a seed gap would break the shared test database. The classification and the log row are pinned by unit tests.
- **Issue #928** (the buyback lock order) is not a bank issue; it is in triage.
- **The archived-mail storage loophole** (SS-M4, social). Archived mail never expires, so it can hold items and cash indefinitely. Social is asking the owner for a cap; the coordinator suggested capping archives that hold attachments or cash at 100.

## UAT checklist

Run steps 1 to 14 on the colo after release 1, and steps 15 to 25 after release 2, as a GM, with a fresh character. At anything odd, type `.bug <what you saw>`. The bookmark captures the scene.

Each step ends with the SigNoz query that verifies it. Filter the logs by the attributes shown (`target=bank event=… reason=…`) plus the tester's `player_id`. The event names and fields are the [telemetry catalog](../work-packets.md#contract-fixed-by-this-ledger). Most `bank` success rows are DEBUG, which `OTEL_FILTER` exports for this target (`bank=debug`).

### Personal bank (release 1)

1. **Declared size.** As a GM, open the vault by typing `.bank`. The window shows `bank_slots` slots, which is 40 before any expansion (BV-05), and does not scroll past them.
   - SigNoz: `target=bank event=vault_session_opened scope=personal gm_override=true`, in the span `bank.console_open`. The declared size is `target=bank event=expand_quote offered=true bank_slots=40 price=100`.
2. **Banker click.** Go to the Castle Cellblock stasis-room debug hub. Right-click the Banker, "Storage Officer" (spawn 470), in the middle of the room's B-C wall. The vault opens on the first click. No "Expand vault" dialog appears: it is quarantined (D-BV35).
   - SigNoz: `target=bank event=vault_session_opened scope=personal` with `banker_id` and `distance` (at most 5), in the span `bank.banker_interact`. The quarantine shows as `target=bank event=expand_offer_suppressed reason=dialog_quarantined`. A failed click is `target=bank event=vault_open_rejected` with its `reason`.
3. **Deposit.** Drag an item from the backpack into a vault slot. It stays there. Close the vault and reopen it: it is still there.
   - SigNoz: `target=bank event=move_accepted kind=deposit target_container_id=17`, with the `item_id` and `target_slot_id` below `bank_slots`, in the span `bank.move_item`. The reopen is a second `vault_session_opened`.
4. **Withdraw.** Drag it back to the backpack. It lands there, and the vault slot empties.
   - SigNoz: `target=bank event=move_accepted kind=withdraw source_container_id=17`, with the same `item_id` as step 3.
5. **Stacks.** Split a stack into the bank, then merge it back. The totals never change.
   - SigNoz: `target=bank event=move_accepted kind=split`, then `kind=merge`. On each row, `source_stack_before + target_stack_before` equals `source_stack_after + target_stack_after`.
6. **Relog.** Deposit an item, log out, log in, and open the vault. The item is still there.
   - SigNoz: `target=bank event=move_accepted kind=deposit`, then `target=bank event=vault_session_closed reason=logout` with `open_ms`, then a new `vault_session_opened`. To check the row itself, run `.bankdump <name>`: `target=bank event=gm_action action=bankdump result=ok` with `item_count`.
7. **Walk away.** Open the vault at the Banker, walk more than about 5 m away, then try to drag. The move is refused with a visible message, and the item snaps back.
   - SigNoz: `target=bank event=move_rejected reason=banker_out_of_range` with `vault_end` (`target` for a deposit, `source` for a withdrawal), `banker_id` and `distance` above 5. The snap-back sends nothing to SigNoz; a `target=bank event=move_resync_skipped` row for that `item_id` means it did not happen.
8. **Mission item.** Try to bank a mission item. It is refused with a message.
   - SigNoz: `target=bank event=move_rejected reason=mission_item_not_bankable vault_end=target`, with the `item_id` and `type_id`.
9. **Trade, vendor and mail.** A banked item cannot be traded, sold or mailed. Only carried bags are offered.
   - SigNoz: there is no `bank` row, because these paths never read 17. A mail attachment from the vault is `target=mail event=mail.attachment_refused reason=item_in_vault`. A trade that names a banked item aborts: the `trade_swaps_total` counter with `outcome=ineligible_container`, and a WARN from the trade executor whose `reason` names the ineligible container. The vendor's sell list simply leaves 17 out (`VENDOR_FILTER_BAGS`), so the check there is only that the item is not listed.
10. **Buyback (#798).** Sell an item, then try to drag it out of the buyback tab without paying. It is refused.
    - SigNoz: `target=bank event=move_rejected reason=source_container_not_player_movable source_container_id=16`.
11. **Expansion (GM).** As a GM, `.bank`, then `.bankexpand`. Chat says "Your vault now has 50 slots. You paid 100 naquadah."; the cash drops by 100; if the vault window is open, its scroll range grows to 50 (BV-E1 Q2, confirm this). Repeat until 100 slots. At 100, `.bankexpand` refuses with "Your vault is already at its full size of 100 slots." and nothing changes. `.bankexpand` with no vault open refuses with "bankexpand: open your vault first (.bank, or a Banker in range). Nothing was charged." A non-GM's `.bankexpand` is refused with ".bankexpand needs GM access. Nothing was charged." and is not said aloud. The player-facing Expand button is pending #943 (D-BV35): until the dialog is served, a player at a Banker sees no Expand offer.
    - SigNoz: `target=bank event=expand trigger=gm_console gm_override=true bank_slots_before=40 bank_slots_after=50 price=100`, with `cash_before - cash_after = 100`, in the spans `bank.console_expand` (cell) and `bank.expand_purchase` (base); one `expand` row per step, up to `bank_slots_after=100`. At the ceiling: `target=bank event=expand_rejected reason=at_ceiling trigger=gm_console bank_slots=100`. With no vault open: `target=bank event=expand_rejected reason=no_vault_session trigger=gm_console`. A non-GM: `target=bank event=expand_rejected reason=not_gm trigger=gm_console`.
12. **Not enough cash (GM).** With less than 100 naquadah, `.bankexpand` refuses with "You need 100 naquadah to expand your vault. Nothing was charged." and nothing changes.
    - SigNoz: `target=bank event=expand_rejected reason=insufficient_cash trigger=gm_console`, with `cash` below `price` and `bank_slots` unchanged; no `expand` row follows.
13. **GM override.** As a GM away from any Banker, `.bank` opens the vault, and moves work. As a non-GM, `.bank` is refused.
    - SigNoz: `target=bank event=vault_session_opened gm_override=true` with no `banker_id`, then `target=bank event=move_accepted gm_override=true`. The non-GM is `target=bank event=vault_open_rejected reason=not_gm`, in the span `bank.console_open`.
14. **Backpack merge.** Drag a stack onto a same-type stack in the backpack; they merge up to the stack limit, and the total never changes. BV-03 turned the legacy merge on for every container, not only the vault (D-BV25).
    - SigNoz: by design no `bank` row records a move that does not touch 17 (`a_carried_move_logs_no_bank_event`), so an empty `target=bank` result for the drag is expected. The commit itself is the unstructured DEBUG "Inventory move persisted" (from `inventory::move_::after_commit`, with `item_id` and `total_items`), which names neither the merge nor the stack sizes, so a wrong total can only be reported with `.bug` (see Known gaps).

### Org vaults, the treasury and the Team expansion (release 2)

Steps 15 to 25 need two characters online at once: **A**, a GM, and **B**, any character. Filter SigNoz by `target=bank` (or `target=org` for the organization rows) and the tester's `player_id`. The org vault success rows are DEBUG (`bank=debug` in `OTEL_FILTER`). The Bankers and the setup are in the [debug hub doc](../../../content/debug-hub.md#team-and-command-bankers-templates-371-and-372).

Setup:

- A founds a Team with `.org_create team Vault Testers` (A leads it, and the Leader holds every bit), then a Command with `.org_create command <name>`. The registrars (spawns 430 and 431) work too.
- A types `.org_info` to read each organization's id and each rank's permission mask (in hex).
- A types `.org_join <orgId> <B>` for each. B joins at the entry rank, Team Member (rank 2) or Command Initiate, which may deposit items and cash but not withdraw either (D-BV12).
- A needs some naquadah for step 23, and B a stackable item, for example a crafting supply from the hub's Common Materials Components vendor (1 naquadah each; it lands in the crafting bag, which is a valid source).

<!-- markdownlint-disable MD029 -- step numbers continue from the personal-bank list above -->

15. **Open as a member.** A right-clicks the Team Banker (spawn 471: "Storage Officer" in the Cellblock guard uniform, in the second row off the B-C wall). The Team vault opens on the first click, with 40 slots. Repeat at the Command Banker (472, plain crew clothes): the Command vault opens, with 100 slots.
    - SigNoz: `target=bank event=org_vault_open_requested scope=team` (cell, with `banker_id`, `space_id` and `distance` at most 5), then `target=bank event=org_vault_opened org_type=team rank=8 can_deposit=true can_withdraw=true vault_slots=40` (span `bank.org_vault_open`), then `target=bank event=vault_session_opened scope=team` with `org_id` (span `bank.org_vault_grant`). The Command run shows `scope=command org_type=command`.
16. **Open as a non-member.** A character in no Team right-clicks the Team Banker. Nothing opens; chat says "You are not in a Team, so there is no Team vault to open." The same at the Command Banker says "Command".
    - SigNoz: `target=bank event=org_vault_open_rejected reason=not_in_org scope=team` (WARN), with `banker_id`. No `org_vault_opened` and no `vault_session_opened` follow.
17. **Deposit.** B opens the Team vault and drags the stackable item from a bag into a vault slot. It stays there; close and reopen, and it is still there.
    - SigNoz: `target=bank event=org_move_accepted direction=deposit perm=DepositBank org_type=team rank=2`, with `item_id`, `type_id`, `quantity`, `target_container_id=19`, `target_slot_id` below `vault_slots`, and the stack sizes before and after, in the span `bank.org_move_item`.
18. **Withdraw without and with `WithdrawBank`.** B drags the item back to a bag. It is refused with a line and snaps back into the vault. A then types `.org_set_perms <orgId> 2 <mask>`, with 0x20000 added to rank 2's mask from `.org_info`. B drags again: the item lands in the bag.
    - SigNoz: first `target=bank event=org_move_rejected reason=missing_permission perm=WithdrawBank` (WARN) with `vault_end=source`; then `target=org event=org.gm_action action=gm_org_set_perms outcome=ok` with the `org_id` and `rank`; then `target=bank event=org_move_accepted direction=withdraw perm=WithdrawBank source_container_id=19`, with the same `item_id` as step 17.
19. **A bound item is refused (local server only).** Nothing in game binds an item, so on a local server bind one of B's carried items by hand, `UPDATE sgw_inventory SET bound = true WHERE item_id = <id>`, then relog B. B drags it into the Team vault: it is refused with a line, and it stays in the bag. Skip this step on the colo.
    - SigNoz: `target=bank event=org_move_rejected reason=bound_item_not_org_storable vault_end=target`, with the `item_id` and `type_id`.
20. **A kick ends the session.** With B's Team vault window open, A removes B from the Team (the Team window's roster). Record whether B's vault window closes. Either way, B's next drag in that window is refused with a line and snaps back. A invites B back (`.org_join <orgId> <B>`) before step 22.
    - SigNoz: `target=org event=org.kick outcome=ok` and `target=org event=member_left reason=kicked`, then `target=bank event=vault_session_closed reason=org_left scope=team` with B's `player_id` and the `org_id`. B's drag: `target=bank event=org_move_rejected reason=no_vault_session`.
21. **Disband is blocked while the vault holds items.** With an item in the Command vault, A types `.org_disband <commandId>`. It is refused with a line, and `.org_list` still shows the Command. A withdraws the item and disbands again: it succeeds. (Keep the Team: steps 22 to 25 use it.)
    - SigNoz: `target=org event=org.disband outcome=rejected reason=vault_not_empty`, then, after the withdrawal (`target=bank event=org_move_accepted direction=withdraw`), `target=org event=org.disband outcome=ok`.
22. **A second online member sees the move.** A and B are both in the Team, both with the Team vault open at the Team Banker. A deposits an item: it appears in B's window without B reopening it. A withdraws it: it leaves B's window.
    - SigNoz: `target=bank event=org_move_accepted` for A, then `target=bank event=org_vault_fanout` with `updated_recipients=1` for the deposit, and `removed_ids=1 removed_recipients=1` for the withdrawal. A `target=bank event=org_move_resync_failed` row means the fan-out did not go out.
23. **Treasury.** With the Team vault open, A deposits 300 naquadah with the window's cash control. A's cash drops by 300, and chat says "You deposited 300 naquadah into the team treasury. It now holds 300." B, with the Team window open, sees the treasury change. B deposits 10: it works, because the entry rank holds `DepositCash`. B's withdraw control should be disabled, because the rank lacks `WithdrawCash`; if B can still send a withdrawal, it is refused with "Your rank may not withdraw naquadah." A withdraws 50: "You withdrew 50 naquadah from the team treasury. It now holds 260."
    - SigNoz: `target=bank event=org_cash_transfer direction=deposit amount=300 org_type=team`, with `player_cash_before - player_cash_after = 300`, `org_cash_after - org_cash_before = 300` and `recipients=2`, in the span `bank.org_cash_transfer`; then B's deposit, then `direction=withdraw amount=50`. A refused withdrawal from B is `target=bank event=org_cash_rejected reason=no_permission perm=WithdrawCash`, with the balances unchanged.
24. **Team vault expansion (GM, leader).** A types `.orgvaultexpand`. Chat quotes: "orgvaultexpand: the Team vault has 40 slots. The next +10 costs 100 from the treasury, which holds 260. Type .orgvaultexpand 40 to buy it." Nothing changes. A types `.orgvaultexpand 40`: "orgvaultexpand: the Team vault now has 50 slots. The treasury paid 100 and holds 160." B sees the treasury drop. A types `.orgvaultexpand 40` again: "orgvaultexpand: the Team vault has 50 slots, not 40. Nothing was charged." A reopens the Team vault at the Team Banker: it shows 50 slots (confirm, and note whether A's window grew while it was open). B's already-open window keeps 40 slots until B reopens it (a known gap).
    - SigNoz: `target=bank event=expand_quote scope=team vault_slots=40 price=100 org_cash=260`, then `target=bank event=expand scope=team trigger=gm_console gm_override=true vault_slots_before=40 vault_slots_after=50 price=100`, with `org_cash_before - org_cash_after = 100`, and beside it `target=bank event=org_cash_transfer direction=vault_expansion amount=100 vault_slots_before=40 vault_slots_after=50`, in the spans `bank.console_org_expand` (cell) and `bank.org_vault_expand` (base). The repeat is `target=bank event=expand_rejected reason=replay vault_slots=50 offered_slots=40`.
25. **Team expansion refusals.** Each is refused with a line, and nothing changes:
    - `.orgvaultexpand command` (A leads a Command; found one again if step 21 disbanded it): "orgvaultexpand: a Command vault is fixed at 100 slots. Nothing was charged."
    - A withdraws the treasury below 100, then `.orgvaultexpand 50`: "orgvaultexpand: the next step costs 100; the treasury holds N. Nothing was charged."
    - B types `.orgvaultexpand`: ".orgvaultexpand needs GM access. Nothing was charged.", and the line is not said aloud. If B has GM access, B's `.orgvaultexpand 50` is refused with "orgvaultexpand: only the Team's leader may expand its vault. Nothing was charged."
    - SigNoz: `target=bank event=expand_rejected` with `reason=command_vault_fixed scope=command`, `reason=insufficient_org_cash` (with `org_cash` below `price`), `reason=not_gm`, or `reason=not_leader` (with `rank`); no `expand` or `org_cash_transfer` row follows any of them.

<!-- markdownlint-enable MD029 -->

### After the UAT

After the UAT, the coordinator also reads the `.bug` bookmarks, and every `target=bank` WARN row in the UAT window: a WARN no step above expects is a finding.
