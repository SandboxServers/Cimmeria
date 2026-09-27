# Bank and Vault Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](README.md), [audit](audit.md), [session resume](handoffs/session-resume.md), [testing playbook](../../../TESTING.md), [organizations ORG-API](../organizations/work-packets.md).

## Dispatch rules

- One worktree per worker, made with `bash tools/build-lane/mk-worktree.sh bank/<packet>-<slug> bank-<packet>`. Each worker has its own `sgw_<worktree>` test database.
- Every cargo call goes through `bash tools/build-lane/lane.sh cargo <cmd> -p <crate>`. Live-DB tests run through `bash tools/build-lane/live-db-test.sh <filter>`. Never `--workspace`, never `--exclusive`.
- The toolchain is pinned to 1.98.1. After any dependency change, run hakari and `python tools/crate-graph/crate_graph.py --check`.
- Before deleting a worktree, remove its `external` junction without recursing (`cmd /c rmdir external`).
- Workers push their branch; the coordinator opens the PR, reviews it, and squash-merges it once CI is green. If a PR's CI is older than the current `main`, the coordinator rebases and re-tests before merging. Copilot review is suspended while its spend is exhausted: green CI plus the coordinator's review is the merge bar.
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

- `cimmeria_entity::inventory::bag_max_slots(container_id) -> i32` is the only capacity table (D-BV20). `cimmeria_wire::containers::bag_max_slots` and `base::resources::bag_max_slots` re-export it, so either path works. Containers 17, 18, 19 and 20 return 100, the maximum.
- `BAG_SIZES` in `crates/entity` derives from it, or is replaced by it. They must not be able to disagree.
- `sgw_player.bank_slots smallint NOT NULL DEFAULT 40`, with `CHECK (bank_slots BETWEEN 40 AND 100 AND bank_slots % 10 = 0)`. It is loaded with the player.
- `onBagInfo` declares container 17 as `bank_slots` for that player, both at world entry and on resync.
- A move into slot `>= bank_slots` of 17 is rejected.

**Player-movable containers (BV-01).** `move_/` gains one explicit allowlist, `player_movable(container_id) -> Movable`, whose values are:

