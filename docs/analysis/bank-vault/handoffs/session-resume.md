# Bank and Vault: Session Resume

> Type: how-to. Audience: the owner (UAT) and any later session.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: Waves 0 to 2 merged; BV-05 and BV-07 writing

| Packet | Status | PR | Notes |
|---|---|---|---|
| Plan | Done | #860 | The ledger, and two doc bugs fixed (audit §3) |
| BV-E1 | Done | #867 | Client evidence |
| BV-01 | Done | #872 (`09aadf59c`) | Capacity, the allowlist, and #798 (closed) |
| BV-02 | Done | #921 (`6f00cce9d`) | The Banker open path and GM `.bank` |
| BV-03a | Done | #927 (`490fe4200`) | The pure `move_/mod.rs` split, merged ahead of BV-03 |
| BV-03 | Done | #935 (`dfb0b0f8d`) | Deposit and withdraw, and the use and removal gate |
| BV-04 | Done | #931 (`1c6bed3cc`) | The debug-hub Banker and `.bankdump` |
| BV-05 | Writing | | Expansion. Branch `bank/bv05-expansion` |
| BV-06 | BlockedDependency (BV-05) | | Close-out and release 1 |
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

## Known gaps (carried forward)

- **Files over the hard cap**, pre-existing and not owned by the bank. BV-02 to BV-04 each added a few lines:
  - `crates/entity/src/cell_entity/entity_struct.rs` (775 lines);
  - `crates/cell-methods/src/cell/cell_methods/player/interaction/mod.rs` (732);
  - `crates/wire/src/cell/messages/cell_to_base.rs` (863).
- **Files over the soft cap:** `move_/container_policy.rs` (564) and `move_/vault_move_tests.rs` (574). `move_/mod.rs` is already split (#927) and is 464 lines.
- **BV-01 residual.** The success-path snapshot is read after the lock is released.
- **Non-vault silent rejects are unchanged:** a split onto an occupied slot in the backpack, a bad slot outside 17, and `item_allows_container` outside the vault.
- **Live resize.** Whether an `onBagInfo` sent mid-session resizes an open vault window is inferred only; BV-05's UAT confirms it (step 11).
- **Issue #928** (the buyback lock order) is not a bank issue; it is in triage.
- **The archived-mail storage loophole** (SS-M4, social). Archived mail never expires, so it can hold items and cash indefinitely. Social is asking the owner for a cap; the coordinator suggested capping archives that hold attachments or cash at 100.

## UAT checklist

Run on the colo after release 1, as a GM, with a fresh character. At anything odd, type `.bug <what you saw>`. The bookmark captures the scene.

1. **Declared size.** As a GM, open the vault by typing `.bank`. The window shows `bank_slots` slots, which is 40 before any expansion (BV-05), and does not scroll past them.
2. **Banker click.** Go to the Castle Cellblock stasis-room debug hub. Right-click the Banker, "Storage Officer" (spawn 470), in the middle of the room's B-C wall. The vault opens on the first click.
3. **Deposit.** Drag an item from the backpack into a vault slot. It stays there. Close the vault and reopen it: it is still there.
4. **Withdraw.** Drag it back to the backpack. It lands there, and the vault slot empties.
5. **Stacks.** Split a stack into the bank, then merge it back. The totals never change.
6. **Relog.** Deposit an item, log out, log in, and open the vault. The item is still there.
7. **Walk away.** Open the vault at the Banker, walk more than about 5 m away, then try to drag. The move is refused with a visible message, and the item snaps back.
8. **Mission item.** Try to bank a mission item. It is refused with a message.
9. **Trade and vendor.** A banked item cannot be traded or sold. Only the backpack is offered.
10. **Buyback (#798).** Sell an item, then try to drag it out of the buyback tab without paying. It is refused.
11. **Expansion.** At the Banker, pick "Expand vault" (100 naquadah). The window grows to 50 slots, and the cash drops by 100. Repeat until 100 slots. At 100, the option refuses with a message.
12. **Not enough cash.** With less than 100 naquadah, expansion refuses with a message, and nothing changes.
13. **GM override.** As a GM away from any Banker, `.bank` opens the vault, and moves work. As a non-GM, `.bank` is refused.

The org-vault steps are added by BV-10.

After the UAT, the coordinator reads SigNoz for `bank` events from the [telemetry catalog](../work-packets.md#contract-fixed-by-this-ledger) (`vault_session_opened`, `move_accepted`, `move_rejected reason=…`, `expand`, `expand_rejected reason=…`) and for the `.bug` bookmarks.
