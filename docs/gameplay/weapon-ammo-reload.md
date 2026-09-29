---
title: "Weapon Ammo & Reload"
type: reference
audience: engineers
last_updated: 2026-09-28
---

# Weapon Ammo & Reload

> **Last updated**: 2026-09-28
> **Status**: Implemented (player fire/reload, bandolier persistence, and finite special ammo from the [ammo campaign](../analysis/ammo/README.md), #1026)

## Overview

Ammo is **server-authoritative**. The cell server is the single source of truth for how many rounds each bandolier slot has, what subtype is loaded, and whether a reload is in progress. The client only renders — it does not predict ammo locally and never decrements its own counter.

Concretely, the server:

1. Validates fires against the active slot's current ammo on every `useAbility`.
2. Decrements `BandolierItem.current_ammo` and mirrors it to `Stat[AMMO_SLOT_1+slot]`.
3. Drives reloads on its own clock (warmup deadline) — the client's reload animation is purely cosmetic and is not trusted.
4. Pushes `onStatUpdate` (method 20) to the attacker after every fire/reload so the bandolier UI re-reads the ammo stat and refreshes the meter and count text.

Persistence is **batched**: dirty slots flush at natural drain points (reload completion, slot swap, ammo change, logout, world transition) rather than on every fire. The trade-off is documented under [Persistence cadence](#persistence-cadence).

## Per-slot ammo storage

Per-slot ammo lives in **two mirrored places** on the cell entity, both written through `set_slot_ammo()`:

```text
                     CellEntity
   ┌─────────────────────────────────────────────┐
   │  bandolier_items: HashMap<i32, Bandolier… >│
   │    [0] BandolierItem {                      │
   │          item_id, clip_size,                │
   │          current_ammo,    ◄── canonical     │
   │          cur_ammo_type,   ◄── canonical     │
   │          default_ammo_type                  │
   │        }                                    │
   │    [1] …                                    │
   │                                             │
   │  stats: StatList                            │
   │    AMMO_SLOT_1 (49)  ◄── mirror of [0]     │
   │    AMMO_SLOT_2 (50)  ◄── mirror of [1]     │
   │    AMMO_SLOT_3 (51)                        │
   │    AMMO_SLOT_4 (52)                        │
   │    AMMO_SLOT_5 (53)                        │
   │                                             │
   │  active_bandolier_slot: i32                │
   │  bandolier_ammo_dirty: HashSet<i32>        │
   │  reload_complete_at: Option<Instant>       │
   └─────────────────────────────────────────────┘
```

`BandolierItem` is the source of truth. `Stat[AMMO_SLOT_1+slot]` exists because the **client's UI subscribes to `Events.StatUpdated`** — it would not see a change to `BandolierItem` directly. Both are kept in sync through the single mutator [`set_slot_ammo()`](../../crates/entity/src/cell_entity/bandolier.rs#L36); never write `current_ammo` directly. See [`CellEntity::active_ammo()`](../../crates/entity/src/cell_entity/bandolier.rs#L12), [`active_clip_size()`](../../crates/entity/src/cell_entity/bandolier.rs#L19), [`active_ammo_type()`](../../crates/entity/src/cell_entity/bandolier.rs#L26), and [`refill_active_slot()`](../../crates/entity/src/cell_entity/bandolier.rs#L50).

Stat IDs `AMMO_SLOT_1..5` (49–53) are **bandolier-slot-relative**, not weapon-relative. The active slot's stat ID is computed as `AMMO_SLOT_1 + active_bandolier_slot`. This matches legacy [`SGWPlayer.py:1023`](../../deprecated/python/cell/SGWPlayer.py#L1023) (`getAmmoStat() = ammoSlot1 + activeSlotId`).

## Wire flow — fire

```text
Client                                         Cell                        Base
  │                                              │
  │ useAbility(abilityId, targetId) ───────────▶ │
  │                                              │ handle_use_ability:
  │                                              │   required = ability_def.required_ammo
  │                                              │   if required > 0 && is_player:
  │                                              │     if active_ammo() < required:
  │                                              │       log "useAbility: not enough ammo"
  │                                              │       return  (fire aborts)
  │                                              │   set_slot_ammo(active_slot, ammo - required)
  │                                              │     ↳ marks bandolier_ammo_dirty
  │                                              │     ↳ Stat[AMMO_SLOT_1+slot].set_current(...)
  │                                              │   stats.serialize_dirty()
  │                                              │   stats.clear_dirty()
  │                                              │
  │ ◀─── onStatUpdate (method 20) ────────────── │
  │       AmmoSlot{N}: { min, cur, max }         │
  │                                              │
  │ Lua: Events.StatUpdated fires →              │
  │   BandolierMod.onStatUpdated(unitId, statId, current, max) │
  │   → refreshSlotAmmo(slotIndex):              │
  │       setMaterialScalarProperty(             │
  │         'CoreMaterial_AmmoMeter'..slotID,    │
  │         'M_Percentage', current/max)         │
  │       count text → string.format("%d", …)    │
```

The fire-gate skips the ammo check entirely for non-players (`entity.is_player == false`), so NPC mobs do not consume rounds — see [NPC ammo](#npc-ammo).

Implementation: [`crates/cell-combat/src/cell/abilities/mod.rs:259-281`](../../crates/cell-combat/src/cell/abilities/mod.rs#L259) for the gate and consume; the `onStatUpdate` is dispatched from the post-resolve drain at [`abilities.rs:511-515`](../../crates/cell-combat/src/cell/abilities/mod.rs#L511).

## Wire flow — reload

`requestReload(EReloadType)` is a Mercury **cell method** on `SGWPlayer` (def: [`entities/defs/SGWPlayer.def:794-797`](../../entities/defs/SGWPlayer.def#L794), wire opcode 86 / `0x56` — defined as `REQUEST_RELOAD` in [`crates/wire/src/cell/cell_methods/player/constants.rs`](../../crates/wire/src/cell/cell_methods/player/constants.rs); the `0x14` value in the [decompiled client binding](../reverse-engineering/decompiled/14_standalone_named.c#L298900) is a registration index, not the wire opcode).

```text
Client                       Cell                                       Base / DB
  │ requestReload(0) ────────▶│
  │                           │ handle_reload (player/world.rs:121):
  │                           │   if active_ammo() >= active_clip_size(): return
  │                           │   warmup  = ability_defs[596].warmup   (e.g. 2.0s)
  │                           │   cooldown = ability_defs[596].cooldown
  │                           │   reload_complete_at = now + warmup
  │                           │   reload_slot_id = Some(active_bandolier_slot)  ← pin
  │                           │   abilities.start_ability_cooldown(596, warmup+cooldown)
  │ ◀── onTimerUpdate (m12) ──│  cooldown bar starts
  │ ◀── onEntityProperty (m7)│  ammo-type sync (cur_ammo_type)
  │                           │
  │  Note: handle_reload does NOT touch bStateField — BSF_IN_COMBAT is
  │  derived from `threatened_mobs` and only flips via combat::generate_threat.
  │  A BeingAppearance rebroadcast only fires on the reload-while-holstered
  │  Phase A path (request_appearance_refresh draws the weapon, the actual
  │  reload defers by UNHOLSTER_DRAW_DURATION).
  │                           │
  │  (warmup elapses, e.g. 2 s later)
  │                           │
  │                           │ reload_completion_tick (every 100 ms):
  │                           │   for each player where now >= reload_complete_at:
  │                           │     slot = reload_slot_id              ← refills the
  │                           │     set_slot_ammo(slot, clip_size)     ←  PINNED slot,
  │                           │                                        ←  not active!
  │                           │     reload_complete_at = None
  │                           │     reload_slot_id     = None
  │                           │     stats.serialize_dirty() → onStatUpdate
  │ ◀── onStatUpdate (m20) ───│  AmmoSlot{N}: cur=clip_size  (N = pinned slot)
  │                           │ ──── BandolierAmmoUpdate ─────────▶│ UPDATE sgw_inventory
  │                           │      { player_id, slot_id,        │   SET ammo = …,
  │                           │        expected_item_id,          │       cur_ammo_type = …
  │                           │        current_ammo, cur_ammo_type}│   WHERE … type_id = …
```

The fire-path **only** reads `active_ammo()`; it does not promote pending refills itself. The 100 ms `reload_completion_tick` is the sole refill path (Stage C cleanup — see [`crates/cell/src/cell/service/ticks/reload_completion.rs:24`](../../crates/cell/src/cell/service/ticks/reload_completion.rs#L24)).

Matches legacy [`Reload.py`](../../deprecated/python/cell/effects/Reload.py): the effect resolves at warmup completion and runs `setCurrent(max)` on the ammo stat. Legacy ammo consumption was at warmup completion ([`AbilityManager.py:669-670`](../../deprecated/python/cell/AbilityManager.py#L669)); the Rust port consumes at fire-gate time instead, since there is no warmup state machine for typical fires.

The flow above is the **default-ammo** reload (`Bullet_Default`, `Dart_Default`, `Dagger_Default`): it is free, and the completion tick refills to `clip_size`. A reload of a special type draws its rounds from the bags first; see [Special ammo](#special-ammo-finite-reserve).

## Special ammo (finite reserve)

The [ammo campaign](../analysis/ammo/README.md) (#1026) made the 15 special types finite. Default ammo stays free (D-AM02). Everything in this section runs only while the `ammo.finite_special` flag is on.

**The flag.** `ammo.finite_special` is **on by default** (D-AM11, since AM-12). `CIMMERIA_AMMO_FINITE_SPECIAL=0` on the server is the rollback lever: reloads go back to free refills and every shot fires unmodified. The lever does not withdraw the pushed ammo item definitions (9000-9014), the loot rows or the GM commands, so the stacks stay in your bags as ordinary items. The flag lives in [`crates/entity/src/ammo_feature.rs`](../../crates/entity/src/ammo_feature.rs); the env row is at the top of [`crates/server/src/main.rs`](../../crates/server/src/main.rs).

**The reserve is ordinary bag stacks** (D-AM01). Each special type is one stackable item, ids 9000-9014 (`ammo_items.sql`, 500 rounds a stack), mapped from its `EAmmoType` through `resources.ammo_item_types`. Nothing hardcodes an item id: the cell resolves `SpaceManager::ammo_catalog.item_id_for(ammo_type)`, and the base resolves the item in SQL. Only the main bag (1) and the crafting bag (15) count; the personal vault (17) never does. The `AmmoReserve` module ([`crates/base-methods/src/base/world_entry/methods/inventory/ammo_reserve/`](../../crates/base-methods/src/base/world_entry/methods/inventory/ammo_reserve/)) does the counting, drawing and returning inside one transaction.

**Rounds, never punish** (D-AM05). Clips and stacks both count individual rounds.

| Case | What happens |
|---|---|
| Reload, stack big enough | The cell flushes the clip, sends `ReloadDraw`, and starts no warmup yet. The base locks the weapon row, draws `clip_size - row.ammo` rounds (counted from the row, not the cell's number), and writes them into the row in the same commit. The answer loads the clip, then the ordinary warmup starts; the completion tick leaves that clip as it is. 18/30 with 100 in the bags gives 30/30. |
| Partial reload | The rounds already in the clip stay; only the gap is drawn. |
| Short stack | You get what is there: 18/30 with 5 in the bags gives 23/30, and the tick adds nothing. |
| Empty stack | Refused before any warmup: `onErrorCode(Ability, 596, 61)` (`CONDITION_FEEDBACK_AmmoCountLessThan`) plus a feedback line, "You have no Hollow Point rounds left." The clip is unchanged. |
| Second press in flight | Ignored; the base also counts from the locked row, so a duplicate request draws nothing. |
| Switch type (`requestAmmoChange`) with special rounds loaded | The unfired rounds go back to their bag stack (topping up existing stacks, then free slots) and the slot takes the new type with an empty clip. |
| Switch, bags full | The rounds that fit go back; the rest stay loaded **as the old type**, and the switch is refused with "Your bags are full: 7 Hollow Point rounds stay loaded. Make room and switch again." Nothing is deleted or relabelled. |
| Default clip switched to a special type | The free default rounds are emptied (they would otherwise fire as free special rounds). |

Loading at draw-commit time, not at warmup end, is the "never punish" guarantee: a slot swap, death or a cancelled warmup cannot lose rounds that already left the bags. Code: [`reload_reserve.rs`](../../crates/cell-combat/src/cell/cell_methods/player/world/reload_reserve.rs) and [`switch_return.rs`](../../crates/cell-combat/src/cell/cell_methods/inventory/bandolier/switch_return.rs) on the cell, `ammo_reserve/commits.rs` and `requests.rs` on the base; the wire is `CellToBaseMsg::AmmoReserve` / `BaseToCellMsg::AmmoReserve` ([`crates/wire/src/cell/messages/ammo_reserve.rs`](../../crates/wire/src/cell/messages/ammo_reserve.rs)).

**Infinite ammo** (D-AM09). `.infiniteammo` (alias `.gmsetinfiniteammo`, see [commands.md](../commands.md)) frees only the reserve: a special reload draws nothing from the bags, but the clip still empties and still needs a reload. The switch is per character and lasts until a server restart ([`crates/entity/src/ammo_infinite.rs`](../../crates/entity/src/ammo_infinite.rs)).

**Which weapons take special ammo.** The client's picker lists what the weapon's `ammo_types` column allows, and that list comes from the server-populated inventory cache, so widening a weapon needs no client patch (AM-01). The campaign widened:

| Family | Ids | Takes |
|---|---|---|
| Standard Pistol | 27 | the 5 bullet specials (D-AM10) |
| Standard SMG | 25 | the 5 bullet specials (D-AM10) |
| High Capacity SMG | 27 | the 5 bullet specials (D-AM10 amendment, #1052) |
| Dart guns | 19 | the 10 dart specials (AM-11a) |

What each type does to the shot is in [combat-system.md § Special ammo](combat-system.md#special-ammo-in-the-damage-pipeline) and [ADR § 31](../architecture/abilities-and-effects-system.md#31-special-ammo-modifies-the-shot-directly-from-resourcesammo_modifiers-ammo-campaign-am-04-d-am07).

## `requestAmmoChange` flow

```text
Client (player clicks an ammo subtype icon)
  │
  │ requestAmmoChange(weapon_instance_id, ammo_type) ──▶ Cell
  │                                              │ ammo_type <= 0          → refuse
  │                                              │ slot = bandolier slot whose instance id matches
  │                                              │   (none → refuse; two → refuse, corrupt state)
  │                                              │ WeaponDef for the slot's design id
  │                                              │   (missing → refuse, fail closed)
  │                                              │ ammo_type not in its ammo_types → refuse
  │                                              │   every refusal: one CHAN_FEEDBACK line + WARN
  │                                              │ special rounds loaded → switch return (above)
  │                                              │ item.cur_ammo_type = ammo_type
  │                                              │ ──── BandolierAmmoUpdate ─▶ DB (immediate)
  │                                              │
  │                                              │ if slot == active_bandolier_slot:
  │ ◀── onEntityProperty(AmmoTypeId, ammo_type) ─│   refreshes the ammo-type indicator
```

The client sends the weapon's **instance** id (`InvItem.id`, `sgw_inventory.item_id`), not its design id (#534). The handler matches the slot on the instance id and reads the whitelist by that slot's design id, never by the wire value. A type the weapon does not list is refused, and a weapon with no `WeaponDef` fails closed (#602, CAT-D-06). Each refusal sends exactly one feedback line ("That weapon cannot use that ammo type.", "That weapon cannot change ammo type.") and logs `ammo_type_change_rejected` with a `reason=` on target `ammo`. The legacy validator was literally `pass`; AM-03 (#1051) added the checks.

The persistence emit is **immediate**, not batched, because subtype is a deliberate user action and we want it durable before the next packet.

Def: [`entities/defs/interfaces/SGWInventoryManager.def:190-194`](../../entities/defs/interfaces/SGWInventoryManager.def#L190). Implementation: [`crates/cell-combat/src/cell/cell_methods/inventory/bandolier/ammo_change.rs`](../../crates/cell-combat/src/cell/cell_methods/inventory/bandolier/ammo_change.rs).

## Active slot swap

```text
Client → Cell:  requestActiveSlotChange(bag_id=3, slot_id)
                                        │
                                        │ if bag_id != 3: ignore (only bandolier has active slot)
                                        │
                                        │ if prev_slot is dirty:
                                        │   ──── BandolierAmmoUpdate(prev_slot, …) ─▶ DB
                                        │   bandolier_ammo_dirty.remove(prev_slot)
                                        │
                                        │ active_bandolier_slot = slot_id
                                        │
                                        │ ──── ActiveSlotUpdate(player_id, slot_id) ─▶ Base
                                        │
                                        │ ── onEntityProperty(AmmoTypeId, value) ──▶ Client
                                        │   value = new slot's cur_ammo_type, or 0 if empty
```

`onEntityProperty` is sent **even when the new slot is empty** (value=0), mirroring legacy [`SGWPlayer.py:522`](../../deprecated/python/cell/SGWPlayer.py#L522) (`activeItem.ammoType if activeItem else 0`). The bandolier UI's `BandolierMod.refreshAll()` re-reads all 5 slot ammo stats independently on its own subscription path ([`Bandolier.lua:205`](../../game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua#L205)), so we don't have to push per-slot stat updates here.

The previous-slot flush catches the **mid-magazine swap** case: a player fires a few rounds, then swaps weapons before reloading the empty one. Without this, those fires would only persist on the next reload (which may never happen if the player swaps back to the original slot after the next world transition).

Implementation: [`crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs:81-571`](../../crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs#L81).

## Persistence cadence

The `bandolier_ammo_dirty: HashSet<i32>` set is the persistence buffer. Every fire/reload/grant marks the affected slot dirty via `set_slot_ammo()`. Dirty slots drain at:

| Drain point                              | What flushes                          | Code path |
|------------------------------------------|---------------------------------------|-----------|
| Reload completion tick (100 ms cadence)  | The active slot only                  | [`ticks/reload_completion.rs:24`](../../crates/cell/src/cell/service/ticks/reload_completion.rs#L24) |
| `requestActiveSlotChange`                | The previous slot, if dirty           | [`active_slot.rs:408-433`](../../crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs#L408) |
| `requestAmmoChange`                      | The mutated slot, immediately         | [`ammo_change.rs:174-208`](../../crates/cell-combat/src/cell/cell_methods/inventory/bandolier/ammo_change.rs#L174) |
| Disconnect (`DisconnectEntity`)          | All dirty slots                       | [`base_messages/lifecycle.rs:218`](../../crates/cell/src/cell/service/base_messages/lifecycle.rs#L218) |
| Logout fallback (`DestroyEntity`)        | All dirty slots (idempotent)          | [`base_messages/lifecycle.rs:154`](../../crates/cell/src/cell/service/base_messages/lifecycle.rs#L154) |
| World transition (`handle_dial_gate`)    | All dirty slots                       | [`gate_travel/mod.rs:486-495`](../../crates/cell-interactions/src/cell/gate_travel/mod.rs#L486) |

The flush hook lives on the `DisconnectEntity` cell handler — graceful logoff (`SGWPlayer.logOff`), Mercury `DISCONNECT (0x0C)`, and the tick-sync 60-second inactivity timeout ([`tick_sync.rs:79-84`](../../crates/base-session/src/base/tick_sync.rs#L79)) all route through `destroy_client_entities` ([`helpers.rs:67-111`](../../crates/base-session/src/base/helpers/mod.rs#L67)) which sends `BaseToCellMsg::DisconnectEntity`. So a player who closes the game without logging out still has their ammo persisted, just with up to a 60-second delay after their last received packet. The `DestroyEntity` flush is a no-op fallback for any path that bypasses `DisconnectEntity`.

**Trade-off**: a server crash mid-magazine — or any crash before the disconnect-detection window elapses — loses up to one magazine of ammo per active slot. We accepted this over write-per-fire because (a) ammo is cheap and easily refilled in-game, (b) write-per-fire would dominate the DB write rate during sustained combat, and (c) mid-magazine state is already non-deterministic from the player's view.

Helper: [`flush_dirty_bandolier_ammo()`](../../crates/cell-combat/src/cell/cell_methods/inventory/bandolier/active_slot.rs#L19) drains the set into one `BandolierAmmoUpdate` per slot.

## Client UI

The bandolier UI lives in [`game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua`](../../game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua):

- **Meter** — material `'CoreMaterial_AmmoMeter' .. slotID` driven via `setMaterialScalarProperty('M_Percentage', stat.current / stat.max)` ([line 246](../../game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua#L246)).
- **Empty state** — when no item: `M_Percentage = 0` ([line 249](../../game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua#L249)).
- **Count text** — `setProperty("Text", string.format("%d", stat.current))`.
- **Subscription** — `Events.StatUpdated → BandolierMod.onStatUpdated → refreshSlotAmmo(slotIndex)` ([lines 180-186, 496](../../game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua#L180)).

Because the UI is stat-driven, **anything that changes `Stat[AMMO_SLOT_1+slot]` and emits `onStatUpdate` will refresh the bandolier**. This is why the server-side mirror in `set_slot_ammo()` is non-negotiable.

## Reload timing

The warmup window (e.g. 2 s for `ABILITY_RELOAD_WEAPON = 596`) ticks down on the client as a cosmetic animation. The server's `reload_completion_tick` runs every 100 ms and refills the magazine when the deadline is reached — typically mid-animation. Because `onStatUpdate` flushes the new ammo value immediately on refill, the meter visually fills before the animation completes. The next fire is gated by the cooldown timer (warmup + cooldown), not by the bar fill.

This matches legacy [`Reload.py`](../../deprecated/python/cell/effects/Reload.py) behavior: the effect script resolved at warmup completion via `setCurrent(max)` and `sendDirtyStats()`. The Rust port lifts the refill logic out of the effect script and into the cell tick — equivalent semantics, simpler ownership.

## NPC ammo

NPCs (`SGWMob`) use the **same `bandolier_items` / `AmmoSlot{N}` model conceptually**, but the fire-gate short-circuits the ammo requirement:

```rust
if required_ammo > 0 && entity.is_player && current_ammo < required_ammo { … abort … }
```

This means mobs do not currently consume rounds, do not need to reload, and do not have their `bandolier_ammo_dirty` populated by combat. The legacy [`SGWMob.py`](../../deprecated/python/cell/SGWMob.py) implemented full mob ammo (`getAmmoStat`/`getClipSize`/`triggerReload`) but in practice mobs were rarely ammo-limited. We deferred the port; if/when mob reload is needed, three changes are required together — partial work will silently break:

1. Remove the `is_player` short-circuit in [`abilities.rs`](../../crates/cell-combat/src/cell/abilities/mod.rs) so the ammo gate runs for NPCs.
2. Add an AI-driven `requestReload` equivalent that calls `set_slot_ammo` + `reload_complete_at` on the mob entity.
3. **Widen `reload_completion_tick` beyond players.** It currently iterates [`space_mgr.all_player_entity_ids()`](../../crates/cell/src/cell/service/ticks/reload_completion.rs#L33) only — an NPC that sets `reload_complete_at` will never be promoted by the existing tick, leaving the magazine empty forever. Either change the tick to scan all entities with `reload_complete_at = Some(_)`, add a `space_mgr.all_reloadable_entity_ids()` helper, or extend the iteration to include NPCs in fighting state.

Cross-reference: [npc-ai.md § Ammo Management](npc-ai.md#ammo-management).

## Sequence diagram (one shot + one reload)

```text
Client                       Cell                              Base / DB
  │
  │ useAbility(596, 0) ───────▶│  active_ammo() = 5
  │                            │  set_slot_ammo(0, 4)
  │                            │  bandolier_ammo_dirty.insert(0)
  │ ◀── onStatUpdate(m20) ─────│  AmmoSlot1: cur=4
  │     refreshSlotAmmo(0): meter 5/5 → 4/5
  │
  │ … (4 more fires) …         │
  │ useAbility(596, 0) ───────▶│  active_ammo() = 1
  │                            │  set_slot_ammo(0, 0)
  │ ◀── onStatUpdate(m20) ─────│  AmmoSlot1: cur=0
  │     refreshSlotAmmo(0): meter 1/5 → 0/5
  │
  │ useAbility(596, 0) ───────▶│  active_ammo() = 0 < required=1
  │                            │  log "not enough ammo", drop
  │
  │ requestReload(0) ─────────▶│  reload_complete_at = now + 2.0s
  │                            │  start_ability_cooldown(596, 3.0s)
  │ ◀── onTimerUpdate(m12) ────│  cooldown bar
  │ ◀── onEntityProperty(m7) ──│  ammo-type sync
  │                            │  (weapon already drawn from the fire above,
  │                            │   so no BeingAppearance — and handle_reload
  │                            │   intentionally does NOT touch bStateField)
  │
  │   (≈2 s later — next reload_completion_tick after deadline)
  │                            │  refill_active_slot()  → cur=5
  │                            │  reload_complete_at = None
  │                            │  bandolier_ammo_dirty.remove(0)
  │ ◀── onStatUpdate(m20) ─────│  AmmoSlot1: cur=5
  │     refreshSlotAmmo(0): meter 0/5 → 5/5 (fills mid-animation)
  │                            │ ── BandolierAmmoUpdate ───▶│ UPDATE sgw_inventory
  │                            │   {pid, slot=0, cur=5,    │   ammo=5, cur_ammo_type=…
  │                            │    cur_ammo_type=…}       │
```

## Legacy reference points

| File | Purpose |
|------|---------|
| [`deprecated/python/cell/effects/Reload.py`](../../deprecated/python/cell/effects/Reload.py) | Reload effect script — `setCurrent(max)` + `sendDirtyStats()` on warmup completion |
| [`deprecated/python/cell/SGWPlayer.py:1023-1081`](../../deprecated/python/cell/SGWPlayer.py#L1023) | `getAmmoStat()`, `getClipSize()`, `getAmmoCount()`, `consumeAmmo()` |
| [`deprecated/python/cell/AbilityManager.py:669-670`](../../deprecated/python/cell/AbilityManager.py#L669) | Legacy ammo consumption point (warmup completion, not fire-gate) |
| [`deprecated/python/cell/AbilityManager.py:548-552`](../../deprecated/python/cell/AbilityManager.py#L548) | Legacy `requiredAmmo` check + `CONDITION_FEEDBACK_AmmoCountLessThan` |
| [`game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua`](../../game/sgw/Working/SGWGame/Content/UI/Core/Bandolier/Bandolier.lua) | Client UI: meter, count text, `onStatUpdated` subscription |
| [`entities/defs/SGWPlayer.def:794-797`](../../entities/defs/SGWPlayer.def#L794) | `requestReload` cell method def (UINT8 EReloadType) |
| [`entities/defs/interfaces/SGWInventoryManager.def:190-194`](../../entities/defs/interfaces/SGWInventoryManager.def#L190) | `requestAmmoChange` cell method def (INT32 ItemId, INT32 AmmoType) |
| [`entities/defs/interfaces/SGWInventoryManager.def:183-187`](../../entities/defs/interfaces/SGWInventoryManager.def#L183) | `requestActiveSlotChange` cell method def (INT32 BagId, INT32 SlotId) |

## Data references (Rust)

| File | Purpose |
|------|---------|
| [`crates/entity/src/cell_entity.rs`](../../crates/entity/src/cell_entity/mod.rs) | `BandolierItem`, `active_ammo()`/`active_clip_size()`/`active_ammo_type()`/`set_slot_ammo()`/`refill_active_slot()`, `reload_complete_at`, `bandolier_ammo_dirty` |
| [`crates/cell-combat/src/cell/abilities/mod.rs`](../../crates/cell-combat/src/cell/abilities/mod.rs) | `handle_use_ability` — fire-gate, consume, `onStatUpdate` drain |
| [`crates/cell-methods/src/cell/cell_methods/player/world/mod.rs`](../../crates/cell-methods/src/cell/cell_methods/player/world/mod.rs) | `REQUEST_RELOAD` dispatch |
| [`crates/cell-combat/src/cell/cell_methods/player/world/reload.rs`](../../crates/cell-combat/src/cell/cell_methods/player/world/reload.rs) | `handle_reload` (warmup deadline + cooldown) |
| [`crates/cell/src/cell/service/ticks/reload_completion.rs`](../../crates/cell/src/cell/service/ticks/reload_completion.rs), [`base_messages/player_init/mod.rs`](../../crates/cell/src/cell/service/base_messages/player_init/mod.rs#L31) | `reload_completion_tick` (sole refill path), `InitPlayerState` bandolier seeding |
| [`crates/cell-methods/src/cell/cell_methods/inventory/dispatch.rs`](../../crates/cell-methods/src/cell/cell_methods/inventory/dispatch.rs) | `REQUEST_ACTIVE_SLOT_CHANGE` and `REQUEST_AMMO_CHANGE` dispatch |
| [`crates/cell-combat/src/cell/cell_methods/inventory/bandolier/`](../../crates/cell-combat/src/cell/cell_methods/inventory/bandolier/) | the active-slot and ammo-change handlers, `flush_dirty_bandolier_ammo` |
| [`crates/wire/src/cell/messages/mod.rs`](../../crates/wire/src/cell/messages/mod.rs) | `CellToBaseMsg::BandolierAmmoUpdate`, `ActiveSlotUpdate`, `InitPlayerState` |

## Related docs

- [Ammo campaign ledger](../analysis/ammo/README.md) — decisions D-AM01 to D-AM11, known issues and follow-ups
- [abilities-and-effects-system.md § 31](../architecture/abilities-and-effects-system.md#31-special-ammo-modifies-the-shot-directly-from-resourcesammo_modifiers-ammo-campaign-am-04-d-am07) — how special ammo modifies the shot
- [ammo-system.md](../reverse-engineering/findings/ammo-system.md) — AM-01's client findings (no client reserve model, picker source, no client gates)
- [inventory-system.md § Bandolier and ammo](inventory-system.md#bandolier-and-ammo) — Bandolier container layout and DB schema
- [combat-system.md](combat-system.md) — Ability fire pipeline, where the ammo gate sits
- [ability-system.md](ability-system.md) — Ability warmup/cooldown semantics, `ABILITY_RELOAD_WEAPON = 596`
- [stat-system.md](stat-system.md) — `AmmoSlot1..5` (49–53) stat IDs
- [npc-ai.md § Ammo Management](npc-ai.md#ammo-management) — NPC fire-gate exemption
