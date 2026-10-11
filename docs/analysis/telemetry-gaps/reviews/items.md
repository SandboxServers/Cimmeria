# Items telemetry review

> Reviewer: items-systems-advisor. Data: colo SigNoz, last 7 days (`cimmeria-server`, `cimmeria-client`). Code read at `tg-ledger` HEAD. All paths are under `crates/`.

- Item mutations that succeed are well covered (grant, bank, vendor, native consumables carry `event=`, ids and names). The weak spots are the refusals and silent drops around them, and the appearance and loot-roll seams.
- Most important: `refresh_player_appearance` has no success or failure row, and on a DB error it broadcasts and caches the default "naked human male" model (section 3).
- No `OTEL_FILTER` change is needed by any packet. New rows must stay on module paths or the pinned `abilities`/`bank` targets: `inventory`, `vendor` are pinned at `info` only, and `loot`, `bandolier`, `bandolier.resend`, `loot.drop` are not named at all (default `info`), so a DEBUG row on those targets never reaches SigNoz.

## 1. Inventory

Pinned (reach SigNoz): `inventory`=info, `vendor`=info, `bank`=debug, `ammo`=debug, `abilities`=debug, `crafting`=debug, `playtest.friction` (WARN floor), and every `cimmeria_base_methods`/`cimmeria_cell_*` module path at DEBUG.

| Area | Rows | Fired in 7 days (colo) |
|---|---|---|
| Grant | `inventory event=grant_container_chosen` (INFO), `grant_refused`, `lookup_failed`, `grant_outcome_unknown`, `loot_restore_*`, `content_grant_mail*` | 122 chosen; **no refusal/failure row ever** |
| Move | `inventory.move_item` span, DEBUG "Inventory move persisted" (`after_commit.rs:80`), `bank event=move_accepted/move_rejected` | 73 persisted; 32 bank accepted; 0 rejected |
| Use | cell `useItem` INFO, base "firing ItemUsed" INFO, `consumable_*` events | 11 / 9 / 5 used, 4 skipped; 0 refused |
| Loot | `loot.drop event=skipped` (INFO), `loot generated` and `loot_interaction_set` (DEBUG, target `abilities`), "Player looted item" INFO, `loot.container_*` | 746 skipped, 151 generated, 17 interaction set, 60 looted, 6 container opens |
| Vendor | `vendor event=store_opened/transaction/refused/failed` | **6 opens, 0 transactions** |
| Bandolier | `bandolier event=weapon_ability_swap/active_slot_change/ammo_type_change`, "equip-display decision", holster refresh | 70 / 11 / 0 / 35+4 / 170 |
| Appearance | "Loaded player data for mapLoaded" + "Player load data: final appearance" (INFO) | 382 each, no trigger label |

Never observed in 7 days (so unverified in telemetry, not necessarily broken): every vendor buy/sell/repair/recharge outcome, every `grant_refused`, every `ammo_type_change`, every `bank move_rejected`.

## 2. Positive gaps

- **Appearance refresh has no row of its own.** `inventory/appearance.rs:39-137` logs only two DEBUG skips (`:53`, `:68`); a refresh caused by an equip, a move, a grant or a holster is visible only through the generic "Loaded player data for mapLoaded" INFO at `player_load/core/player_data.rs:74`, which is mislabelled (it runs for every refresh, not only map load) and cannot say why it ran or how many witnesses got it.
- **Move success row is thin.** `move_/after_commit.rs:80` has `item_id`, `total_items`, no source/target container, no shape (whole/split/merge/swap), no quantity. A duplicate or vanished-stack report cannot be reconstructed.
- **Remove success row has no before/after.** `core/remove_instance.rs:307` and `core/remove_by_type.rs` log `quantity`, not `stack_before/after`, `removed_all` or whether the call was a drop, a GM remove, a chain `remove_item` or a native consume. The double-consume trap (`UseInventoryItem` + `remove_item` on a stack above 1) is therefore visible only as two unlabelled rows.
- **Loot roll with no outcome.** `abilities/loot_drop.rs:37` returns silently when the mob has no table; `:57` rolls and logs only the entries that drop. Colo: 773 `target_killed`, 746 NPC-only, so about 27 player kills, 17 `loot_interaction_set`. The other ~10 cannot be told apart (no table, table missing, every roll failed).
- **Free repair/recharge is untracked.** `vendor/repair.rs:123-232` and `recharge.rs:23-198` (the `vendor_template_id = None` path) log only DEBUG, without `target: "vendor"`, `action`, `account_id` or cash. The paid paths use `VendorLog`.
- **Rule 5/6 misses (confirmed on real rows).** `item_sequence.rs:26` (`fire_item_sequence`, INFO, 169 rows) has no `player_id`/`account_id` and Debug-formats `archetype_id`, `event_set_id`, `seq_id` (SigNoz stores `Some(1873)` as a string). `weapon_abilities.rs:124` and `player_init/mod.rs:50` Debug-format `item_id` (`Some(55)`/`None`) and carry no player identity. Cell entry rows `item_ops.rs:37,85,129,197` (`removeItem`, `listItems`, `moveItem`, `useItem`) carry `entity_name` only, no `player_id`/`account_id`.

