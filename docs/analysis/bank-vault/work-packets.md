# Bank and Vault Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](README.md), [audit](audit.md), [session resume](handoffs/session-resume.md), [testing playbook](../../../TESTING.md), [organizations ORG-API](../organizations/work-packets.md).

## Dispatch rules

- One worktree per worker, made with `bash tools/build-lane/mk-worktree.sh bank/<packet>-<slug> bank-<packet>`. Each worker has its own `sgw_<worktree>` test database.
- Every cargo call goes through `bash tools/build-lane/lane.sh cargo <cmd> -p <crate>`. Live-DB tests run through `bash tools/build-lane/live-db-test.sh <filter>`. Never `--workspace`, never `--exclusive`.
- The toolchain is pinned to 1.98.1. After any dependency change, run hakari and `python tools/crate-graph/crate_graph.py --check`.
- Before deleting a worktree, remove its `external` junction without recursing (`cmd /c rmdir external`).
- Workers push their branch; the coordinator opens the PR and squash-merges it once CI is green. If a PR's CI is older than the current `main`, the coordinator rebases and re-tests before merging.
- Initial state: documentation only, against `main` @ `004bccb4`.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

`rust-gameserver-dev` is the default writer. The advisors are:

- `items-systems-advisor`: containers, `moveItem`, and stacks;
- `server-authority-enforcer`: reviews BV-01, BV-03, BV-05, BV-07 and BV-08;
- `database-persistence`: schema, for BV-01, BV-05 and BV-07;
- `game-archaeology-specialist`: BV-E1;
- `documentation-writer`: the doc updates each packet owes.

## Contract fixed by this ledger

Parallel packets build against these names. A worker who needs to change one raises it with the coordinator instead of renaming it locally.

**Capacity (BV-01).**

- `cimmeria_wire::containers::bag_max_slots(container_id) -> i32` is the only capacity table. Container 17 returns 100, the maximum; 19 returns 100; 20 returns 100.
- `BAG_SIZES` in `crates/entity` derives from it, or is replaced by it. They must not be able to disagree.
- `sgw_player.bank_slots smallint NOT NULL DEFAULT 40`, with `CHECK (bank_slots BETWEEN 40 AND 100 AND bank_slots % 10 = 0)`. It is loaded with the player.
- `onBagInfo` declares container 17 as `bank_slots` for that player, both at world entry and on resync.
- A move into slot `>= bank_slots` of 17 is rejected.

**Player-movable containers (BV-01).** `move_/` gains one explicit allowlist, `player_movable(container_id) -> Movable`, whose values are:

