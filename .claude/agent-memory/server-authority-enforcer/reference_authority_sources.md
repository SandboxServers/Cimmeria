---
name: reference-authority-sources
description: Where server-of-truth state lives for inventory, position, currency, GM flag, vendor session
metadata:
  type: reference
---

- **Inventory ownership:** `sgw_inventory.character_id = $player_id AND item_id = $instance_id`. Every mutation path uses this pair. `item_id` is the unique row id; `type_id` is the design id (resources.items.item_id).
- **Player cash (naquadah):** `sgw_player.naquadah` — read with `FOR UPDATE` before debit/credit.
- **Player advisory lock for inventory moves:** `SELECT pg_advisory_xact_lock($player_id, 0)` for all-containers; `pg_advisory_xact_lock($player_id, $container_id)` for per-container slot reservations. Documented in `inventory/move_/mod.rs:99-126`.
- **Live position (cell-authoritative):** `space_mgr.get_entity(eid).position` (`crates/services/src/cell/space_manager/entities.rs`). Currently overwritten unconditionally from client wire `AVATAR_UPDATE_EXPLICIT` — see [[exploit-avatar-update-explicit]].
- **GM/access level:** `ConnectedClientState.access_level` (`crates/services/src/base/mod.rs:116`), populated from `account.accesslevel` at auth time. Never read from inbound packets. Speaker flags computed at `crates/services/src/base/dispatch.rs:131`.
- **Vendor session (open vendor for purchase/sell):** `CellEntity.vendor_entity: Option<u32>` set by `send_store_open` from the server-side entity, then `template_id` looked up server-side. Client-supplied `vendor_template_id` is validated against this in `cell_methods/player/vendor.rs:93-122`.
- **Bandolier active slot:** `CellEntity.active_bandolier_slot` (cell) + `sgw_player.bandolier_slot` (base persist). Per-slot ammo lives in `sgw_inventory` row (container_id = 3).
- **Looting target:** `CellEntity.looting_entity` — set by `interact()` after range check; not re-checked on subsequent lootItem calls. See [[exploit-loot-no-ownership]].
- **Cooldowns:** `CellEntity.abilities` (`crates/entity/src/abilities`). `is_on_cooldown` + `start_ability_cooldown` are the canonical seam.
- **Faction (hostility):** `CellEntity.faction` vs `combat::HOSTILE_FACTION` sentinel. Single-source-of-truth in `crates/services/src/cell/combat/mod.rs`. AoE and cone use it; single-target useAbility does NOT — see [[exploit-use-ability-no-faction]].