## 3. Negative gaps

- **Appearance refresh trusts a failed load.** `appearance.rs:80` takes `query_player_load_data` as always valid. On `Err`/`Ok(None)` it returns `default_player_load_data()` (`player_load/meta.rs:15`, `player_id: 0`, bodyset `BS_HumanMale`, no components) after one ERROR with no account (`player_data.rs:227`). The refresh then caches it in `cached_appearance_args` (`appearance.rs:96-105`) and sends it to the player and every witness (`:107-136`). Late joiners get the cached default too.
- **Silent swallow after a move.** `after_commit.rs:148-156` is `.ok().flatten().unwrap_or(false)`: a DB error on the `visual_component` lookup skips the appearance refresh with no row (armor persists, model stays stale). `:92` is `let _ = cell_tx.send(InventoryItemMoveApplied)`: a closed channel loses the `OnItemEquipped` content event and the bandolier sync trigger with no row.
- **Inline move refusals are quiet, event-less WARNs** with no source container and no snap-back: invalid slot (`move_/mod.rs:278`), quantity over stack (`:448`), item not allowed in container (`finish.rs:60`, vault moves excepted), split onto occupied (`finish.rs:388`). Only the bank path emits `event=move_rejected reason=` and resyncs the dragged item. Same-slot no-op (`mod.rs:460`) and "no DB pool" (`:151`, DEBUG) are silent by design.
- **Stale ammo writeback is DEBUG without counts.** `ammo.rs:46-55` (`rows_affected == 0`: slot empty or instance swapped, the TOCTOU guard) logs at DEBUG with no `rows_affected`/`expected`. This is exactly the data-loss shape of PR #520, and the negative-logging convention's Pattern B asks for WARN plus both counts.
- **Logout/transfer bandolier flush is blind.** `active_slot.rs:20-81` has no summary row; a dirty slot with no item is dropped without a trace (`:43-47`); a failed send breaks the loop with one WARN and no count of what was left.
- **Loot silent drops.** `loot/mod.rs:186-188` returns after `list.remove(i)` already took the item off the corpse (item lost, no row). `:267` is `let _ = tx.send(GrantCash)` (the cash was already removed from the list; items get `return_unsent`, cash does not). `:48` drops the `onLootDisplay` send. `cell-content/.../executor/loot.rs:227` returns silently when the container vanished after a roll, with no feedback line. `abilities/loot_drop.rs:44` reports a loot table id with no rows (a content fault) at DEBUG.
- **Outbox rows are unlinkable.** `base-session/.../outbox/mod.rs:296,303,377,401,409` carry only `outbox_id`: no `entity_id`, no `event_type`. A drainer replay that succeeds logs nothing, and the drainer selects every undelivered row (`:357`), including one `try_dispatch_now` is still about to send. For `ItemUsed` a duplicate delivery re-runs the chain, so a chain with `remove_item` and a stack above 1 consumes a second unit. Nothing in SigNoz shows that a replay happened.
- **Dropped-on-purpose-looking returns that need a reason:** `item_ops.rs:301-312` (`repairItemRequest` is "UNIMPLEMENTED" at INFO, and its truncated-args branch is silent); `use_instance.rs:103-114` and `remove_instance.rs:102-113` ("no DB pool" at DEBUG).