- `Yes`: every container that is legal today, including 1 and 15 in both directions, the equipment and bandolier containers, and whatever else is currently movable;
- `No`: 16 (buyback, which closes #798), 18, 19 and 20;
- `VaultSession`: 17.

BV-01 treats `VaultSession` as `No`. BV-03 wires it to the session.

**Vault session (BV-02).** On the cell player entity:

```rust
pub struct VaultSession {
    pub scope: VaultScope,        // Personal | Team | Command
    pub banker_id: Option<u32>,   // None = GM `.bank`
    pub space_id: u32,
    pub opened_at: Instant,
}
pub enum VaultScope { Personal, Team, Command }
```

- One `vault_session: Option<VaultSession>` per player.
- It is set only by the Banker arm or by `.bank`.
- It is cleared on a space change, on logout, and whenever a new `interact` pins a different target.
- A pure predicate, `vault_move_allowed(&player, &space) -> Result<(), VaultReject>`, is the only place the session and proximity rule lives. It uses the same `interact_target_in_range` distance and same-space rule as other interactions.
- The base checks the move, so the cell attaches the verdict to the forwarded move. The exact message shape is BV-03's choice, recorded in its worknote.

**Banker template (BV-02).** `resources.entity_templates.vault_scope text NOT NULL DEFAULT 'personal' CHECK (vault_scope IN ('personal','team','command'))`. The value is read only when `interaction_type & INT_BANKER != 0`. It becomes `NpcInteractionType::Banker { scope: VaultScope }`.

**Expansion price (BV-05).** `resources.bank_expansion_price (to_slots smallint PRIMARY KEY, price_naquadah integer NOT NULL CHECK (price_naquadah >= 0))`, seeded with 50 through 100 at 100 naquadah each (D-BV02).

**Log target.** Every packet logs under the `bank` target, with events `vault_open`, `vault_open_rejected`, `move_accepted`, `move_rejected reason=…`, `expand`, `expand_rejected reason=…`, and later `org_cash_transfer`. Follow `docs/architecture/instrumentation-discipline.md`, and add the OTEL filter rows it requires.

## Dependency graph and waves

```text
Wave 0 (now)                Wave 1          Wave 2                          Wave 3                    Wave 4 (after cimmeria-fa ORG-02 + ORG-07)
BV-01 capacity+allowlist ─► BV-02 Banker ─┬► BV-03 bank moves ────────┬► BV-05 expansion ─► BV-06 close-out, release 1 ─► BV-07 org vault ─► BV-08 org cash ─┬► BV-10 close-out, release 2
BV-E1 client evidence ─────────────────────┼► BV-04 debug Banker + GM ─┘                                                   └──────────► BV-09 team expansion ──┘
                                           └──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘ (BV-E1 feeds BV-03 and BV-05)
```

BV-01 is the bottleneck. It stays small: tables, allowlist, #798, and the capacity column. No bank behaviour is switched on.

## Contended files

The coordinator merges these one packet at a time, and tells the named campaign first.

- `crates/base-methods/src/base/world_entry/methods/inventory/move_/mod.rs`: BV-01 (allowlist), then BV-03 (session wiring). Crafting's CR-05 may add a post-commit notify; cimmeria-af messages before starting it.
- `crates/wire/src/containers.rs` and `crates/entity/src/inventory.rs`: BV-01 only. Crafting depends on container 15 staying movable in both directions.
- `crates/entity/src/cell_entity/mod.rs` and `crates/cell-world/src/cell/space_manager/spawn.rs`: BV-02.
- `crates/cell-interactions/src/cell/interactions/dispatch/interact.rs`: BV-02.
- `db/resources/Entities/Seed/entity_templates.sql` and `db/resources/Worlds/Seed/spawnlist.sql`: BV-04 only, rows 370-389 and 470-489 only. Harset, crafting, guilds and pets own the other blocks.
- `db/sgw/Players/Tables/sgw_player.sql`: BV-01 (`bank_slots`).

## BV-01 capacity and the player-movable allowlist

**Status: Ready.** Audit rows A-20, A-21, A-22, A-29 and A-31; decisions D-BV06, D-BV07 and D-BV17.

Scope:

1. Make `bag_max_slots` the single capacity table, with 17, 19 and 20 at 100, and derive `BAG_SIZES` from it. 20 is newly declared, which is harmless because it is not movable.
2. Add `sgw_player.bank_slots` (see the contract). Load it with the player, and send it as container 17's size in `onBagInfo`, both at world entry and on resync.
3. Add `player_movable` in `move_/`. Every move that is legal today must stay legal. Refuse moves into and out of 16 (#798).
4. Fix the `VENDOR_COST_BAGS` comments (A-31).

Tests:

- **Unit.** `bag_max_slots` agrees with `BAG_SIZES` for every id from 1 to 20. `player_movable` gives the expected answer for each container.
- **Wire-format.** `onBagInfo` carries container 17 at 40 for a default player, byte-exact.
- **Live-DB regression guards**, each of which must fail with the fix reverted:
  - a move out of 16 is rejected and no row changes;
  - a move into 17 is still rejected;
  - a move between 1 and 15 still succeeds, in both directions;
  - a row placed in container 17 by hand loads, and is sent.

Docs to update: `docs/gameplay/inventory-system.md` (the capacity source), `TESTING.md` if any type guidance shifts, and close #798 in the PR body.

## BV-E1 client evidence

**Status: Ready.** Read-only. The writer is `game-archaeology-specialist`. Audit rows A-02, A-09 and A-12.

Answer, with Ghidra addresses and Lua lines, in `docs/reverse-engineering/findings/bank-vault-client.md`, and index it:

1. Does the client need an `onBagInfo` for 17 before or with `onVaultOpen`, or is the world-entry declaration enough?
2. Does an `onBagInfo` sent mid-session with a larger size for 17 resize an open vault window, or only take effect when it next opens?
3. Can a Banker NPC offer a dialog button such as "Expand vault" through `DialogOverride` or a dialog set? Is the chosen button id reported back through `dialogButtonChoice`? Name the exact seed and wire shape. See `docs/content/` and `reference_client_dialog_ui_types` in agent memory.
4. How does `onVaultOpen`'s `Position` argument get used: range-closing the window, or nothing?
5. A property-index sweep for `isBankingOverride` (A-12). This is low priority.

## BV-02 Banker open path

**Status: BlockedDependency (BV-01).** Audit rows A-01, A-02, A-24 to A-27; decisions D-BV03, D-BV05, D-BV09 and D-BV10.

Scope:

- Add `vault_scope` to `entity_templates`.
- Add `NpcInteractionType::Banker { scope }`, derived in `static_interaction_for_flags`.
- Add `crates/cell-interactions/src/cell/interactions/bank.rs`. It checks distance, pins the Banker, sets `vault_session`, sends `onVaultOpen(banker_id, banker_pos)`, and logs `bank vault_open`.
- Team and Command scopes send an error reply with feedback until Wave 4.
- Add a GM-gated `.bank` console command that opens the personal vault anywhere, with a session whose `banker_id` is `None`.
- Clear the session on a space change, logout and re-pin.

Every refusal gets visible feedback (a project rule).

Tests:

- **Unit.** The derivation, and the session lifecycle.
- **Wire-format.** `onVaultOpen` args, byte-exact.
- **Smoke / dispatch.** An `interact` on a Banker template sends 106, and one out of range does not.
- **Console.** `.bank` needs GM access.

Docs to update: `docs/content/interaction-flags.md`, `docs/gameplay/inventory-system.md`, and the console command docs.

## BV-03 bank moves

**Status: BlockedDependency (BV-02).** Audit rows A-20, A-22, A-28 and A-30; decisions D-BV04, D-BV05, D-BV07 and D-BV08.

Scope:

- Wire `Movable::VaultSession` to `vault_move_allowed`, checking proximity on **every** move.
- A slot must be below `bank_slots`.
- Mission items are refused.
- Moves both into and out of 17 cover deposit, withdraw, swap, stack merge and split. Reuse the existing split and merge path.
- A rejection sends feedback, then re-syncs the affected slots with `onUpdateItem`, so the client's drag snaps back.

Tests:

- **Live-DB regression guards**, each proven to fail with the fix reverted:
  - walking away after opening, then moving, is rejected;
  - a move with no session is rejected;
  - a slot at or above `bank_slots` is rejected;
  - a mission item is rejected;
  - deposit then withdraw round-trips the same `item_id`, with no duplicate;
  - a split into the bank conserves the total count.
- **Concurrency.** Two concurrent moves of one stack cannot duplicate it.

Review by `server-authority-enforcer`.

## BV-04 debug-hub Banker and GM helpers

**Status: BlockedDependency (BV-02).** Audit rows A-32, A-33 and A-34.

Scope:

- A Banker template, 370 (`INT_BANKER`, `vault_scope = 'personal'`, a terminal or human body per A-34, non-hostile), spawned as 470 in the Castle Cellblock stasis-room debug hub.
- Follow `docs/content/debug-hub.md` and `.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`.
- Add a `.bankdump [player]` GM command that lists container 17 read-only.
- If it is cheap, let `gmGiveItem` take a target container, so a tester can seed 17.
- Update `docs/content/debug-hub.md`: replace the "Bank: known missing" row.

Tests:

- **Seed guard (live-DB).** Template 370 has `INT_BANKER`, and spawn 470 is in the hub.
- **Console tests** for the helpers.

## BV-05 vault expansion

**Status: BlockedDependency (BV-03, BV-E1).** Decision D-BV02.

Scope:

- The `bank_expansion_price` seed table.
- A Banker dialog option, shaped by BV-E1 question 3.
- Buying one step debits naquadah and raises `bank_slots` by 10, in one statement or one transaction, so the purchase is atomic and replay-safe.
- It re-declares container 17 with `onBagInfo` (behaviour per BV-E1 question 2) and updates the cash.
- At 100 slots, or without the cash, the player gets feedback.

Tests:

- **Live-DB guards.** A double purchase from one click is charged once. Insufficient funds leaves no change. 100 is the ceiling.
- **Wire-format.** The re-sent `onBagInfo`.

Review by `server-authority-enforcer`.

## BV-06 personal-bank close-out and release 1

**Status: BlockedDependency (BV-03, BV-04, BV-05).**

Scope:

- Docs: `docs/gameplay/inventory-system.md` (a Bank/Vault section), `docs/gap-analysis.md`, `docs/project-status.md`, and the test inventory if the counts cross the 5% threshold.
- Finalize the [UAT checklist](handoffs/session-resume.md#uat-checklist).
- Post `/release` on this PR once it has merged, from PowerShell (D-BV11).

## BV-07 org vault storage and open path

**Status: BlockedDependency (cimmeria-fa ORG-02, ORG-07).** Decisions D-BV09, D-BV12, D-BV13, D-BV14 and D-BV18.

Scope:

- Org-owned item storage. `sgw_inventory.character_id` is `NOT NULL` (A-23), so decide the table shape with `database-persistence`. Its FK to `sgw_organizations` must not cascade-delete (D-BV18).
- Declare 19 and 20 in `onBagInfo`, and send org vault contents to members.
- Team and Command Banker arms that send 107 and 108, gated on org membership.
- Deposits and withdrawals under ORG-LOCK (`lock_org`, then `member_access_locked`, then items), with the `DepositBank` and `WithdrawBank` bits.
- A bank log table.
- Replace cimmeria-fa's stubs `org_vault_is_empty(tx, org_id)` and `org_vault_is_empty_sql(org_id)`.
- Fan out with `broadcast_to_org`.

## BV-08 org cash

**Status: BlockedDependency (BV-07, ORG-07).** Decision D-BV15.

Scope:

- Replace the reject-with-feedback base arm for `OrgCellToBase::TransferCash`. The sign of the amount picks the direction.
- `DepositCash` and `WithdrawCash` are separate bits.
- No overflow, and no negative balance (`sgw_organizations.cash CHECK >= 0`).
- One transaction under ORG-LOCK, with the lock order org, then player.
- Log each transfer.
- Broadcast `onOrganizationCashUpdate` (built by `build_on_organization_cash_update`) after the commit.

## BV-09 Team vault expansion

**Status: BlockedDecision.** Who pays for a +10 step: the org treasury, or the member who clicked, and which permission bit is needed? Ask the owner before Wave 4. The expected price table matches D-BV02.

## BV-10 org close-out and release 2

**Status: BlockedDependency (BV-07, BV-08, BV-09).** Update the docs and `docs/gameplay/organization-system.md`, extend the UAT checklist, and post `/release` (D-BV11).