- `Yes`: every container that is legal today, including 1 and 15 in both directions, the equipment and bandolier containers, and whatever else is currently movable;
- `No`: 16 (buyback, which closes #798), 18, 19 and 20;
- `VaultSession`: 17.

BV-01 treats `VaultSession` as `No`. BV-03 (#935) wires it to the session through the cell's verdict (D-BV24).

19 and 20 stay `No` in `player_movable`. BV-07 routes a move whose target is 19 or 20, or whose item is in `sgw_organization_vault_items`, to `move_/org/` before the allowlist runs, and that path applies its own rules (D-BV38).

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
- The base checks the move, so the cell attaches the verdict to the forwarded move. As built (D-BV24), the verdict is `cimmeria_wire::cell::vault::VaultAccess`, carried on the move, use and both removal messages; the design is in `worknotes/bv-03.md`.

**Banker template (BV-02).** `resources.entity_templates.vault_scope text NOT NULL DEFAULT 'personal' CHECK (vault_scope IN ('personal','team','command'))`. The value is read only when `interaction_type & INT_BANKER != 0`. It becomes `NpcInteractionType::Banker { scope: VaultScope }`.

**Expansion price (BV-05).** `resources.bank_expansion_price (to_slots smallint PRIMARY KEY, price_naquadah integer NOT NULL CHECK (price_naquadah >= 0))`, with a CHECK holding `to_slots` to 50-100 in steps of 10, seeded with 50 through 100 at 100 naquadah each (D-BV02). A missing row makes that step unbuyable, never free.

**Telemetry contract (D-BV19).** Every packet satisfies all of the following. A worker who needs a new event adds a row here through the coordinator, never locally.

- **Target:** `bank`. Every event is structured: an `event="…"` discriminator plus fields, never free text alone.
- **Correlators on every event:** `account_id`, `player_id`, `entity_id`. Org events also carry `org_id`, `org_type`, `rank` and the permission bit checked (`perm`). One exception: an infrastructure-failure event (a database error, not a player decision) carries `account_id` only if the account was read before the failure. `account_lookup_failed` (on `grant_rejected` and `use_rejected`), `move_lock_begin_failed`, `move_lock_failed` and `refusal_context_query_failed` therefore have `player_id` and `entity_id` but no `account_id`. `bank_feedback_send_failed reason=no_client_address` has `player_id`, `entity_id` and `item_id` only. `vault_open_rejected reason=player_entity_missing` has only `entity_id` and `banker_id`, because the player's own entity is what went missing. BV-05's cell-side rows for an entity with no character (`expand_rejected reason=player_missing`, `expand_quote_skipped reason=no_player_id`) have no `player_id` for the same reason.
- **Refusals:** a stable `reason=` string from the packet's reject enum. Never a formatted message only.
- **Before and after:** every event that changes state records the prior and new values of whatever it changed: container and slot, `stack_size`, `bank_slots`, player cash, org cash.
- **Guards:** every event row below has a `LogCapture` test (TESTING.md type 12) that asserts its target, level and required fields. Refusal events have one test per `reason`, infrastructure reasons included: inject the failure with a pool that cannot connect, or with a lock held by another connection under a short `lock_timeout`. The only exemption is a reason that cannot be injected without dropping the connection mid-transaction (BV-01: `resync_read_failed` and `move_lock_release_failed`, which need the connection to fail after both locks were taken on it). The worknote names each exempt reason and why.
- **Filter:** `bank` at `debug` has an `OTEL_FILTER` row plus its pinning assertion in `crates/server/src/logging/`. BV-02 added it, as the first packet to emit a debug `bank` event.
- **Spans:** an info span on each dispatch entrypoint: the Banker interact, `.bank`, the bank branch of `moveItem`, the expand purchase and the org cash transfer. No spans inside per-tick work. As shipped: `bank.banker_interact` and `bank.console_open` (BV-02), `bank.move_item` (BV-03), `bank.console_dump` and `bank.gm_dump` (BV-04), and `bank.expand` (the dialog answer), `bank.console_expand` (`.bankexpand`), `bank.expand_purchase` and `bank.expansion_quote` (BV-05), `bank.org_vault_open` (base), `bank.org_vault_grant` (cell) and `bank.org_move_item` (BV-07), `bank.org_cash_transfer` (BV-08), and `bank.org_vault_expand` (base) and `bank.console_org_expand` (cell) (BV-09).

| Event | Level | Packet | Fields beyond the correlators |
|---|---|---|---|
| `move_rejected` | warn | BV-01, BV-03 | `reason` (BV-01: `source_container_not_player_movable`, `target_container_not_player_movable`; BV-03, which retired BV-01's `source_container_needs_vault_session` and `target_container_needs_vault_session`: `no_vault_session`, `banker_out_of_range`, `banker_gone`, `banker_other_space`, `vault_session_other_space`, `player_missing`, `vault_scope_mismatch`, `target_slot_beyond_bank_slots`, `mission_item_not_bankable`, `item_not_allowed_in_container`, `split_onto_occupied_slot`; infrastructure: `move_lock_begin_failed`, `move_lock_failed`, `refusal_context_query_failed`, `move_lock_release_failed`), `item_id`, `type_id`, `quantity` (as requested; `<= 0` is the whole stack), `stack_size`, `source_container_id`, `source_slot_id`, `target_container_id`, `target_slot_id`. The item fields are read under the move lock and the item's row lock, and omitted when the player does not own the item. BV-03's vault refusals add `vault_end` (`source` or `target`), `banker_id`, `distance` and `gm_override`, and `bank_slots` on `target_slot_beyond_bank_slots` |
| `grant_rejected` | warn | BV-01 | `reason` (`grant_into_storage_container`; infrastructure: `account_lookup_failed`), `type_id` (a grant names a type, not an instance), `quantity`, `target_container_id` |
| `move_resync_skipped` | warn | BV-01 | `reason` (`refused_item_not_owned`: the refused move named an item the player does not own, so nothing was resent; `lock_timeout`: the move lock or the item's row lock could not be taken, so nothing was resent, because an unlocked read could overtake a concurrent write's own update; `resync_read_failed`), `item_id` |
| `vault_session_opened` | debug | BV-02 | `scope`, `banker_id` or `gm_override=true`, `space_id`, `distance` |
| `vault_session_closed` | debug | BV-02, BV-07 | `reason` (`space_change`, `logout`, `re_pin`; BV-07 adds `org_left`, when the player leaves, is kicked from, or disbands the organization whose vault is open), `scope`, `open_ms` (milliseconds since the session opened), and `org_id` on a Team or Command session |
| `vault_open_rejected` | warn | BV-02 | `reason` (`out_of_range`, `not_gm`, `banker_missing`; `player_entity_missing`, moved under `bank` by BV-03; `org_vault_not_available` is retired, because BV-07 opens the org vaults), `banker_id`, `distance` |
| `vault_open_send_failed` | warn | BV-02, BV-07 | `reason` (`base_channel_closed`), `banker_id`, `error`; on the org request (BV-07) also `scope` |
| `bank_feedback_send_failed` | warn | BV-02, BV-03, BV-05, BV-08 | `reason` (`base_channel_closed` on the cell, with `error`; `no_client_address` on the base, when there is no session address for the feedback line, with `item_id`; BV-05's expansion sends add `no_session`, `not_in_world` and `send_error`, with `what` = `bag_info`, `cash` or `feedback_line`; BV-08's treasury sends, also used by BV-09, add `what` = `org_cash`) |
| `move_accepted` | debug | BV-03 | `item_id`, `type_id`, `quantity`, `kind` (`deposit`, `withdraw`, `within`, `split`, `merge`, `swap`), source and target container and slot, `source_stack_before`/`source_stack_after`, `target_stack_before`/`target_stack_after`, `bank_slots`, `banker_id`, `distance`, `gm_override` |
| `use_rejected` | warn | BV-03 | `reason` (`container_not_accessible`; infrastructure: `account_lookup_failed`), `item_id`, `container`, `op` (`use` or `remove`), `vault_reason`, `banker_id` |
| `gm_action` | info; warn for infrastructure | BV-04 | `action` (`bankdump`), `target_player_id` or `target_name`, `result`, `item_count` and `bank_slots` on success, `reason` on refusal: `target_not_found`, `caller_not_player`, `not_gm` (info); `db_unavailable`, `query_failed`, `base_channel_closed` (warn, with `error`). `give_to_container` is not emitted: BV-04 skipped a GM grant into 17, because the def fixes `gmGiveItem`'s arguments and grants into 17-20 are refused by design. There is no `target_ambiguous`, because `player_name` is `UNIQUE` |
| `expand` | info | BV-05 | `bank_slots_before`, `bank_slots_after`, `price`, `cash_before`, `cash_after`, `banker_id` or `gm_override=true`, `distance`, `trigger` (`dialog` or `gm_console`) |
| `expand_rejected` | warn | BV-05 | `reason`: the vault verdict's label, as `move_rejected` passes it through (`no_vault_session`, `banker_out_of_range`, `banker_gone`, `banker_other_space`, `vault_session_other_space`, `player_missing`, `vault_scope_mismatch`; these replace the planned `no_session` and `out_of_range`), then `no_offer`, `replay`, `at_ceiling`, `insufficient_cash`, `price_missing`, `price_changed`, `row_changed`; infrastructure: `player_row_missing`, `db_unavailable`, `query_failed`; on the cell: `player_missing` (no character id), `base_channel_closed`, `not_gm`. Fields: `offered_slots`, `offered_price`, `bank_slots`, `cash`, `price`, `banker_id`, `gm_override`, `distance`, `trigger`, and `error` on `query_failed` |
| `expand_quote` | debug; warn on failure | BV-05 | `offered` (`true`, or `false` with `reason`: `at_ceiling` at debug; `price_missing`, `player_row_missing`, `db_unavailable`, `query_failed`, `cell_channel_closed` at warn), `bank_slots`, `cash`, `price`, `error` |
| `expand_offered` | debug | BV-05 | `banker_id`, `gm_override`, `bank_slots`, `price` |
| `expand_offer_dropped` | debug | BV-05 | `speaker_id`, `bank_slots`, `reason` (`entity_missing`, `entity_is_another_player`, `no_vault_session`, `vault_scope_mismatch`, `speaker_changed`) |
| `expand_offer_suppressed` | debug | BV-05 | `speaker_id`, `bank_slots`, `price`, `reason=dialog_quarantined` (once per open while dialog 60110 is quarantined, D-BV35) |
| `expand_dismissed` | debug | BV-05 | `button_id`, `reason` (`closed`, `unexpected_button`) |
| `expand_quote_skipped` / `expand_quote_send_failed` | debug / warn | BV-05 | `reason` (`no_player_id` / `base_channel_closed`) |
| `org_vault_open_requested` | debug | BV-07 | Cell, on a Team or Command Banker click: `scope`, `banker_id`, `space_id`, `distance` |
| `org_vault_opened` | debug | BV-07 | Base: `org_id`, `org_type`, `rank`, `perm="none"` (opening needs no bit, D-BV12), `permissions`, `can_deposit`, `can_withdraw`, `scope`, `vault_slots`, `item_count`, `banker_id`, `space_id`, `distance` |
| `org_vault_open_rejected` | warn | BV-07 | `reason` (base: `not_in_org`, `not_a_member`, `no_such_org`, `wrong_org_type`, `player_missing`, `player_unknown`, `open_query_failed`, `cell_channel_closed`; cell: `banker_not_pinned`, `out_of_range`, `banker_missing`, `stale_entity`, `player_entity_missing`), `org_id`, `org_type`, `scope`, `banker_id`, `space_id`, `distance` |
| `org_move_accepted` | debug | BV-07 | `org_id`, `org_type`, `rank`, `perm` (`DepositBank`, `WithdrawBank`, or both for a cross swap), `item_id`, `new_item_id` (a split), `type_id`, `quantity`, `kind` (`deposit`, `withdraw`, `within`, `split`, `merge`, `swap`), `direction`, both ends, `source_stack_before`/`after`, `target_stack_before`/`after`, `vault_slots`, `banker_id`, `distance`. The same move is one `sgw_organization_vault_log` row |
| `org_move_rejected` | warn | BV-07 | `reason` (the closed verdict's labels, `no_vault_session`, `banker_out_of_range`, `banker_gone` and the rest; `vault_scope_mismatch`; `not_a_member`, `no_such_org`, `wrong_org_type`, `player_missing`; `missing_permission` with `perm`; `item_not_in_vault`; `target_container_not_player_movable`, `source_container_not_player_movable`; `invalid_target_slot`; `target_slot_beyond_vault_slots`; `quantity_exceeds_stack`; `bound_item_not_org_storable`; `mission_item_not_bankable`, `item_not_allowed_in_container`, `split_onto_occupied_slot`; infrastructure: `move_failed`, `move_lock_begin_failed`, `move_lock_failed`), the org fields, `perm`, the item fields when found, `vault_end`, `vault_slots`, `banker_id`, `distance` |
| `org_vault_fanout` | debug | BV-07 | `updated_recipients`, `removed_ids`, `removed_recipients`: the other online members sent the changed vault rows (`broadcast_to_org_except`) |
| `org_move_resync_failed` | warn | BV-07 | `reason` (`vault_read_failed`, `fanout_read_failed`): the read-back after a committed move failed |
| `org_cash_transfer` | info | BV-08, BV-09 | `org_id`, `org_type`, `rank`, `direction` (`deposit`, `withdraw`; BV-09: `vault_expansion`), `amount`, `player_cash_before`/`after` (not on `vault_expansion`), `org_cash_before`/`after`, `vault_slots_before`/`after` (only on `vault_expansion`), `recipients` (online members sent `onOrganizationCashUpdate`). The same transfer is one `sgw_organization_cash_log` row |
| `org_cash_rejected` | warn | BV-08 | `reason` (cell: `zero_amount`; base: `actor_mismatch`, `db_unavailable`, `query_failed`, `player_missing`, `no_such_org`, `not_a_member`, `no_permission`, `insufficient_player_cash`, `insufficient_org_cash`, `player_cash_overflow`, `org_cash_overflow`), `org_type`, `rank`, `direction`, `amount`, the balances read, `perm` and `permissions` on `no_permission`, `error` on `query_failed` |
| `expand` / `expand_quote` / `expand_rejected` (Team vault) | info / debug / warn | BV-09 | The personal vault's events, for `.orgvaultexpand`, with `scope`, `trigger = gm_console`, `gm_override = true` and the org fields (`org_id`, `org_type`, `rank`). `expand`: `vault_slots_before`/`after`, `price`, `org_cash_before`/`after`. `expand_quote`: `vault_slots`, `price`, `org_cash`. `expand_rejected`: `reason` (cell: `not_gm`, `bad_args`, `player_missing`, `base_channel_closed`; base: `not_in_org`, `not_a_member`, `no_such_org`, `wrong_org_type`, `player_missing`, `command_vault_fixed`, `not_leader`, `at_ceiling`, `price_missing`, `replay`, `insufficient_org_cash`, `row_changed`, `db_unavailable`, `query_failed`), `vault_slots`, `offered_slots`, `price`, `org_cash`, `error` |

## Dependency graph and waves

```text
Wave 0 (done)               Wave 1 (done)   Wave 2 (done)                   Wave 3 (done)             Wave 4 (done; BV-07 on ORG-02 + ORG-06; BV-08 on ORG-07)
BV-01 capacity+allowlist ─► BV-02 Banker ─┬► BV-03 bank moves ────────┬► BV-05 expansion ─► BV-06 close-out, release 1 ─► BV-07 org vault ─► BV-08 org cash ─┬► BV-10 close-out, release 2
BV-E1 client evidence ─────────────────────┼► BV-04 debug Banker + GM ─┘                                                   └──────────► BV-09 team expansion ──┘
                                           └──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘ (BV-E1 feeds BV-03 and BV-05)
```

BV-01 is the bottleneck. It stays small: tables, allowlist, #798, and the capacity column. No bank behaviour is switched on.

## Contended files

The coordinator merges these one packet at a time, and tells the named campaign first.

- `crates/base-methods/src/base/world_entry/methods/inventory/move_/mod.rs`: BV-01 (allowlist), then BV-03 (session wiring). It is now split: #927 moved the post-commit side effects into `after_commit.rs`, and BV-03 added `finish.rs`, `apply.rs` and `bank_rules.rs`, leaving `mod.rs` at 464 lines. Crafting's CR-05 may add a post-commit notify in `after_commit.rs`; cimmeria-23 messages before starting it.
- `crates/wire/src/containers.rs` and `crates/entity/src/inventory.rs`: BV-01 only. Crafting depends on container 15 staying movable in both directions.
- `crates/entity/src/cell_entity/mod.rs` and `crates/cell-world/src/cell/space_manager/spawn.rs`: BV-02.
- `crates/cell-interactions/src/cell/interactions/dispatch/interact.rs`: BV-02.
- `db/resources/Entities/Seed/entity_templates.sql` and `db/resources/Worlds/Seed/spawnlist.sql`: BV-04 only, rows 370-389 and 470-489 only. Harset, crafting, guilds and pets own the other blocks.
- `db/sgw/Players/Tables/sgw_player.sql`: BV-01 (`bank_slots`).

## BV-01 capacity and the player-movable allowlist

**Status: Done** (PR #872, `09aadf59c`; closed #798). Audit rows A-20, A-21, A-22, A-29 and A-31; decisions D-BV06, D-BV07 and D-BV17.

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

Telemetry: emit `move_rejected`, `grant_rejected` and `move_resync_skipped` exactly as the contract catalog specifies, with a `LogCapture` test per reason.

## BV-E1 client evidence

**Status: Done** (PR #867). Findings: `docs/reverse-engineering/findings/bank-vault-client.md`. Read-only. The writer is `game-archaeology-specialist`. Audit rows A-02, A-09 and A-12.

Answer, with Ghidra addresses and Lua lines, in `docs/reverse-engineering/findings/bank-vault-client.md`, and index it:

1. Does the client need an `onBagInfo` for 17 before or with `onVaultOpen`, or is the world-entry declaration enough?
2. Does an `onBagInfo` sent mid-session with a larger size for 17 resize an open vault window, or only take effect when it next opens?
3. Can a Banker NPC offer a dialog button such as "Expand vault" through `DialogOverride` or a dialog set? Is the chosen button id reported back through `dialogButtonChoice`? Name the exact seed and wire shape. See `docs/content/` and `reference_client_dialog_ui_types` in agent memory.
4. How does `onVaultOpen`'s `Position` argument get used: range-closing the window, or nothing?
5. A property-index sweep for `isBankingOverride` (A-12). This is low priority.

## BV-02 Banker open path

**Status: Done** (PR #921, `6f00cce9d`). Audit rows A-01, A-02, A-24 to A-27; decisions D-BV03, D-BV05, D-BV09 and D-BV10.

Scope:

- Add `vault_scope` to `entity_templates`.
- Add `NpcInteractionType::Banker { scope }`, derived in `static_interaction_for_flags`.
- Add `crates/cell-interactions/src/cell/interactions/bank.rs`. It checks distance, pins the Banker, sets `vault_session`, sends `onVaultOpen(banker_id, banker_pos)`, and emits the catalog event `vault_session_opened`.
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

Telemetry: emit `vault_session_opened`, `vault_session_closed` and `vault_open_rejected` as the catalog specifies, with spans on the Banker interact and on `.bank`. Add the `OTEL_FILTER` debug row. A `LogCapture` test per event and per reason.

## BV-03 bank moves

**Status: Done** (PR #935, `dfb0b0f8d`; the `move_/mod.rs` split landed first as BV-03a, PR #927, `490fe4200`). Audit rows A-20, A-22, A-28 and A-30; decisions D-BV04, D-BV05, D-BV07 and D-BV08.

Scope:

- Wire `Movable::VaultSession` to `vault_move_allowed`, checking proximity on **every** move.
- A slot must be below `bank_slots`.
- Mission items are refused.
- Moves both into and out of 17 cover deposit, withdraw, swap, stack merge and split. Reuse the existing split and merge path.
- A rejection sends feedback, then re-syncs the affected slots with `onUpdateItem`, so the client's drag snaps back.
- **The slot bound is the player's `bank_slots`, read inside the move transaction.** It is not `bag_max_slots(17)`, which is the ceiling of 100. Apply the same bound wherever `reserve_free_inventory_slots` can reach 17. Without it, a 40-slot player can use slots 40-99 without buying the expansion (BV-01 review, follow-up 2).
- **Player-accessible containers for use and removal.** `useItem`, `removeItem` and content `RemoveItem` (`use_instance.rs`, `remove_instance.rs`, `remove_by_type.rs`) find an item by id without checking its container. So an item sitting in buyback (16) can be used today, and a banked item would be usable from anywhere. Add one shared check next to `player_movable`: 1-15, and 17 only with a vault session (BV-01 review, follow-up 1).
- Split `move_/mod.rs` first. BV-01 (#872) takes it to 694 lines, against a 700-line hard cap: move the post-commit side effects into `move_/after_commit.rs`. Done as BV-03a (#927).

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

Telemetry: emit `move_accepted`, `move_rejected` (session, proximity, slot bound, mission item) and `use_rejected` as the catalog specifies, with before and after slot and stack state on every outcome. A `LogCapture` test per event and per reason.

## BV-04 debug-hub Banker and GM helpers

**Status: Done** (PR #931, `1c6bed3cc`). Audit rows A-32, A-33 and A-34. The Banker is template 370, "Storage Officer", spawn 470 at the middle of the stasis room's B-C wall. The optional `gmGiveItem` target container was skipped (see the `gm_action` catalog row).

Scope:

- A Banker template, 370 (`INT_BANKER`, `vault_scope = 'personal'`, a terminal or human body per A-34, non-hostile), spawned as 470 in the Castle Cellblock stasis-room debug hub.
- Follow `docs/content/debug-hub.md` and `.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`.
- Add a `.bankdump [player]` GM command that lists container 17 read-only.
- If it is cheap, let `gmGiveItem` take a target container, so a tester can seed 17.
- Update `docs/content/debug-hub.md`: replace the "Bank: known missing" row.

Tests:

- **Seed guard (live-DB).** Template 370 has `INT_BANKER`, and spawn 470 is in the hub.
- **Console tests** for the helpers.

Telemetry: emit `gm_action` as the catalog specifies, including refusals with a `reason`. A `LogCapture` test for success and for refusal.

## BV-05 vault expansion

**Status: Done** (PR #947, `6607fbdc2`). Server side and GM `.bankexpand` done; the player-facing Expand button waits on the #943 dialog quarantine (D-BV35). Decisions D-BV02 and D-BV27 (the purchase keeps `bank_slots` grow-only, now held by `persist_expansion`, D-BV32), plus D-BV31 to D-BV35. Worknote: [bv-05.md](worknotes/bv-05.md).

As built, against the scope below:

- The answer buys only on ButtonID 8 (`VAULT_EXPAND_BUTTON_ID`), the server-authority review's should-fix 2. The rest of "the server ignores `button_id`" stands: the button is not an authority check, and the purchase re-checks everything.
- Dialog 60110 is defined but quarantined with the debug-hub dialogs (#943), so no player sees the offer yet. GM `.bankexpand` runs the same purchase with `trigger=gm_console`.
- The seed table is as the contract above says, with the step CHECK on `to_slots`.

Scope:

- The `bank_expansion_price` seed table.
- A Banker dialog option, per BV-E1:
  - On a Banker click, send `onVaultOpen`, and, while `bank_slots < 100`, also send a single-button "Expand vault" dialog. It is a Generic1 button (`button_type = 4`), not Accept.
  - The dialog's `dialogButtonChoice` is **not** an authority check: the server ignores `button_id`, and `OnDialogChoice` matches only the dialog id. The purchase handler must re-check the vault session and the proximity rule itself, then the cash and the ceiling.
- Buying one step debits naquadah and raises `bank_slots` by 10, in one statement or one transaction, so the purchase is atomic and replay-safe.
- It re-declares container 17 with `onBagInfo` and updates the cash. BV-E1 infers that an open window resizes live (through `InventoryUpdateContainerSize`); confirm this in UAT. Every outcome also sends a chat line, so there is feedback even when the window is closed.
- At 100 slots, or without the cash, the player gets feedback.

Tests:

- **Live-DB guards.** A double purchase from one click is charged once. Insufficient funds leaves no change. 100 is the ceiling.
- **Wire-format.** The re-sent `onBagInfo`.

Review by `server-authority-enforcer`.

Telemetry: emit `expand` and `expand_rejected` as the catalog specifies. A zero-row update is logged as `expand_rejected reason=replay`. A `LogCapture` test per event and per reason.

## BV-06 personal-bank close-out and release 1

**Status: Done** (PR #950, `3684fa7eb`, docs only; release 1 is deployed). The guard audit's result is in [session-resume.md § Known gaps](handoffs/session-resume.md#known-gaps-carried-forward): every catalog event from BV-01 to BV-05 has a `LogCapture` guard on `main` except the reasons listed there.

Scope:

- Docs: `docs/gameplay/inventory-system.md` (a Bank/Vault section), `docs/gap-analysis.md`, `docs/project-status.md`, and the test inventory if the counts cross the 5% threshold.
- Finalize the [UAT checklist](handoffs/session-resume.md#uat-checklist).
- Post `/release` on this PR once it has merged, from PowerShell (D-BV11).

Telemetry: add `bank` and its catalog to `docs/architecture/observability.md`. Every UAT checklist step names its SigNoz query (event plus fields). Check that each catalog event from BV-01 to BV-05 has its `LogCapture` guard on `main`.

## BV-07 org vault storage and open path

**Status: Done**, as two stacked PRs: BV-07a (PR #948, `673929c0d`: the schema, the real vault predicate, the org id on the verdict and the session, the Banker round trip, `onBagInfo` with the Team's size, 107/108, `org_left`) and BV-07b (PR #949, `13643442d`: moves into, out of and within 19/20, the bank bits, the entry rules, the vault log, the fan-out). It builds on ORG-02 (#881, `cdbd5ce88`) and ORG-06 (#941); ORG-07's `broadcast_to_org` landed during review and carries the fan-out. Decisions D-BV09, D-BV12, D-BV13, D-BV14, D-BV18 and D-BV24, plus D-BV36 to D-BV40. Worknote: [bv-07.md](worknotes/bv-07.md).

As built, against the scope below:

- The storage is a standalone table, `sgw_organization_vault_items` (D-BV36), with a sibling `sgw_organization_vault_log`. The Team vault's size is `sgw_organizations.vault_slots` (40 to 100); the Command vault is 100 in Rust.
- Entry into 19/20 follows the personal vault's rules, plus no bound items (D-BV37, D-BV38). Every vault action takes the lock order in D-BV39.
- `onOrgMoveItemResult` and `onClearOrgVaultInventory` are not used (D-BV40): a refusal sends a line and snaps back, as the personal vault does.
- 19 and 20 stay `No` in `player_movable`; `moveItem` routes org moves away before the allowlist runs.
- World entry still declares 19 and 20 at 100; the open re-sends `onBagInfo` with the Team's real size ([Known gaps](handoffs/session-resume.md#known-gaps-carried-forward)).

Scope:

- Org-owned item storage. `sgw_inventory.character_id` is `NOT NULL` (A-23), so decide the table shape with `database-persistence`. Its FK to `sgw_organizations` must not cascade-delete (D-BV18).
- Declare 19 and 20 in `onBagInfo`, and send org vault contents to members.
- Team and Command Banker arms that send 107 and 108, gated on org membership.
- Deposits and withdrawals under ORG-LOCK (`lock_org`, then `member_access_locked`, then items), with the `DepositBank` and `WithdrawBank` bits.
- A bank log table.
- Replace cimmeria-1f's stubs `org_vault_is_empty(tx, org_id)` and `org_vault_is_empty_sql(org_id)`.
- Carry the org scope through `VaultAccess::Open { scope }` (D-BV24), and give 19 and 20 their own arms in `player_movable` and `container_policy`.
- Fan out with `broadcast_to_org`.

Telemetry: emit `org_vault_opened`, `org_vault_open_rejected`, `org_move_accepted` and `org_move_rejected` as the catalog specifies, with the correlators and org fields. Rows in the vault log table carry the same `account_id`, `player_id` and `org_id`. A `LogCapture` test per event and per reason.

## BV-08 org cash

**Status: Done** (PR #963, `acb43359c`). Decision D-BV15, plus D-BV41 to D-BV44. Worknote: [bv-08.md](worknotes/bv-08.md).

As built, against the scope below:

- The lock order is D-BV39's, not "org, then player": the actor's `sgw_player` row `FOR KEY SHARE`, then `lock_org`, then `member_access_locked`, then the wallet as a plain `UPDATE`, then the treasury `UPDATE`, all in one transaction.
- Transfers are logged in a sibling table, `sgw_organization_cash_log` (D-BV41), which BV-09 also writes.
- There is no Banker or vault-session check (D-BV42). A zero amount is refused on the cell (D-BV43). `no_such_org` and `not_a_member` share one line (D-BV44).

Scope:

- Replace the reject-with-feedback base arm for `OrgCellToBase::TransferCash`. The sign of the amount picks the direction.
- `DepositCash` and `WithdrawCash` are separate bits.
- No overflow, and no negative balance (`sgw_organizations.cash CHECK >= 0`).
- One transaction under ORG-LOCK, with the lock order org, then player.
- Log each transfer.
- Broadcast `onOrganizationCashUpdate` (built by `build_on_organization_cash_update`) after the commit.

Telemetry: emit `org_cash_transfer` and `org_cash_rejected` as the catalog specifies. A `LogCapture` test per event and per reason.

## BV-09 Team vault expansion

**Status: Done** (PR #966, `6d7d43149`). Decision D-BV28 (owner, 2026-09-27): a +10 step costs 100 naquadah, the D-BV02 price, paid from the org treasury (`sgw_organizations.cash`). Only the leader may buy it; there is no new permission bit. Also D-BV45 to D-BV47. Worknote: [bv-09.md](worknotes/bv-09.md).

As built: the trigger is the GM-only `.orgvaultexpand [team|command] [from_slots]`, which quotes with no size and buys keyed on the current size (D-BV45). It needs no vault session (D-BV46). A Command vault is refused (`command_vault_fixed`). Only the buyer gets the new `onBagInfo`; other members' open Team windows keep the old size until they reopen them ([Known gaps](handoffs/session-resume.md#known-gaps-carried-forward)). There is no player trigger until the Expand dialog's quarantine lifts (D-BV35).

Telemetry: emit `org_cash_transfer` for the payment and `expand`/`expand_rejected` with the org fields added. A `LogCapture` test per event.

## BV-10a debug-hub Team and Command Bankers

**Status: Done** (PR #960, `07f27ce19`). Split out of BV-10 by the coordinator. Templates 371 (Team) and 372 (Command), `INT_Banker` with `vault_scope` `team` / `command`, spawned as 471 and 472 in a second row off the stasis room's B-C wall. Both show "Storage Officer" (no shipped moniker names a Team or Command banker, and the `name` column never reaches the client), so the bodies tell them apart: the Cellblock guard uniform for 371, plain crew clothes for 372. The cross-track smoke `bank_org_round_trip_tests::debug_hub_org_bankers_open_the_members_vault_and_refuse_others` drives both from the real seed rows. Worknote: [bv-10a.md](worknotes/bv-10a.md); tester setup: [debug-hub.md](../../content/debug-hub.md#team-and-command-bankers-templates-371-and-372).

## BV-10 org close-out and release 2

**Status: Review** (this PR, branch `docs/bank-vault-bv10-closeout`, docs only). Update the docs and `docs/gameplay/organization-system.md`, extend the UAT checklist, and post `/release` (D-BV11) once it merges.

As built: the ledger, the decisions D-BV36 to D-BV47, the [UAT checklist](handoffs/session-resume.md#uat-checklist) steps 15 to 25 with their SigNoz queries, the bank section of [unified-uat.md](../../guides/unified-uat.md#bank-and-vault), the known gaps, `docs/gap-analysis.md` §12 and §23, `docs/project-status.md`, the two gameplay docs, and the `bank` rows of `observability.md`.

Telemetry: extend `observability.md` and the UAT checklist queries for the org events. Check that every org catalog event has its guard.