## 4. Noise

- `loot.drop event=skipped` NPC-only kill: 746 INFO rows/week, 10x the player loot rows. Keep (it answers "why no loot") but it is the loudest item row; DEBUG would be defensible.
- `persist.rs:305,319` logs "container full" at WARN, then `grant_item.rs:412` logs `grant_refused` at INFO for the same event. A full bag is a player condition: one row, not two, and not WARN.
- `use_instance.rs:175` WARNs "instance not found" on a double-click of a consumable's last unit (the first click already consumed it; `consume_for_use.rs` documents this). Expected, so INFO with a reason.
- `grant_refused` is INFO for every reason, including `DatabaseError` ("the inventory write failed"): a lost mission reward is not an INFO.
- `client.entity.appearance_request`: 83,220 client INFO rows/week from 34 sessions, about 8 rows per entity (1,184 type-4 entities carry 10k rows per step name). Informational for the appearance seam; a rollup belongs to the client-DLL owners (T9 family).

## 5. Seams (two hops)

| Neighbour | Hand-off | Both sides log enough? |
|---|---|---|
| Combat (ammo, weapon abilities) | `BandolierAmmoUpdate`, ammo-reserve requests, `ammo_consumed` | Cell logs (`ammo`, `bandolier`); the base stale guard does not (ITM-06). Combat to damage to NPC death to loot is a hop: `target_killed` then the silent loot roll (ITM-02). |
| Missions/content | `useItem` to outbox to `ItemUsed` to `fire_item_use` to chain to `RemoveInventoryItem*`/`GrantItem` | Cell and base log separately; **no `chain_id` crosses the channel**, so a base row joins its chain only by player and time. Replays invisible (ITM-03). Mission neighbour: dialog and rewards go through the same `GrantItem`. |
| Persistence | write-through `sgw_inventory`; outbox; dirty-ammo flush at logout/transfer | Grants and moves log commits and errors. The flush does not (ITM-11). |
| Crafting | transaction consume/grant (`transaction/consume.rs`, `grant.rs`, no tracing) and `AfterFullInventoryUpdate` hook | `crafting` target covers the outcome (`completed`); no `inventory`-target row, so a player's item timeline from `inventory` events misses crafting input consumption. |
| Black market, mail, trade | direct `sgw_inventory` DELETE/INSERT in `black_market/escrow.rs`, `mail/send/escrow.rs`, `mail/take.rs`, `trade/execute/swap.rs`, `character_create/starter_kit.rs` | Each logs under its own target; mail and bm are fine, trade is DEBUG-only. No common "inventory changed" row exists across the 27 writer files. |
| Appearance broadcast to AoI | `refresh_player_appearance` to `send_to_witness_reliable` + `broadcast_to_witnesses` to witnesses to client `appearance_request` | Item side silent on success (ITM-01); AoI reviewer owns witness delivery. Client rows exist but are 83k/week of per-step noise. |
| NPC AI/spawns | `loot_table_id` on the spawn, `Loaded loot tables` at startup | Startup row exists; a mob pointing at a missing table is DEBUG only (ITM-02). |
| Client | `onUpdateItem`, `onRemoveItem`, `onLootDisplay` | No client-side inventory event reached SigNoz in 7 days (only `appearance_request`); T1 (bundle drops) covers delivery. A "server committed, client never showed it" report cannot be settled from SigNoz. |

## 6. Adversarial

