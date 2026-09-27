# Organizations: Audit

> Type: reference. Audience: the coordinator and packet workers.
> Updated: 2026-09-27, against `main` @ `70795027`. Companions: [decisions](README.md), [work packets](work-packets.md).

Each row is one claim with its evidence. Packets cite rows by id. "Client" means the cooked client Lua tree (`..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI\Core\`) or `SGW.exe` in Ghidra.

## Server code today

| Id | Claim | Evidence |
|---|---|---|
| A-01 | Cell methods 8-19 are routed and decoded, log `UNIMPLEMENTED`, and return `true`. Their `_tx` and `_space_mgr` arguments are unused. | `crates/cell-methods/src/cell/cell_methods/organization.rs:13-162`; routed from `crates/cell/src/cell/dispatch/router.rs:68-69`; constants in `crates/wire/src/cell/cell_methods/organization.rs:6-17`. |
| A-02 | Five decoders drop a trailing `WSTRING`: CM 13 (MOTD), 14 (note), 15 (name and note), 17 (rank name), and CM 94 `onOrganizationCreation` (name). | `organization.rs:83,90,97,119`; `crates/cell-methods/src/cell/cell_methods/player/social.rs:61-64`. |
| A-03 | The four org base methods (0xCF invite, 0xD0 invite by type, 0xD1 kick, 0xD2 rank change) are not decoded; they fall into the "Unhandled SGWPlayer base method" arm. | `crates/base/src/base/dispatch/mod.rs:38-74` (constants), `:90` (dispatcher), `:151` (catch-all). |
| A-04 | Client methods 34-51, 74, 106-108, 134 and 135 exist only as index constants. Nothing builds them, and the `method_idx` module has no org entries. | `crates/wire/src/cell/client_methods/organization.rs:4-38`; `…/player.rs:20-24,76,78`; `…/inventory.rs:14`; `crates/wire/src/mercury/mod.rs:195`. |
| A-05 | No org, squad or guild logic exists anywhere in `crates/`. The only `squad` hits are NPC cover "squad affinity" (`cell-cover`) and the `net_speak` channel names. | Workspace grep; `crates/cell-console/src/cell/console/net.rs:285-290`. |
| A-06 | `onOrganizationHeaderUpdate(INT32, WSTRING, UINT64, WSTRING, UINT64)` is a server-internal **cell** method, not a client method. | `entities/defs/interfaces/OrganizationMember.def:342-348`. |
| A-07 | `gmReloadOrganizations` is `SGWGmPlayer` cell method 164, unimplemented. | `entities/defs/SGWGmPlayer.def:355`; `docs/protocol/cell-method-dispatch-table.md:606`. |

## Wire contract

| Id | Claim | Evidence |
|---|---|---|
| A-08 | Base-method argument order is `organizationInvite(INT32 orgId, WSTRING name)`, `organizationInviteByType(UINT8 type, WSTRING name)`, `organizationKick(INT32 orgId, WSTRING name)`, `organizationRankChange(INT32 orgId, WSTRING name, UINT8 rank)`. `docs/protocol/sgwplayer-base-method-dispatch-table.md` had the order and types wrong; the plan PR fixes it. | `OrganizationMember.def:418-449`. |
| A-09 | Creation is **cell** method 94 `onOrganizationCreation(WSTRING name)`. The type is not on the wire; it is implied by which creation dialog is open. `organization-wire-formats.md` described a base method `organizationCreation(UINT8, WSTRING)` that does not exist; the plan PR fixes it. | `SGWPlayer.def:877-880`; client `TeamMod.onTeamCreate` / `CommandMod.onCommandCreate`. |
| A-10 | `EReasons`: requested 0, kicked 1, disbanded 2, logout 3. | `entities/defs/enumerations.xml:104`. |
| A-11 | `RosterInfo` has no online flag. The client computes "Online" when a member's roster record holds a non-zero id **and** that id resolves to a `GamePlayer` in its own entity table; otherwise it shows "Offline". So online status is limited to entities the client has streamed. | `teamGetMemberInfo` native `0x00ac8c70` → `0x00ae8620` → builder `0x00ae83d0`; entity lookup through `0x00dd0de0` → `0x00e221a0`. |
| A-12 | Team's editor exposes ranks 2, 3 and 8 and 12 permission bits; Command's exposes ranks 1-7 (8 fixed) and 14 bits (adds OfficerChat and EmailLists). | Client `Team/Team.lua:59-123`, `Command/Command.lua:64-118`. |
| A-13 | The name rule is "unique, 1-60 characters" for both types. | Client `Team.int` `CreateTeamMoniker`, `Command.int` `CreateCommandMoniker`. |
| A-14 | No creation cost, maximum member count, or `RetCode` text exists anywhere in the client strings. | Grep over `Organization.int`, `Team.int`, `Command.int`, `Squad.int` and the RTTI string table. |
| A-15 | The Team and Command vaults size from the ordinary container size: `maxSize = floor(getContainerSize(Container.TeamBank) / 10)`, 40 visible slots, and the source comment "General has 40 to 100, in intervals of 10". No vault-size message exists. | Client `Team.lua` / `Command.lua` `ValidateScrollbar()`, `Team.lua:207`. |
| A-16 | `SquadLootType` is RoundRobin 0 and FreeForAll 1. The loot menu is offered to **every** squad member (`bShowSquadOptions` comes from `isInSquad()` only). | `deprecated/python/Atrea/enums.py:468-469`; client `SelfWindow/SelfWindow.lua`. |
| A-17 | No evidence settles Team/Command exclusivity. `records` is keyed per organization, `squad` is a separate property, no "already in a team" string exists, and the only exclusion is that the two creation dialogs cannot be open together. | `OrganizationMember.def` properties; client `Team.lua`, `Command.lua` (`CreateCommandWin:hide()`). |
| A-18 | The maximum squad size is 6. The shipped client wires frame 6 to `Unit.Squad5` by mistake, so frame 6 never tracks its member. That is a client display bug only. | Client `Squad.lua` `setupSquadMember`; RTTI `TargetSquadMember1..6` at `0x0184094c`-`0x018409ec`. |
| A-19 | `squad` is `INT32 CELL_PUBLIC`, which is ghosted between CellApps and **never** sent to a client. Clients learn membership only through methods 34-51. | `OrganizationMember.def` properties; `docs/protocol/entity-property-sync.md:29,45,225`. |
| A-20 | `launchOrganizationCreation` [135] opens `CreateTeamWin` or `CreateCommandWin`. The strings point players to a registrar ("Talk to the team registrar on Harset…", "…at the Omega Site"), and `EInteractionType.OrganizationCreation = 9`. Legacy constants name the registrar interactions 7447 (Team) and 7448 (Command). | Client `Organization.int`; `enumerations.xml:843-858`; legacy `Constants.py`. |
| A-21 | `squadKick(name)` is a separate Lua native from `teamKick` and `commandKick`. Its wire path is unknown (ORG-E1 Q2). | Client `Squad.lua` `SquadMod.squadKickUnit`. |
| A-43 | The client's `EDBErrorType` carries org error codes: `EDB_ERROR_Player_in_org_type` -20071, `Player_rank_too_low` -20072, `Invoker_not_org_member` -20090, `Target_not_org_member` -20091, `Invalid_org_rank` -20092. They support one membership per type and the rank checks in D-ORG09. Whether the client renders them as text is ORG-E1 Q4. | `entities/defs/enumerations.xml:1948-1959`. |
| A-22 | Vault windows open only from `onTeamVaultOpen` [107] / `onCommandVaultOpen` [108] (`INT32 EntityId, VECTOR3 Position`), an NPC-shaped signature. Nothing in the org panel opens them. | `SGWPlayer.def:1158-1171`; no vault button in `Organization.lua`, `Team.lua` or `Command.lua`. |

## Infrastructure the packets reuse

| Id | Claim | Evidence |
|---|---|---|
| A-23 | The cell can call a client method on **any** player's entity: `CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }`. Chat fanout and trade use it. | `crates/wire/src/cell/messages/cell_to_base.rs:57`; `crates/cell-console/src/cell/console/chat.rs:141-160`; `crates/cell-methods/src/cell/cell_methods/player/trade/handlers.rs:342-430`. |
| A-24 | The cell resolves a player name across all spaces with `SpaceManager::find_online_player_by_name`, which returns Found, InTransition, NotFound or Ambiguous. The name is missing during gate travel until `InitPlayerState` re-caches it. | `crates/cell-world/src/cell/space_manager/queries.rs:342`. |
| A-25 | The base's online-session map is `connected: HashMap<SocketAddr, ConnectedClientState>` (with `active_player_id`, `player_entity_id`, `player_name`, `access_level`) plus `entity_to_addr`. `send_to_witness_reliable` is a single-recipient send despite its name. | `crates/base-session/src/base/mod.rs:131`; `crates/base-session/src/base/helpers/mod.rs:489,593`. |
| A-26 | The contact list is the model for DB-backed fanout: a DB query for recipients, a scan of `connected` for their entity ids, then one reliable send each, with no in-memory roster. | `crates/base-session/src/base/contact_list/handlers/presence_fanout.rs:26,56-144,151`. |
| A-27 | The cell-to-base message pattern for a DB-backed social feature is the `ContactList*` variants routed through `contact_list_dispatch::route`. | `cell_to_base.rs:668-758`; `crates/base-world-entry/src/base/world_entry/cell_dispatch/mod.rs:135-142`; `…/contact_list_dispatch.rs`. |
| A-28 | Base-method handlers have `connected`, `entity_to_addr`, `cell_tx`, `db_pool` and `transport` in scope. | `crates/base/src/base/dispatch/mod.rs:90`. |
| A-29 | Login completes in `handle_on_client_ready`: `InitPlayerState` to the cell, the burst bundle, `push_contact_lists_on_login`, then the online fanout. The cell seeds per-player state in `handle_init_player_state`. | `crates/base-world-entry/src/base/world_entry_appearance/client_ready/mod.rs:41,336,431,436,452-468`; `crates/cell/src/cell/service/base_messages/player_init/mod.rs:72`. |

## Prior attempt: PR #584

| Id | Claim | Evidence |
|---|---|---|
| A-30 | #584 (branch `feat/org-system-568`, June 2026, 5,824 lines in 55 files) implemented #568 Phases 1-3 in the pre-split `cimmeria-services` crate. It cannot be rebased onto `main` after #614 and #825. Salvage: the schema (`db/sgw/Organizations/`), the models, `base/organization/wire.rs`, persistence and its tests. Known defects to fix while porting: (1) `SquadManager` is **per space**, so a squad breaks when a member changes worlds; (2) its `EReasons` values are wrong (disbanded 0, left 1, kicked 2, against A-10); (3) its loot enum comment guesses four values (A-16 says two); (4) it keeps a `leader_player_id` column that must stay in step with the member ranks; (5) CM 11, 12 and 18 have no request correlation or leader check. | `git show origin/feat/org-system-568:<path>`; the PR's review thread. |

## Gaps outside the org code

| Id | Claim | Evidence |
|---|---|---|
| A-35 | Only `logOff` announces a player going offline. `destroy_client_entities`, the path for client disconnect, inactivity timeout, duplicate login and logoff, does no presence fanout, so a crash or quit never tells contacts. | `crates/base/src/base/dispatch/session.rs:20,80-96`; `crates/base-session/src/base/helpers/mod.rs:391`; callers at `crates/base/src/base/connect_loop/encrypted/mod.rs:373`, `crates/base-session/src/base/tick_sync.rs:183`, `crates/base/src/base/login/mod.rs:127,403`. |
| A-36 | The cell's chat match handles only say, emote and yell. Team, squad and command are registered with the client and then dropped; officer (6) is not registered. | `crates/cell-console/src/cell/console/chat.rs:81-93`; `crates/base-session/src/base/world_entry_chat.rs:20-29`. |
| A-37 | Player-originated tell is not implemented: the base parses the target and drops it. | `crates/base/src/base/dispatch/chat.rs:47-52,99-109`. |
| A-38 | No Rust code references `EInteractionType`. The interact dispatcher knows Dialog, Vendor, Trainer and Loot, plus the DHD pre-check. The notification bits `INT_BANKER` (2) and `INT_ORGANIZATION` (64) exist. | `crates/cell-interactions/src/cell/interactions/dispatch/interact.rs:32-246`; `crates/entity/src/cell_entity/mod.rs:85-94`; `crates/entity/src/interaction_flags.rs:21,31`. |
| A-39 | On `main` @ `70795027`, containers 17-20 had zero capacity in `bag_max_slots`, so the move path rejected every bank move. The Bank campaign's BV-01 (#872) moves `bag_max_slots` to `cimmeria_entity::inventory` with 100 slots for 17-20 and adds a `player_movable` allowlist that still refuses 19 and 20 until its BV-07. (The Bank campaign owns this.) | Before: `crates/wire/src/containers.rs:10-20`; `crates/base-methods/src/base/world_entry/methods/inventory/move_/mod.rs:64-81`. After, per PR [#872](https://github.com/SandboxServers/Cimmeria/pull/872) as reported by the Bank campaign (cimmeria-97), not yet re-read here: `crates/entity/src/inventory.rs` (`bag_max_slots`) and `crates/base-methods/src/base/world_entry/methods/inventory/move_/container_policy.rs` (`player_movable`). |
| A-40 | `EChannel` in `enumerations.xml` has server 8, feedback 9, tell 10 and no 7. The Rust registration and `docs/gameplay/chat-system.md` use server 7 and tell 9, citing a `Constants.py` that defines neither. The legacy `Chat.py` builds every channel from the enum. | `enumerations.xml:113-128`; `crates/wire/src/cell/chat.rs:24-35`; `world_entry_chat.rs:20-32`; `deprecated/python/base/Chat.py:144-154`. |
| A-41 | The wireclient can already run two players against one server (`two_client_castle_visibility.rs`), but has no public cell-method or base-method builder, and its test binary is not in CI's live-DB job. | `crates/wireclient/tests/it/two_client_castle_visibility.rs`; `crates/wireclient/src/session.rs:237`; `docs/architecture/wireclient.md`. |
| A-42 | `.` console commands are a `Spec` row in `registry/commands/<family>.rs` plus an `exec` match arm, gated by `console::is_gm`. | `crates/cell-console/src/cell/console/registry/`; `console/dispatch.rs:179`; `crates/cell-world/src/cell/dispatch/gm_gate.rs:216`. |
