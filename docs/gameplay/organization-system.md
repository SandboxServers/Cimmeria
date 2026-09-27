---
title: "Organization System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Organization System

> **Last updated**: 2026-09-27
> **Status**: Squads work (ORG-03, ORG-04): invite, accept, leave, kick, loot mode, disconnect, gate travel, squad chat, the minimap ping and the GM `.squad_*` commands, as cell state; see [group-system.md § Squads](group-system.md#squads-org-03). Teams and Commands are not implemented yet: their calls are decoded and answered with "not available yet", with no persistence, roster or fanout. The organizations campaign ([docs/analysis/organizations/](../analysis/organizations/README.md)) builds them on the same wire contract. Teams and Commands have their schema and persistence layer (ORG-02, [Persistence](#persistence)); no handler calls it yet.

## Overview

Organizations are persistent player groups: Commands (guilds), Squads (parties), and Teams (strike teams). Each organization has a roster, rank hierarchy with permissions, shared cash, experience, MOTD, and member/officer notes. The system supports inviting, kicking, rank changes, loot modes, minimap pings, PvP strike team status, and organization vault storage.

The `OrganizationMember` interface in `entities/defs/interfaces/OrganizationMember.def` is the largest gameplay interface by property + method count.

## Implementation Status

What exists after the campaign's contract packet (ORG-01) and the squad core (ORG-03):

- **Models** in `cimmeria_entity::organization` ([`crates/entity/src/organization/`](../../crates/entity/src/organization/)): `OrgType`, `OrgRank` with the ranks each type uses, the 26 `OrgPermission` bits with the 12 (Team) and 14 (Command) bits the client's rank editors expose, `OrgLeaveReason`, `SquadLootType`, the id-space constants, the default rank permissions and `org_text`, the one implementation of the text rules (lengths, forbidden characters, the name normaliser and its uniqueness key). Every enum value is pinned against `entities/defs/enumerations.xml`.
- **Inbound decoders** in `cimmeria-wire`: cell methods 8–19 and SGWPlayer cell method 94 `onOrganizationCreation` ([`crates/wire/src/cell/cell_methods/organization/`](../../crates/wire/src/cell/cell_methods/organization/)), and base methods 0xCF–0xD2 ([`crates/wire/src/base/organization.rs`](../../crates/wire/src/base/organization.rs)). Each bounds a `WSTRING`'s declared length by the bytes left before allocating, and rejects truncation, trailing bytes and unpaired surrogates. CM 10 rejects a non-finite coordinate, and CM 19's signed amount decodes to a `CashDir` (positive deposits, negative withdraws, zero is rejected). CM 13, 14, 15, 17 and 94 used to drop their text; they now read it.
- **Dispatch.** The cell router ([`crates/cell-methods/src/cell/cell_methods/organization/`](../../crates/cell-methods/src/cell/cell_methods/organization/)) decodes cell methods 8–19 and routes on the id each carries (D-ORG05, D-ORG06): CM 8 with a cell-issued request id, CM 9 with a squad-range org id, and CM 18 go to the squad handlers in `squad/`; everything else, and CM 94 in `player/social.rs`, logs `UNIMPLEMENTED` at DEBUG on the `org` target (the text length, never the text) and is answered with `onErrorCode` and the feedback line "Organizations are not available yet." (`forward.rs`, which ORG-07 turns into a forward to the base). The base arm for 0xCF–0xD2 ([`crates/base/src/base/dispatch/organization.rs`](../../crates/base/src/base/dispatch/organization.rs)) forwards `organizationInviteByType` type 0 and `organizationKick` with a squad id to the cell, refuses a type above 2, and answers every other well-formed call with the same pair, so the press is not silent.
- **Outbound serializers** for client methods 34–51, `onOrganizationCreationResult` (134) and `launchOrganizationCreation` (135) in [`crates/wire/src/cell/client_methods/organization/`](../../crates/wire/src/cell/client_methods/organization/) and `player.rs`, each byte-tested. The squad handlers send 34–40 and 51.
- **Cell↔base messages** `CellToBaseMsg::Org(OrgCellToBase)` and `BaseToCellMsg::Org(OrgBaseToCell)`. `SquadInvite` and `SquadKick` reach the cell's squad handlers; `OrgCellToBase` is still a logged no-op on the base.
- **Squads** ([group-system.md § Squads](group-system.md#squads-org-03)): the service-wide `SquadRegistry` on `SpaceManager` and `CellEntity::squad_id`.

The schema and the base-side persistence layer came with ORG-02; see [Persistence](#persistence).

| Feature | Status | Notes |
|---------|--------|-------|
| Organization types | DEFINED | Command, Squad, Team in entity defs; typed models in `cimmeria_entity::organization` |
| Invite response | PARTIAL | `organizationInviteResponse` (CM 8): squads DONE; a base-issued request id (Team, Command) is answered "not available yet" |
| Leave | PARTIAL | `organizationLeave` (CM 9): squads DONE; a Team or Command id is answered "not available yet" |
| Minimap ping | PARTIAL | `BroadcastMinimapPing` (CM 10): squads validated (own squad, one a second) and logged, never relayed, since no client method shows another member's ping (ORG-E1 Q3, ORG-04); a Team or Command id is answered "not available yet" |
| Strike team (PvP) | STUB | `strikeTeamResponse` (CM 11) decodes, logs, drops |
| PvP leave confirmation | STUB | `pvpOrganizationLeaveResponse` (CM 12) decodes, logs, drops |
| MOTD | STUB | `organizationMOTD` (CM 13) decodes the org id and the MOTD, logs, drops |
| Member note | STUB | `organizationNote` (CM 14) decodes the org id and the note, logs, drops |
| Officer note | STUB | `organizationOfficerNote` (CM 15) decodes the org id, the member name and the note, logs, drops |
| Rank permissions | STUB | `organizationSetRankPermissions` (CM 16) decodes, logs, drops |
| Custom rank names | STUB | `organizationSetRankName` (CM 17) decodes the org id, rank and name, logs, drops |
| Loot mode | DONE | `squadSetLootMode` (CM 18): the leader only, 0 or 1, then `onSquadLootType` to every member |
| Cash management | STUB | `organizationTransferCash` (CM 19) decodes, logs, drops |
| Creation | STUB | `onOrganizationCreation` (SGWPlayer CM 94) decodes the name, logs, drops. `launchOrganizationCreation` (135) and `onOrganizationCreationResult` (134) have serializers, never sent |
| Invite issue / kick / rank change | PARTIAL | 0xD0 type 0 (squad invite) and 0xD1 with a squad id (squad kick) are DONE; a type above 2 is refused; 0xCF, 0xD2 and the Team and Command forms answer with `onErrorCode` and a feedback line |
| Roster info | PARTIAL | `onOrganizationRosterInfo` (38) is sent for squads only |
| Experience tracking | NOT IMPL | `onOrganizationExperienceUpdate` (CM 44) never sent |
| Persistence | IMPLEMENTED | Tables, constraints, the leader trigger and the locked write API (ORG-02, [Persistence](#persistence)); no handler calls them yet |
| Organization vault | NOT IMPL | Only `onClearOrgVaultInventory` reference |

## Persistence

Teams and Commands are stored in three tables under [`db/sgw/Organizations/`](../../db/sgw/Organizations/). Squads are never stored (D-ORG03). The base is the authority and the database is the source of truth (D-ORG04). Decisions are in the [campaign ledger](../analysis/organizations/README.md).

| Table | What it holds | Constraints |
|---|---|---|
| `sgw_organizations` | One row per organization: `org_id`, `org_type` (1 Team, 2 Command), `name`, `name_key`, `motd`, `cash`, `experience`, `created_at` | `org_id` in 1 to `0x3FFF_FFFF`, below the squad id range (D-ORG05); the sequence stops at the same value. `UNIQUE (org_type, name_key)`: names are unique per type on the case-folded key (D-ORG10). `cash >= 0`. `UNIQUE (org_id, org_type)` exists only as the target of the members' composite foreign key |
| `sgw_organization_ranks` | One row per rank the type uses (Team 2, 3, 8; Command 1 to 8): a copy of `org_type`, a custom `name` (NULL shows the client default) and a 26-bit `permissions` mask | The per-type rank set is a CHECK on `(org_type, rank)`, and a composite foreign key to the organization's `(org_id, org_type)` keeps the copy honest (D-ORG07); mask 0 to `0x3FF_FFFF`; the Leader row (8) must hold every bit (D-ORG08) |
| `sgw_organization_members` | One row per member: `org_id`, `player_id`, copies of `org_type` and the character's `account_id`, `rank`, `note`, `officer_note`, `joined_at` | `UNIQUE (player_id, org_type)`: one Team and one Command per player (D-ORG18). Composite foreign keys to the organization's `(org_id, org_type)`, so the copy cannot drift, and to the rank row `(org_id, rank)`, so rank 0 and a rank the type does not use are refused. At most one member at rank 8 (a unique partial index). The row goes with its character (`ON DELETE CASCADE` from `sgw_player (player_id, account_id)`). A `BEFORE UPDATE` trigger refuses a change to `org_id`, `player_id`, `org_type` or `account_id`, and any move to or from rank 8 outside the delete trigger's promotion |

There is no leader column: **the leader is the member at rank 8**. A disband deletes the organization row, and its ranks and members cascade.

### Losing a leader

An `AFTER DELETE` trigger on `sgw_organization_members` (`org_member_after_delete()` in [`db/sgw/_functions.sql`](../../db/sgw/_functions.sql)) runs on every member delete: a leave, a kick, a GM tool, test cleanup, or the cascade from a character delete (D-ORG12, D-ORG20). It locks the organization row first, then:

1. If a Leader remains, it does nothing.
2. Otherwise it promotes the highest-ranked member to Leader, the longest-standing (earliest `joined_at`, then lowest `player_id`) among equals.
3. If nobody remains, it calls `org_vault_is_empty_sql(org_id)`. An empty vault disbands the organization. A non-empty one leaves a **memberless organization** that keeps its vault for GM recovery; a memberless organization accepts a new member only as Leader.

`org_vault_is_empty_sql` is a stub that returns true. The Bank / Vault campaign replaces it, and its Rust twin `api::org_vault_is_empty`, when the vaults land.

Every character delete keeps the lock order in the database, whatever issued it. A `BEFORE DELETE` trigger on `sgw_player` (`org_player_before_delete()`) locks the character's organizations in `org_id` order after the character row and before the member rows cascade. An account delete deletes all its characters in one statement, so a `BEFORE DELETE` trigger on `account` (`org_account_before_delete()`) first locks every character row in `player_id` order, then all their organizations in `org_id` order. The game's delete path, `organization::character_delete::delete_character`, adds an ownership check (another account's request locks nothing) and logs the trigger's results after its commit.

### The write API (ORG-LOCK)

The code is in [`crates/base-session/src/base/organization/`](../../crates/base-session/src/base/organization/):

- `api.rs` (the ORG-API the Bank campaign builds on): `lock_org(tx, org_id)` takes `SELECT ... FOR UPDATE` on the organization row and returns its `OrgHeader`; `member_access_locked(tx, org_id, player_id)` takes that lock and returns the member's `OrgAccess` (type, rank, permissions) read under it; `OrgAccess::system(tx, org_id, actor)` builds one for a GM or server action and logs it (`org.gm_action` or `system_action`); `org_vault_is_empty(tx, org_id)` is the vault stub. There is no pool-level membership check.
- `persistence/`: `create_org` (the organization, one rank row per `default_rank_permissions` entry, and the leader, all under a savepoint), `add_member`, `remove_member` (which reports what the trigger did: nothing, a promotion, a disband, or a memberless organization), `set_rank`, `set_text` (MOTD, note, officer note, rank name), `set_rank_permissions`, `disband` (refused while the vault is not empty), and the display reads `load_memberships`, `load_roster`, `load_ranks` and `name_available`.

Every mutation takes a transaction and an `actor: &OrgAccess`, and locks the organization row first. It refuses an `OrgAccess` for another organization (`ActorMismatch`) or one read in another transaction (`StaleAccess`), so authorization is always read under the same lock as the write. The lock order is the organization row, then `sgw_player` rows, then item rows (D-ORG04); character deletes are the one exception, described in `api.rs` § "Lock order". Deciding who may invite, kick, promote or edit belongs to the handler, from that `OrgAccess`. The persistence layer enforces the data invariants:

- A rank must be one the type uses.
- The Leader rank is never assigned by a rank change, the leader is never moved off it, and its permission row is never edited.
- Text passes `org_text::validate` before it is stored (D-ORG10).

A miss (no such organization, not a member, a rank row that does not exist) is a typed `OrgStoreError`, never `Ok`. Every refusal leaves the caller's transaction usable.

### Telemetry

Everything logs on the `org` target (catalog row in [observability.md](../architecture/observability.md)):

- Each persistence function logs a DEBUG `event` named after itself on success, with `org_id`, `player_id` where there is one, `rows_affected`, and the before and after values of a rank (`from_rank`, `to_rank`), a mask (`from_mask`, `to_mask`) or a text (`from_units`, `to_units`; never the text).
- Each typed refusal logs exactly one WARN, with the function as `event` and the `OrgStoreError` reason as `reason`.
- The member-delete trigger runs inside Postgres, where tracing cannot see it. It writes each promotion, disband or memberless result to `sgw_organization_events`, with the deleted member's and the new leader's player and account ids captured at delete time (member rows keep an `account_id` copy for this). Rust logs each row and then stamps `exported_at`. Delivery is at least once: a crash between the log and the stamp re-sends the row at the next startup, so every exported event carries `org_event_id` for deduplication.
  - the character delete logs its own rows at INFO right after it commits (`source = character_delete`);
  - a leave or kick handler logs `remove_member`'s rows at INFO after it commits, with the returned `tx_id` (`source = member_removal`);
  - a sweep at base startup logs anything still unstamped, such as a bare `DELETE` from psql (`source = startup_sweep`).

To find a character delete that changed an organization's leader in SigNoz Logs: `service.name = 'cimmeria-server' AND scope_name = 'org' AND event = 'leader_changed' AND reason = 'character_deleted'`, then narrow on `from_player_id` or `org_id`. `org` reaches SigNoz at DEBUG (`OTEL_FILTER`), so the persistence events are there too.

The default rank masks, the text caps and the name rule are project policy, not recovered data (D-ORG08, D-ORG10, D-ORG21). The live-DB tests are in `persistence/tests/`.

## Entity Definition (OrganizationMember.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `records` | PYTHON | CELL_PRIVATE | Organization membership records |
| `squad` | INT32 | CELL_PUBLIC | Current squad (party) group ID |
| `strikeTeamTimers` | PYTHON | CELL_PRIVATE | PvP change response timeouts |
| `pendingPvPTimers` | PYTHON | CELL_PRIVATE | PvP requests causing org leave |
| `pendingGroups` | PYTHON | CELL_PRIVATE | Groups pending creation (type -> invite list) |
| `pendingJoins` | PYTHON | CELL_PRIVATE | Pending join requests |
| `pendingInvitesByType` | PYTHON | CELL_PRIVATE | Pending invites by org type |

### Client Methods (Server -> Client) -- 18 methods

| Method | Args | Purpose |
|--------|------|---------|
| `onOrganizationInvite` | InviterName, OrgType, RequestID, Name, IsStrikeTeam | Invitation prompt |
| `onOrganizationJoined` | OrgId, OrgType, Rank, NewMember | Joined notification |
| `onOrganizationLeft` | Reason, OrgId | Left notification |
| `onMemberJoinedOrganization` | MemberName, MemberId, OrgId, Rank, NewMember | Roster update: join |
| `onOrganizationRosterInfo` | OrgId, ARRAY\<RosterInfo\> | Full roster sync |
| `onMemberLeftOrganization` | MemberId, Reason, OrgId, MemberName | Roster update: leave |
| `onMemberRankChangedOrganization` | MemberId, Rank, OrgId, MemberName | Rank change |
| `onStrikeTeamUpdate` | OrgId, PvPValue | PvP status change |
| `onPvPOrganizationLeaveRequest` | OrgId, PvPValue | Confirm PvP flag change |
| `onOrganizationNameUpdate` | OrgId, Name | Name change |
| `onOrganizationExperienceUpdate` | OrgId, Experience (UINT64) | XP update |
| `onOrganizationMOTDUpdate` | OrgId, MOTD | MOTD change |
| `onOrganizationNoteUpdate` | OrgId, Name, Note | Member note |
| `onOrganizationOfficerNoteUpdate` | OrgId, Name, Note | Officer note |
| `onOrganizationCashUpdate` | OrgId, Cash (UINT64) | Cash update |
| `onOrganizationRankUpdate` | OrgId, RankIds, RankFlags | Rank permissions |
| `onOrganizationRankNameUpdate` | OrgId, RankIds, RankNames | Custom rank names |
| `onSquadLootType` | OrgId, LootType | Loot mode |

### Cell Methods -- 30+ methods

Key exposed (client-invoked) methods:

| Method | Args | Purpose |
|--------|------|---------|
| `organizationInviteResponse` | RequestID, Response | Accept/decline invite |
| `organizationLeave` | OrgId | Leave organization |
| `organizationMOTD` | OrgId, MOTD | Set MOTD |
| `organizationNote` | OrgId, Note | Set member note |
| `organizationOfficerNote` | OrgId, Name, Note | Set officer note |
| `organizationSetRankPermissions` | OrgId, Rank, Permissions | Set rank permissions |
| `organizationSetRankName` | OrgId, Rank, Name | Set rank name |
| `squadSetLootMode` | LootMode | Change squad loot mode |
| `organizationTransferCash` | OrgId, Cash | Deposit/withdraw cash |
| `BroadcastMinimapPing` | OrgId, Location | Ping minimap for group |
| `strikeTeamResponse` | OrgId, Response | Respond to PvP change |
| `pvpOrganizationLeaveResponse` | OrgId, Response | Confirm PvP leave |

### Base Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `organizationInvite` | YES | OrgId, PlayerName | Invite by org ID |
| `organizationInviteByType` | YES | OrgType, PlayerName | Invite by org type (auto-create) |
| `organizationKick` | YES | OrgId, PlayerName | Kick member |
| `organizationRankChange` | YES | OrgId, PlayerName, Rank | Change member rank |

## Organization Types

| Type | Purpose | Example |
|------|---------|---------|
| Squad (0) | Temporary party of up to six, never persisted | Dungeon group |
| Team (1) | Small persistent group; ranks Member, SeniorMember, Leader | Regular group of friends |
| Command (2) | Persistent guild; ranks Initiate to Leader, vault and treasury | Player guild |

## Data References

- **Enumerations**: `EOrganizationType`, `EOrganizationRank`, `EReasons`
- **Custom types**: `RosterInfo` (roster data structure)
- **Related**: `GroupAuthority.def` manages group lifecycle

## RE Priorities

1. **Organization persistence** - Resolved: `db/sgw/Organizations/` and `base::organization` (see [Persistence](#persistence))
2. **Rank permissions** - Resolved: the 26 `EOrganizationPermission` bits, `OrgPermission` in `cimmeria_entity::organization`
3. **Loot modes** - Resolved: `EGroupLootType` has two values, RoundRobin (0) and FreeForAll (1)
4. **Group authority integration** - How `GroupAuthority` entity manages org lifecycle
5. **Organization vault** - Cross-entity vault storage protocol

## Related Docs

- [group-system.md](group-system.md) - GroupAuthority that manages organizations
- [chat-system.md](chat-system.md) - Organization channels (command, officer, squad)
- [inventory-system.md](inventory-system.md) - Organization vault