1. **"I killed it and nothing dropped" (player-hit).** Today: `target_killed` INFO, then nothing, or a DEBUG `loot_interaction_set`. Cannot tell no table, missing table, or failed rolls. Should: one `loot_rolled` row with `loot_table_id`, `entries`, `dropped`, and a WARN when the table id has no rows.
2. **"Dragged armor on, model unchanged / I turned into a default human."** Today: if the DB read in `query_player_load_data` fails, one ERROR with no cause and no account; the default model is broadcast and cached; the `.ok()` at `after_commit.rs:156` hides the second query. Should: `appearance_refresh_aborted` with `cause` and account, and no broadcast.
3. **"My medkit stack lost two."** Today: `Content: RemoveItem` INFO, "Inventory remove persisted" DEBUG twice, indistinguishable from two separate drops; a drainer replay leaves no trace. Should: `origin`, `stack_before/after` on the remove row, and `outbox_replayed` rows.
4. **"I dragged an item and it sits in the wrong bag on screen."** Today: a WARN with no source container and no event; no snap-back. Should: `event=move_rejected reason=invalid_slot|over_stack|not_allowed|split_onto_occupied` with both containers.
5. **"My ammo count reverted after relog."** Today: nothing (stale guard at DEBUG, no flush summary). Should: `ammo_writeback_stale` WARN with `rows_affected=0 expected=1`, and a flush summary with sent/dropped/unsent.
6. **Vendor free repair.** Today: DEBUG "Inventory items repaired". Should: a `vendor event=transaction action=repair price=0`. Handoff below: this path may itself be a hole.

## Candidate packets

| ID | Title | Sev | Files | Notes |
|---|---|---|---|---|
| ITM-01 | Appearance refresh: result row, abort on load failure | high | 1 | behaviour change (no default broadcast) |
| ITM-02 | Loot roll outcome row, loot table fault to WARN | high | 1 | |
| ITM-03 | Outbox: replay row and identity on every outbox WARN | med | 1 | live-DB test |
| ITM-04 | Inline move refusals: `event`, `reason`, containers | med | 2 | |
| ITM-05 | After-commit: no silent drops, richer move row | med | 1 | |
| ITM-06 | Stale bandolier ammo writeback to WARN with counts | med | 1 | live-DB test |
| ITM-07 | Free vendor repair/recharge through `VendorLog` | med | 2 | |
| ITM-08 | Numeric ids and identity on bandolier and item_sequence rows | med | 3 | |
| ITM-09 | Loot silent drops and lost cash | med | 2 | |
| ITM-10 | Remove rows: origin, stack before/after | med | 2 | |
| ITM-11 | Bandolier flush summary, dropped slot trace | low | 1 | |
| ITM-12 | Grant refusal levels and duplicate full-bag row | low | 2 | |
| ITM-13 | Double-click `useItem` level; identity on cell entry rows | low | 2 | |

### ITM-01 Appearance refresh result row (high)
- Files: `base-methods/src/base/world_entry/methods/inventory/appearance.rs` (`refresh_player_appearance`).
- After `query_player_load_data` (`:80`), if `player_data.player_id != player_id` (the default has 0): `tracing::error!(event = "appearance_refresh_aborted", cause = "player_load_failed", entity_id, player_id, player_name, account_id, account_name)`, return before the cache write and both sends.
- On success add `tracing::debug!(event = "appearance_refreshed", entity_id, player_id, player_name, account_id, account_name, holstered, component_count, bodyset)`. Turn the two skip rows (`:53`, `:68`) into `event = "appearance_refresh_skipped", reason = "no_addr" | "no_client_state"` (still DEBUG; a disconnect is normal).
- Test: unit with `LogCapture`, `TestTransport` and the existing `make_connected_state`, `db_pool = None`. Assert one ERROR with the event and `cached_appearance_args` still `None`, and no packet sent. Reverting the guard caches the default and sends, so both assertions fail.
- Pin: none (module path is DEBUG-exported).

### ITM-02 Loot roll outcome (high)
- Files: `cell-combat/src/cell/abilities/loot_drop.rs` (`generate_loot_on_death`).
- At `:37` log `event = "loot_skipped", reason = "no_loot_table", target_eid` (DEBUG, target `abilities`). At the end of the roll log `event = "loot_rolled", target_eid, loot_table_id, loot_table_name, entries, dropped`. Raise `loot_table_empty` (`:44`) to WARN with `reason = "table_has_no_rows"`; make the `:59-62` early return log `reason = "target_gone"`.
- Test: `LogCapture` unit test in the file's `tests`: probability 0 table gives `loot_rolled` with `dropped = 0`; an unknown table id gives the WARN. Reverting removes the rows.
- Pin: none (`abilities=debug`).

