# Bank and Vault: Audit Against `main`

> Type: reference. Audience: the coordinator and packet workers.
> Updated: 2026-09-27, against `main` @ `004bccb4`. Companions: [campaign README](README.md), [work packets](work-packets.md), [CAT-D inventory findings](../../security-audit/2026-05-31-server-authority/findings/CAT-D-inventory.md), [inventory system](../../gameplay/inventory-system.md).

Every row was checked against the code, the seed, the client Lua, or Ghidra (`SGW.exe`, image base `00400000`). IDs (`A-nn`) are cited by the work packets. Paths are relative to the repo root unless marked as client files. Client Lua lives under `SGWGame\Content\UI\Core\`.

## 1. Client

| ID | Finding | Evidence | Confidence |
|---|---|---|---|
| A-01 | No client code is specific to a Banker. Right-clicking any NPC sends the generic `interact(targetEntityId)`, and the server picks which window to open. | `docs/protocol/client-method-dispatch-table.md:497`. `Vault.lua`, `Team.lua` and `Command.lua` contain no `interact` or banker call. | High |
| A-02 | `onVaultOpen` (106), `onTeamVaultOpen` (107) and `onCommandVaultOpen` (108) each take `(INT32 EntityId, VECTOR3 Position)` and only show the window. The handler emits `Event_UI_VaultVisibility`, which runs `VaultMod.onVaultVisibility`, which calls `moveToFront()` and `setVisible(true)`. | `docs/protocol/client-method-dispatch-table.md:253-255`. Ghidra: `00d7e560` → `00d7e640` → `00d7e5e0`. `Vault.lua:9-16` and `:803`. Team and Command match (`Team.lua:152,619`, `Command.lua:148,614`). | High |
| A-03 | Nothing on the client opens a vault without the server. Closing one sends no message to the server. | `Vault.lua:19-21` (`onCloseClicked` only calls `hide()`). The Team and Command close handlers work the same way. | High |
| A-04 | `Container.Vault` is 17, `Container.TeamVault` and `Container.TeamBank` are both 19, and `Container.CommandVault` and `Container.CommandBank` are both 20. Each pair names the same container. | `entities/defs/enumerations.xml:925-951` has only 17-20. `Team.lua` and `Command.lua` use both names on the same slot grid. | High |
| A-05 | Items move in and out of every vault with the plain `moveItem`. There is no organization move RPC. | `Vault.lua:571`, `Team.lua:428`. The client tree has no `organizationMoveItem`. | High |
| A-06 | The Team and Command vaults are opened from an NPC only. The organization panel has no vault button. | A search of the whole UI tree finds no open-vault button. `Organization.lua` has no vault call. | High for Lua; a native-only path is not ruled out |
| A-07 | The Team and Command cash buttons are enabled by `hasTeamPermission` and `hasCommandPermission` for `DepositCash` and `WithdrawCash`. Without the permission sync they render disabled, with no error. | `Team.lua:167-198`, `Command.lua:163-194`. Cash moves through the natives `teamTransferCash(±amount)` and `commandTransferCash(±amount)`. | High |
| A-08 | No fee or cost UI in any vault file. | Searching `Vault.lua`, `Team.lua` and `Command.lua` for "fee" or "cost" finds nothing. | High |
| A-09 | The visible vault size is `getContainerSize(Container.Vault)`, taken from the declared container size. The window scrolls a fixed 40-slot view in rows of 10. There is no expansion UI. | `Vault.lua:298-314`. There is no `expand` token in the vault files. | High |
| A-10 | Legacy sizes are flat: `BAG_SIZES` has 100 for each of 17-20. Growing the bank from 40 to 100 is project design, not restoration. | `deprecated/python/common/Constants.py:159-162`. The "40 to 100, in intervals of 10" comment is in `Team.lua:207`. | High |
| A-11 | `event-net-mapping.md` drops the `onTeamVaultOpen` row and shifts the rows below it up one. The correct addresses are: `onVaultOpen` `00d7e560`, `onTeamVaultOpen` `00d7e800`, `onCommandVaultOpen` `00d7eaa0`, `onStoreOpen` `00d7ed40`. | Ghidra: each registration stub decompiles to its own `Event_NetIn_on…` literal. Fixed in the plan PR. | High |
| A-12 | Nothing reads `SGWPlayer.isBankingOverride`, which is `INT8 CELL_PUBLIC` with default 0. | `entities/defs/SGWPlayer.def:91-95`. No Lua hit, and no hit in Ghidra's names or strings. | Low: this is a negative result |
| A-13 | `onOrgMoveItemResult` and `onClearOrgVaultInventory` (74) have no Lua subscriber. | A search of the whole UI tree finds none. `docs/gameplay/inventory-system.md:38` marks the org vault "NOT IMPL". | High for Lua; they may be dead protocol |
| A-14 | `organizationTransferCash` is **cell** method 19. | `docs/protocol/cell-method-dispatch-table.md:142`, `crates/wire/src/cell/cell_methods/organization.rs:17` (confirmed by cimmeria-fa). Client method 19 is the unrelated `onStateFieldUpdate`. | High |

## 2. Server

| ID | Finding | Evidence |
|---|---|---|
| A-20 | `bag_max_slots` has no arm for 17-20, so it returns 0. `moveItem` then rejects every target slot as invalid before any other check runs. | `crates/wire/src/containers.rs:10-19`, `crates/base-methods/src/base/world_entry/methods/inventory/move_/mod.rs:64,79` |
| A-21 | `BAG_SIZES` is a separate table. It declares 17, 18 and 19 as 100 each, and leaves out 20. It is what `onBagInfo` sends at world entry and on resync, so the client is told about slots the server will not accept. | `crates/entity/src/inventory.rs:29-55`, `crates/wire/src/mercury/world_data/map_loaded.rs:340-346`, `crates/base-methods/.../inventory/core/mod.rs:131-140` |
| A-22 | The login inventory query has no container filter. A row in container 17 loads, and reaches the client in the `onUpdateItem` batch. | `crates/base-methods/.../inventory/core/mod.rs:37-49`, `player_load/core/inventory_items.rs:14-52`, `map_loaded.rs:365-372` |
| A-23 | `sgw_inventory` accepts container 17 as it is. The unique index `(character_id, container_id, slot_id)` does not care which container, and no `CHECK` limits `container_id`. `character_id` is `NOT NULL`, so org-owned rows need their own design. | `db/sgw/Inventory/Tables/sgw_inventory.sql` |
| A-24 | `NpcInteractionType` has `Dialog`, `Vendor`, `Trainer` and `Loot`, and no Banker. It is derived once, at spawn, from the template's interaction bits. | `crates/entity/src/cell_entity/mod.rs:85-94`, `crates/cell-world/src/cell/space_manager/spawn.rs:345-382` |
| A-25 | `INT_BANKER = 2` exists and matches legacy `INT_Banker = 2`. Nothing dispatches on it. | `crates/entity/src/interaction_flags.rs:20-21`, `deprecated/python/Atrea/enums.py:612` |
| A-26 | `ON_VAULT_OPEN = 106`, `ON_TEAM_VAULT_OPEN = 107` and `ON_COMMAND_VAULT_OPEN = 108` exist, and nothing sends them. The wire log already knows their names. | `crates/wire/src/cell/client_methods/player.rs:19-24`, `crates/wire-log/src/wire_log/client_names.rs:130-132` |
| A-27 | The interaction patterns to copy are the vendor (`send_store_open`, a base round trip) and the trainer (`try_open_trainer`, sent from the cell). The trainer is resolved before `handle_interact` runs. | `crates/cell-interactions/src/cell/interactions/vendor.rs:12-63`, `trainer.rs`, `dispatch/interact.rs:156-245`, `crates/cell-methods/.../player/interaction/interact.rs` |
| A-28 | Loot pins its target once and never checks proximity again. Vault moves must not repeat that. | CAT-D-02 in `docs/security-audit/2026-05-31-server-authority/findings/CAT-D-inventory.md` |
| A-29 | Items can be moved out of the buyback container (16) for free. This is issue #798, in the same function as A-20. | CAT-D-07, issue #798, `move_/mod.rs` |
| A-30 | Trade allows only `INV_MAIN`, so bank items cannot be traded. Keep it that way. | `crates/base-methods/.../trade/execute/swap.rs:277-297` |
| A-31 | The comments in `VENDOR_COST_BAGS` are wrong: 2 is `INV_Mission`, not the bank, and 15 is `INV_Crafting`, not the hotbar. | `crates/base-methods/.../vendor/purchase_helpers.rs:8-16`, `crates/entity/src/inventory.rs:14,27` |
| A-32 | The GM give helpers exist: `gmGiveItem` (defaults to `INV_MAIN`, maximum quantity 1000) and `gmGiveCash`. | `crates/cell-console/src/cell/console/gm/give.rs` |
| A-33 | Templates 370-389 and spawns 470-489 are unused. The debug-hub doc lists Bank as a known gap. | `db/resources/Entities/Seed/entity_templates.sql`, `db/resources/Worlds/Seed/spawnlist.sql`, `docs/content/debug-hub.md` |
| A-34 | The legacy body visuals for a Banker are terminal meshes (`WorldObject_StandingTerminal_*`, `WorldObject_WallTerminal`) plus the playable-race meshes. | `deprecated/db-monolithic-sql/db-deprecated/resources.sql:12406,13299-13300,14846-14853,17260` |
| A-35 | The legacy org bank permission bits are `EORG_PERM_DepositBank = 65536`, `WithdrawBank = 131072` and `ViewBankLogs = 1048576`. | `deprecated/python/Atrea/enums.py:1325-1329`. The Rust side is `cimmeria_entity::organization::OrgPermission` (ORG-01). |

## 3. Doc bugs fixed in the plan PR

- `docs/analysis/event-net-mapping.md`: the vault and store address rows (A-11).
- `docs/client/ui-layout-inventory.md`: the `Command/CommandVault.layout` rows are the Command organization vault, not a macro library.

A-31 is a code comment, so BV-01 fixes it.
