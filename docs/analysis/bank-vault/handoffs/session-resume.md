# Bank and Vault: Session Resume

> Type: how-to. Audience: the owner (UAT) and any later session.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: plan written, Wave 0 dispatching

| Packet | Status | PR | Notes |
|---|---|---|---|
| Plan | Review | #860 | The ledger, and two doc bugs fixed (audit §3) |
| BV-01 | Writing | | Worktree `bank-bv01`, branch `bank/bv01-capacity-allowlist` | Capacity, the allowlist, and #798 |
| BV-E1 | Writing | | Worktree `bank-bve1`, branch `bank/bve1-client-evidence` | Client evidence |
| BV-02 to BV-06 | BlockedDependency | | The personal bank, ending with release 1 |
| BV-07 to BV-10 | BlockedDependency or BlockedDecision | | The org vaults, waiting on cimmeria-fa ORG-02 and ORG-07 |

Coordinator: session cimmeria-97, worktree `.claude/worktrees/bank-coord`. Worker rules: `%TEMP%\cimmeria-castle\BANK-WORKER-RULES.md`.

Cross-campaign agreements:

- **cimmeria-fa (organizations).**
  - The ORG-API is described in its ledger.
  - `org_vault_is_empty(tx, org_id)` and `org_vault_is_empty_sql(org_id)` ship as stubs that return `true`; BV-07 replaces them.
  - D-ORG20 (PR #857): when an org's last member deletes their character and the vault is not empty, the org is kept, memberless.
  - D-ORG21 is our D-BV12.
- **cimmeria-af (crafting).** Container 15 stays movable to and from 1. Crafting rebases onto BV-01. CR-05 may add a post-commit notify in `move_/mod.rs`; af messages before starting it.

## Resuming

1. Read the status lines above and in [work-packets.md](../work-packets.md).
2. For any packet in Writing, check its worktree `.claude/worktrees/bank-<packet>` and its branch `bank/<packet>-*`.
3. For any packet in Review, check its PR's CI. If the CI is older than `main`, rebase and re-test before merging.
4. Before starting Wave 4, ask cimmeria-fa whether ORG-02 and ORG-07 have merged.

## UAT checklist

Run on the colo after release 1, as a GM, with a fresh character. At anything odd, type `.bug <what you saw>`. The bookmark captures the scene.

1. **Declared size.** Open the vault by typing `.bank`. The window shows 40 slots and does not scroll past them.
2. **Banker click.** Go to the Castle Cellblock stasis-room debug hub. Right-click the Banker (spawn 470). The vault opens on the first click.
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

After the UAT, the coordinator reads SigNoz for `bank` events (`vault_open`, `move_rejected reason=…`, `expand`) and for the `.bug` bookmarks.