### ITM-03 Outbox replay and identity (med)
- Files: `base-session/src/base/outbox/mod.rs`.
- Add `entity_id` and `event_type` (derive a `&'static str` from the payload before the match in `try_dispatch_now`; the drainer already has them in `OutboxRow`) to the five WARN rows. In `drain_undelivered` success (`:399`) log `tracing::info!(event = "outbox_replayed", outbox_id, entity_id, event_type, attempts)` (the drainer only picks undelivered rows, so a healthy run emits none).
- Test: live-DB (`require_db_or_skip!`, `live_db` in the name) in the outbox tests: enqueue an `ItemUsed` row, call `drain_undelivered`, assert the INFO row carries `event_type = "item_used"`. Reverting removes the row.
- Follow-up for a domain agent, not this packet: the drainer should not pick rows younger than a few seconds.

### ITM-04 Inline move refusals (med)
- Files: `base-methods/.../inventory/move_/mod.rs` (`:278`, `:375`, `:448`), `move_/finish.rs` (`:60`, `:388`).
- Keep each WARN; add `target: "inventory", event = "move_rejected", reason = "invalid_slot" | "source_not_found" | "over_stack" | "not_allowed_in_container" | "split_onto_occupied"`, plus `source_container_id`, `source_container_name` where the source row is known, `account_id`. Keep the vault branches untouched.
- Test: extend `move_/named_log_tests.rs` (LogCapture, live DB where the existing ones need it): one case per reason asserting `event` and `reason`. Reverting drops the fields.

### ITM-05 After-commit drops (med)
- Files: `move_/after_commit.rs`.
- Replace `let _ = cell_tx.send` (`:92`) with a match logging WARN `event = "move_cell_notify_failed", reason = "cell_channel_closed"`. Replace `.ok().flatten().unwrap_or(false)` (`:154`) with a match: on `Err`, ERROR `event = "move_appearance_lookup_failed"` and treat as no refresh. Extend the DEBUG row at `:80` with `source_container_id`, `target_container_id`, `swapped_item_id`, `source_deleted`.
- Test: live-DB + LogCapture in `refusal_resync_tests.rs` style: drop the cell receiver, assert the WARN; assert the extra fields on the persisted row. Reverting loses both.

### ITM-06 Stale ammo writeback (med)
- Files: `base-methods/.../inventory/ammo.rs` (`:46-55`).
- Make it WARN with `event = "ammo_writeback_stale"`, `rows_affected = 0`, `expected = 1`, `reason = "slot_empty_or_instance_swapped"`, `account_id` if cheaply known. If lab shows more than one row/min in steady state, demote to INFO and rate-limit (note in PR).
- Test: the existing live-DB tests (stale-instance case) gain a LogCapture assertion of the WARN and fields. Reverting to DEBUG fails the level assertion.

