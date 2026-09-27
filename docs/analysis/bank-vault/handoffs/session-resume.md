# Bank and Vault: Session Resume

> Type: how-to. Audience: the owner (UAT) and any later session.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: Waves 0 to 3 merged; BV-06 in review, BV-07 writing

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
| BV-06 | Review | this PR | Close-out docs and the UAT queries. The coordinator posts `/release` after it merges (release 1) |
| BV-07 | Writing | | The org vaults. Branch `bank/bv07-org-vault`. Builds on ORG-02 (#881) and ORG-06 (#941); ORG-07 is not needed |
| BV-08 | BlockedDependency (BV-07) | | Org cash. The CM 19 cell forward comes with org ORG-07 |
| BV-09 | BlockedDependency (BV-07) | | No longer BlockedDecision: the owner settled the payer (D-BV28) |
| BV-10 | BlockedDependency (BV-07, BV-08, BV-09) | | Close-out and release 2 |

Coordinator: session cimmeria-79 (formerly cimmeria-97), worktree `.claude/worktrees/bank-ledger`. Worker rules: `%TEMP%\cimmeria-castle\BANK-WORKER-RULES.md`.

Merge bar: green CI plus the coordinator's review. Copilot review is suspended while its spend is exhausted.

Cross-campaign agreements:

- **cimmeria-1f (organizations, formerly cimmeria-fa).**
  - The ORG-API is described in its ledger.
  - `org_vault_is_empty(tx, org_id)` and `org_vault_is_empty_sql(org_id)` ship as stubs that return `true`; BV-07 replaces them.
  - D-ORG20 (PR #857): when an org's last member deletes their character and the vault is not empty, the org is kept, memberless.
  - D-ORG21 is our D-BV12.
  - BV-07 needs only ORG-02 and ORG-06, both merged. BV-08 needs ORG-07's CM 19 cell forward.
- **cimmeria-23 (crafting, formerly cimmeria-af).** Container 15 stays movable to and from 1. CR-05 may add a post-commit notify in `move_/after_commit.rs`; cimmeria-23 messages before starting it.
- **cimmeria-3d (social, formerly cimmeria-19).**
  - Vault mail aliases (D-SS07, our D-BV29) are ours, in Wave 4 after BV-07. The seam is `resolve_recipient_flags`; until we replace it, `MAIL_ToVault` is refused.
  - Mail attachments come from bags 1 and 15 only (D-BV30); SS-M2 (#912) refuses 16 to 20.
- **cimmeria-11 (black market).** The auction container (18) is theirs. `player_movable` refuses it until their packet wires it.

## Resuming

1. Read the status lines above and in [work-packets.md](../work-packets.md).
2. For any packet in Writing, check its worktree `.claude/worktrees/bank-<packet>` and its branch `bank/<packet>-*`.
3. For any packet in Review, check its PR's CI. If the CI is older than `main`, rebase and re-test before merging.
4. BV-07 needs only ORG-02 and ORG-06, which have merged. Before starting BV-08, ask cimmeria-1f whether ORG-07 has merged.
5. Once BV-06 merges, post `/release` on it from PowerShell (D-BV11), then hand the owner the [UAT checklist](#uat-checklist).

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

- **Players cannot buy an expansion.** Dialog 60110 is quarantined (D-BV35), so a Banker offers nothing and only a GM can buy, with `.bankexpand`. See [Lifting the #943 quarantine](#lifting-the-943-quarantine).
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
- **Files over the soft cap:** `move_/container_policy.rs` (564), `move_/vault_move_tests.rs` (574), and BV-05's `cell-interactions` `bank/expand_tests.rs` (511). `move_/mod.rs` is already split (#927) and is 464 lines; `base-session` `bank_expand/mod.rs` is 473.
- **BV-01 residual.** The success-path snapshot is read after the lock is released.
- **Non-vault silent rejects are unchanged:** a split onto an occupied slot in the backpack, a bad slot outside 17, and `item_allows_container` outside the vault.
- **Live resize.** Whether an `onBagInfo` sent mid-session resizes an open vault window is inferred only (BV-E1 Q2). UAT step 11 checks it with `.bankexpand` while the window is open; the chat line acknowledges the purchase either way.
- **`price_missing` has no live-DB test**, because a seed gap would break the shared test database. The classification and the log row are pinned by unit tests.
- **Issue #928** (the buyback lock order) is not a bank issue; it is in triage.
- **The archived-mail storage loophole** (SS-M4, social). Archived mail never expires, so it can hold items and cash indefinitely. Social is asking the owner for a cap; the coordinator suggested capping archives that hold attachments or cash at 100.

## UAT checklist

Run on the colo after release 1, as a GM, with a fresh character. At anything odd, type `.bug <what you saw>`. The bookmark captures the scene.

Each step ends with the SigNoz query that verifies it. Filter the logs by the attributes shown (`target=bank event=… reason=…`) plus the tester's `player_id`. The event names and fields are the [telemetry catalog](../work-packets.md#contract-fixed-by-this-ledger). Most `bank` success rows are DEBUG, which `OTEL_FILTER` exports for this target (`bank=debug`).

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

The org-vault steps are added by BV-10.

After the UAT, the coordinator also reads the `.bug` bookmarks, and every `target=bank` WARN row in the UAT window: a WARN no step above expects is a finding.