### ITM-07 Free vendor repair/recharge (med)
- Files: `vendor/repair.rs` (`handle_repair_inventory_items`), `vendor/recharge.rs`.
- Build a `VendorLog::new("repair"|"recharge", ...)` with `vendor_template_id = None`; call `completed(item, lines, None, None)` when rows were changed, `refused("nothing_to_do", ...)` when none, `failed("db_error", ...)` on errors (replace the existing ERRORs' fields, keep their messages).
- Test: live-DB (existing sentinel tests in both files) with LogCapture asserting `target == "vendor"`, `event = "transaction"`, `action`. Reverting leaves no `vendor` row.

### ITM-08 Numeric ids and identity (med)
- Files: `cell-combat/.../world/item_sequence.rs:26`, `cell-combat/.../bandolier/weapon_abilities.rs:118`, `cell/.../player_init/mod.rs:43`.
- Replace `?archetype_id`, `?event_set`, `?seq_id`, `?item_id`, `?removed`, `?added` with numeric `Option` fields (`archetype_id = archetype_id` etc.) and add `account_id`, `account_name`, `player_id`, `player_name` from `space_mgr.player_identity(entity_id)`. Set `event = "item_sequence_lookup"` on the first.
- Test: `LogCapture` unit per row asserting `item_id` equals `"55"` not `"Some(55)"` and `player_id` present. Reverting yields the string form.

### ITM-09 Loot silent drops (med)
- Files: `cell-interactions/.../loot/mod.rs`, `cell-content/.../executor/loot.rs`.
- `loot/mod.rs:186`: WARN `event = "loot_item_lost", reason = "corpse_gone"` with item, qty, index. `:267`: match the send; on error WARN `event = "loot_cash_lost", reason = "base_channel_closed", amount`. `:48`: WARN on send failure. `executor/loot.rs:227`: log `reason = "container_gone"` through the existing `refuse` closure and send the feedback line.
- Test: unit `LogCapture` in `loot/tests.rs`: close the `tx` receiver, loot cash, assert the WARN. Reverting leaves `let _ =`.

### ITM-10 Remove origin and stack counts (med)
- Files: `core/remove_instance.rs` (`:307`), `core/remove_by_type.rs` (persisted row).
- Add `stack_before`, `stack_after`, `removed_all`, `origin` (`"consume_for_use"` for `AccessOp::Use`, `"gm"` if `notify_gm`, else `"remove"`; by-type is `"remove_by_type"`), `container_id`, `container_name`.
- Test: live-DB LogCapture: partial remove gives `stack_before = 3, stack_after = 2`. Reverting drops the fields.

### ITM-11 Bandolier flush summary (low)
- Files: `cell-combat/.../bandolier/active_slot.rs` (`flush_dirty_bandolier_ammo`).
- Log WARN `event = "bandolier_slot_dropped", reason = "no_item"` at `:43`; at the end DEBUG `event = "bandolier_flush", sent, dropped, unsent`. Include `account_id`, `player_name`.
- Test: unit with a dirty slot without an item and a closed channel; assert both rows. Reverting removes them.

### ITM-12 Grant refusal levels (low)
- Files: `grant/persist.rs` (`:305`, `:319`), `grant/grant_item.rs` (`:412`).
- Demote the two "container full" WARNs to DEBUG (the `grant_refused` row carries the fact); in `grant_item.rs` emit `grant_refused` at WARN when `reason` is `DatabaseError` or `NotGrantable`, INFO otherwise.
- Test: extend `full_bag_tests.rs`: full bag gives exactly one INFO `grant_refused` and no WARN. Reverting brings the WARN back.

### ITM-13 Double-click level and entry identity (low)
- Files: `core/use_instance.rs:175`, `cell-methods/.../inventory/item_ops.rs`.
- `use_instance.rs`: INFO `event = "use_refused", reason = "instance_gone"` with `account_id`. `item_ops.rs`: add `event` and `player_id`/`account_id` to the `removeItem`, `moveItem`, `useItem`, `listItems` rows (resolve once with `space_mgr.player_identity`); give the truncated-args branch of `repairItemRequest` a WARN.
- Test: LogCapture unit in `use_instance_tests.rs` and `tests/use_item.rs`. Reverting restores the WARN level.

## Handoffs

- **server-authority-enforcer:** `cell-methods/.../vendor/mod.rs:124-138` treats a missing trailing template id on `repairItems`/`rechargeItems` as "free repair" (`vendor_template_id: None`, `repair.rs:133`). A client that omits the trailing id with an open vendor session gets a free repair. Verify whether the stock client ever omits it before ITM-07 makes it look intentional.
- **rust-gameserver-dev:** non-vault move refusals have no snap-back (only the bank path resyncs the dragged item); behaviour change, not telemetry.
- **database-persistence / mission-systems-advisor:** the drainer picks rows `try_dispatch_now` is still sending (`outbox/mod.rs:357`), and no `chain_id` rides `RemoveInventoryItem*`/`GrantItem`; both are cross-system changes.
- **aoi-witness-broadcast:** owns witness delivery after `broadcast_to_witnesses` and the 83k/week `client.entity.appearance_request` volume.
- **combat-systems-advisor:** ITM-02 touches the kill-to-loot hop; ITM-06 and ITM-11 touch ammo persistence.
