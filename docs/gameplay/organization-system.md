---
title: "Organization System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Organization System

> **Last updated**: 2026-09-27
> **Status**: Squads work (ORG-03, ORG-04): invite, accept, leave, kick, loot mode, disconnect, gate travel, squad chat, the minimap ping and the GM `.squad_*` commands, as cell state; see [group-system.md § Squads](group-system.md#squads-org-03). Teams and Commands (organizations campaign, [docs/analysis/organizations/](../analysis/organizations/README.md)) are persisted (ORG-02) and can be founded (ORG-05), restored at login, left and disbanded (ORG-06), invited into, kicked from and re-ranked (ORG-07, [below](#invite-kick-and-rank-change-org-07)), have team, command and officer chat (ORG-09, [below](#team-command-and-officer-chat-org-09)), and have a MOTD, member and officer notes and a rank editor (ORG-08, [below](#motd-notes-and-the-rank-editor-org-08)), with a GM suite (ORG-10, [below](#gm-suite-org-10)). **Feature-complete server-side at the ORG-11 close-out and not yet client-verified**: the owner's two-client UAT ([organizations-uat.md](../guides/organizations-uat.md)) is next. The vault and the treasury belong to the Bank and Vault campaign: the Team and Command vaults open and take moves (BV-07), and the treasury takes deposits and withdrawals ([below](#the-treasury-bank-vault-bv-08), BV-08).

## Overview

Organizations are persistent player groups: Commands (guilds), Squads (parties), and Teams (strike teams). Each organization has a roster, rank hierarchy with permissions, shared cash, experience, MOTD, and member/officer notes. The system supports inviting, kicking, rank changes, loot modes, minimap pings, PvP strike team status, and organization vault storage.

The `OrganizationMember` interface in `entities/defs/interfaces/OrganizationMember.def` is the largest gameplay interface by property + method count.

## Implementation Status

What exists after the campaign's contract packet (ORG-01), the squad core (ORG-03), creation (ORG-05), the Team and Command lifecycle (ORG-06), invite, kick and rank change (ORG-07), texts and the rank editor (ORG-08), organization chat (ORG-09) and the GM suite (ORG-10). **DONE** in the table below means implemented server-side and covered by tests; nothing in it has been run in a real client yet ([ORG-UAT](../analysis/organizations/work-packets.md#org-uat-owner-two-client-uat-colo)). The open owner questions and known gaps are in the [campaign README](../analysis/organizations/README.md#open-questions-for-the-owner).

- **Models** in `cimmeria_entity::organization` ([`crates/entity/src/organization/`](../../crates/entity/src/organization/)): `OrgType`, `OrgRank` with the ranks each type uses, the 26 `OrgPermission` bits with the 12 (Team) and 14 (Command) bits the client's rank editors expose, `OrgLeaveReason`, `SquadLootType`, the id-space constants, the default rank permissions and `org_text`, the one implementation of the text rules (lengths, forbidden characters, the name normaliser and its uniqueness key). Every enum value is pinned against `entities/defs/enumerations.xml`.
- **Inbound decoders** in `cimmeria-wire`: cell methods 8–19 and SGWPlayer cell method 94 `onOrganizationCreation` ([`crates/wire/src/cell/cell_methods/organization/`](../../crates/wire/src/cell/cell_methods/organization/)), and base methods 0xCF–0xD2 ([`crates/wire/src/base/organization.rs`](../../crates/wire/src/base/organization.rs)). Each bounds a `WSTRING`'s declared length by the bytes left before allocating, and rejects truncation, trailing bytes and unpaired surrogates. CM 10 rejects a non-finite coordinate, and CM 19's signed amount decodes to a `CashDir` (positive deposits, negative withdraws, zero is rejected). CM 13, 14, 15, 17 and 94 used to drop their text; they now read it.
- **Dispatch.** The cell router ([`crates/cell-methods/src/cell/cell_methods/organization/`](../../crates/cell-methods/src/cell/cell_methods/organization/)) decodes cell methods 8–19 and routes on the id each carries (D-ORG05, D-ORG06): CM 8 with a cell-issued request id, CM 9 with a squad-range org id, and CM 18 go to the squad handlers in `squad/`; CM 8 with a base request id, and CM 9, 10 and 13-17 with a Team or Command id, are forwarded to the base (`OrgCellToBase::ForwardCellCall`), and CM 19 with one as `OrgCellToBase::TransferCash`; CM 11 and 12 are refused as unsolicited; CM 94 goes to the creation handler in `creation/` ([Creation](#creation-org-05)); a squad-range id on a method squads do not have logs `UNIMPLEMENTED` at DEBUG on the `org` target (the text length, never the text) and is answered with `onErrorCode` and the feedback line "Organizations are not available yet." ([Cell routing](#cell-routing)). The base arm for 0xCF–0xD2 ([`crates/base/src/base/dispatch/organization.rs`](../../crates/base/src/base/dispatch/organization.rs)) forwards `organizationInviteByType` type 0 (after the Ignore check) and `organizationKick` with a squad id to the cell, refuses a type above 2, answers a rank change with a squad id with the "not available yet" pair, and hands every Team and Command call to the ORG-07 handlers.
- **Outbound serializers** for client methods 34–51, `onOrganizationCreationResult` (134) and `launchOrganizationCreation` (135) in [`crates/wire/src/cell/client_methods/organization/`](../../crates/wire/src/cell/client_methods/organization/) and `player.rs`, each byte-tested. The squad handlers send 34–40 and 51.
- **Cell↔base messages** `CellToBaseMsg::Org(OrgCellToBase)` and `BaseToCellMsg::Org(OrgBaseToCell)`. `SquadInvite` and `SquadKick` reach the cell's squad handlers. On the base, a forwarded CM 9 and `GmDisband` reach the ORG-06 handlers, a forwarded CM 8, `GmJoin` and `GmRank` the ORG-07 handlers, a forwarded CM 13-17 the ORG-08 handlers, and the creation messages (`RegistrarOpen`, `Create` and `GmCreate` to the base; `RegistrarEligible` and `CreateResult` back) the ORG-05 creation handlers; a forwarded CM 10 is answered "not available yet", and `TransferCash` goes to the treasury handler ([The treasury](#the-treasury-bank-vault-bv-08), BV-08).
- **Squads** ([group-system.md § Squads](group-system.md#squads-org-03)): the service-wide `SquadRegistry` on `SpaceManager` and `CellEntity::squad_id`.

The schema and the base-side persistence layer came with ORG-02; see [Persistence](#persistence).

| Feature | Status | Notes |
|---------|--------|-------|
| Organization types | DEFINED | Command, Squad, Team in entity defs; typed models in `cimmeria_entity::organization` |
| Invite response | DONE | `organizationInviteResponse` (CM 8): squads (ORG-03); Teams and Commands on the base, single use and re-validated under ORG-LOCK (ORG-07, [below](#invite-response)) |
| Leave | DONE | `organizationLeave` (CM 9): squads (ORG-03); Teams and Commands on the base, with D-ORG12's leader rule and D-ORG20's vault check before a last-member disband (ORG-06, [below](#login-restore-presence-leave-and-disband-org-06)) |
| Minimap ping | PARTIAL | `BroadcastMinimapPing` (CM 10): squads validated (own squad, one a second) and logged, never relayed, since no client method shows another member's ping (ORG-E1 Q3, ORG-04); a Team or Command id is forwarded to the base and answered "not available yet" |
| Strike team (PvP) | REFUSED | `strikeTeamResponse` (CM 11): refused as unsolicited, since no strike-team request is ever sent (CAT-M-16) |
| PvP leave confirmation | REFUSED | `pvpOrganizationLeaveResponse` (CM 12): refused as unsolicited (CAT-M-17) |
| MOTD | DONE | `organizationMOTD` (CM 13): `MOTD` bit, D-ORG10 text, [45] to every online member (ORG-08, [below](#motd-notes-and-the-rank-editor-org-08)) |
| Member note | DONE | `organizationNote` (CM 14): the actor's own note, `RosterNotes` bit, [46] to every online member (ORG-08) |
| Officer note | DONE | `organizationOfficerNote` (CM 15): `OfficerNotes` bit, a target in the same organization and below the actor, [47] only to members holding `OfficerNotes` (ORG-08) |
| Rank permissions | DONE | `organizationSetRankPermissions` (CM 16): `AlterPerms`, D-ORG09 (2), (3), (6) through `OrgPermission::apply_edit` (D-ORG22), the `Leader` row refused, [49] to every online member (ORG-08) |
| Custom rank names | DONE | `organizationSetRankName` (CM 17): `RankNames`, D-ORG09 (2), (3), D-ORG10 / D-ORG23 names, [50] to every online member (ORG-08) |
| Loot mode | DONE | `squadSetLootMode` (CM 18): the leader only, 0 or 1, then `onSquadLootType` to every member |
| Cash management | DONE | `organizationTransferCash` (CM 19): a Team or Command id is forwarded as `TransferCash`; the base moves naquadah between the wallet and the treasury under the organization lock, `DepositCash` or `WithdrawCash` by direction, and sends [48] to every online member (bank-vault BV-08, [below](#the-treasury-bank-vault-bv-08)). Not yet client-verified |
| Creation | DONE | A registrar NPC opens the naming dialog (`launchOrganizationCreation`, 135) for an eligible player; `onOrganizationCreation` (SGWPlayer CM 94) founds the Team or Command against that offer and answers with `onOrganizationCreationResult` (134) and the founder's roster. GM `.org_create` skips the NPC. See [Creation](#creation-org-05) (ORG-05) |
| Invite issue / kick / rank change | DONE | Squads: 0xD0 type 0 and 0xD1 with a squad id (ORG-03). Teams and Commands: 0xCF, 0xD0 types 1 and 2, 0xD1 and 0xD2 under ORG-LOCK and D-ORG09 (ORG-07, [below](#invite-kick-and-rank-change-org-07)); GM `.org_join` and `.org_rank`. A type above 2 is refused; a rank change with a squad id answers "not available yet" |
| Roster info | DONE | `onOrganizationRosterInfo` (38): squads (ORG-03); Teams and Commands at every world entry, followed by `onMemberJoinedOrganization` (37) for each online member, and presence updates on login and logout (ORG-06) |
| Disband | PARTIAL | The last member leaving, and `.org_disband <orgId>` for GMs (ORG-06); both refused while the vault is not empty |
| Experience tracking | NOT IMPL | `onOrganizationExperienceUpdate` (client method 44) is sent in the state push, always with 0: nothing in the client or data says how an organization earns experience (out of scope) |
| Persistence | IMPLEMENTED | Tables, constraints, the leader trigger and the locked write API (ORG-02, [Persistence](#persistence)); the creation handlers (ORG-05), the ORG-06 and the ORG-07 handlers use them |
| Organization chat | DONE (not client-tested) | Team (3), command (5) and officer (6) lines are handled on the base and reach the online members of the speaker's Team or Command; officer needs `OfficerChat` (ORG-09, [below](#team-command-and-officer-chat-org-09)). Squad chat (4) is ORG-04 |
| Organization vault | PARTIAL | Storage (`sgw_organization_vault_items`), the Team/Command Banker open and the vault session (bank-vault BV-07a); moves are BV-07b. See [inventory-system.md](inventory-system.md#opening-a-team-or-command-vault) |
| GM tools | DONE | `.squad_invite`, `.squad_join`, `.squad_info`, `.org_create`, `.org_disband`, `.org_join`, `.org_rank`, `.org_info`, `.org_list`, `.org_set_perms` and `/ReloadOrganizations` (SGWGmPlayer 164), GameMaster-gated ([GM suite](#gm-suite-org-10)) |

## Creation (ORG-05)

A Team or Command is founded at an **Organization Registrar**, or by a GM with `.org_create`. The client sends only the name (SGWPlayer cell method 94); the type is implied by which dialog the server opened, so the server remembers what it offered.

1. **Right-click a registrar.** A registrar is recognised by its seed data alone: the `INT_Organization` bit (64) on `entity_templates.interaction_type`, which also gives the organization cursor, plus exactly one of the 2009 server's registrar interaction sets in `static_interaction_sets`: 7447 for a Team, 7448 for a Command (`deprecated/python/common/Constants.py`). `try_open_org_registrar` ([`crates/cell-interactions/src/cell/interactions/org_registrar.rs`](../../crates/cell-interactions/src/cell/interactions/org_registrar.rs)) runs beside the DHD check, after any content bind. A click from beyond the 5-unit interact range, or from another space, gets a line ("You are too far away from the registrar.").
2. **Eligibility (base).** The cell forwards `OrgCellToBase::RegistrarOpen`. The base refuses a player who already belongs to an organization of that type (D-ORG18) with a line, and otherwise answers `OrgBaseToCell::RegistrarEligible`.
3. **The offer (cell).** The cell records a pending creation keyed by `player_id` (`SpaceManager::org_creations`, [`crates/cell-world/src/cell/org_creation/`](../../crates/cell-world/src/cell/org_creation/)): the type, the registrar, the space, a 5-minute expiry and 3 attempts. It then sends `launchOrganizationCreation` (135) with the type, which opens `CreateTeamWin` or `CreateCommandWin`.
4. **The name (cell).** Cell method 94 is honoured only against that offer, in the same space, before it expires, and with one name at a time in flight. The name must pass D-ORG10 (`org_text::validate`). A refused name costs one attempt. When the three are spent, the offer stays spent until it expires, and clicking the registrar again does not refill it. The normalised name goes to the base as `OrgCellToBase::Create`, with the offer's type.
5. **Founding (base).** `found_organization` ([`crates/base-session/src/base/organization/creation/`](../../crates/base-session/src/base/organization/creation/)) runs `create_org` and debits the D-ORG15 cost in one transaction. The cost is `ORG_CREATE_COST_TEAM` / `ORG_CREATE_COST_COMMAND`, both 0 for now. A taken name, a second organization of the type, or a short purse is refused **before** an organization id is drawn: the id sequence is `NO CYCLE`, and a refused insert would still use up an id. Two transaction-scoped advisory locks (the founder's, then the name's) serialise concurrent creations, so neither check goes stale before the insert.
6. **The answer.** The base answers the client itself, then tells the cell (`OrgBaseToCell::CreateResult`) to close the offer or charge it the attempt.

On success the founder gets `onOrganizationCreationResult` (134) `(1, 0)` first, then the organization's state through ORG-06's `push_org_state` with `aNewMember = 1` (35 as Leader, the name, MOTD, cash, experience, rank permissions and names, the roster, and 37 with the founder's own entity id), then a line ("Team "…" founded. You are its leader."). Every refusal gets 134 `(0, RetCode)` and a line: the client shows no text for either byte (ORG-E1 Q4). The `RetCode` values are project policy, pinned in `crates/wire/src/cell/client_methods/player.rs` (`org_creation_ret_code`): 1 name taken, 2 name invalid, 3 already in an organization of the type, 4 insufficient funds, 5 no pending creation or expired, 6 rate limited or already in flight, 7 server error.

The offer ends on a creation, on expiry, on a change of space (checked when the name arrives, so gate travel needs no hook), and on `DisconnectEntity`.

**GM `.org_create <team|command> <name>`** founds an organization the GM leads, with no registrar and no offer. The console forwards `OrgCellToBase::GmCreate`; the base re-reads the access level from its own session (D-ORG13) and applies every other rule above.

**Telemetry** (target `org`): `org.registrar_open` and `org.create` spans and one INFO outcome row per action, written by whichever side decides it, plus `org.gm_action` (`action = gm_org_create`) for the console command. All count on `org_actions_total{action, outcome, reason}`. The reasons and transitions are in the [observability.md](../architecture/observability.md) catalog row.

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

`org_vault_is_empty_sql` is true when the organization has no rows in `sgw_organization_vault_items` and no treasury cash (bank-vault BV-07). Its Rust twin `api::org_vault_is_empty` runs the SQL function, so the two cannot disagree. The vault's key to the organization is `ON DELETE RESTRICT`, so the trigger's delete of an organization whose vault still holds items could not succeed; the predicate is why it never tries.

Every character delete keeps the lock order in the database, whatever issued it. A `BEFORE DELETE` trigger on `sgw_player` (`org_player_before_delete()`) locks the character's organizations in `org_id` order after the character row and before the member rows cascade. An account delete deletes all its characters in one statement, so a `BEFORE DELETE` trigger on `account` (`org_account_before_delete()`) first locks every character row in `player_id` order, then all their organizations in `org_id` order. The game's delete path, `organization::character_delete::delete_character`, adds an ownership check (another account's request locks nothing) and logs the trigger's results after its commit.

### The write API (ORG-LOCK)

The code is in [`crates/base-session/src/base/organization/`](../../crates/base-session/src/base/organization/):

- `api.rs` (the ORG-API the Bank campaign builds on): `lock_org(tx, org_id)` takes `SELECT ... FOR UPDATE` on the organization row and returns its `OrgHeader`; `member_access_locked(tx, org_id, player_id)` takes that lock and returns the member's `OrgAccess` (type, rank, permissions) read under it; `OrgAccess::system(tx, org_id, actor)` builds one for a GM or server action and logs it (`org.gm_access` or `system_action`; ORG-10 renamed the GM row so that `org.gm_action` is only ever a GM command's result row); `org_vault_is_empty(tx, org_id)` is the vault predicate (D-ORG20, D-BV13). There is no pool-level membership check.
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

## Login restore, presence, leave and disband (ORG-06)

The handlers are in [`crates/base-session/src/base/organization/handlers/`](../../crates/base-session/src/base/organization/handlers/).

### Login restore

At every world entry (`onClientReady`, after the contact lists), the base sends one reliable bundle per Team and Command the character belongs to, Team first. The order comes from the client's handlers (ORG-E1 Q1, [organization-restoration.md](../reverse-engineering/findings/organization-restoration.md)):

1. `onOrganizationJoined` [35] with `aNewMember = 0`.
2. The name [43], MOTD [45], cash [48] and experience [44].
3. The rank permissions [49], and the custom rank names [50] (only renamed ranks; the others keep the client's default label).
4. The roster [38]. The client stores every roster row with member id 0, which it shows as "Offline".
5. `onMemberJoinedOrganization` [37] with `aNewMember = 0` and the live entity id for each online member, the character included. This is the only message that sets a roster id.

The same push (`push_org_state`) is what ORG-05 sends after a creation. Because it runs at every world entry, gate travel refreshes the entity id the other members' rosters hold.

### Presence

"Online" means a session whose character is in the world (`listed_online`). When a character enters the world, the other online members of each of its organizations get [37] with its entity id. When its session ends they get [37] with id 0: the client's handler overwrites the id of an existing name in place, so the row turns Offline and stays. `onMemberLeftOrganization` [39] would delete the row, so logout never uses it.

Every teardown path announces offline exactly once:

- `helpers::destroy_client_entities` (client disconnect, inactivity timeout, send error, duplicate login) announces a session that was still listed online, on its own task (`session_presence`), with the teardown's `disconnect_reason`.
- `logOff` announces and clears `listed_online`, so the disconnect that reaps a full exit does not announce again.

The same hook tells contact-list watchers (CM 89 `LoggedInStatus`, offline). Before ORG-06 only `logOff` did, so a crash or timeout left a character "online" in every friend list (audit A-35).

### Leave

`organizationLeave` (CM 9) with a Team or Command id is forwarded to the base with the character the cell's entity plays. The base checks that the entity is still that character's session in the world, then, under ORG-LOCK:

| Case | Result |
|---|---|
| Not a member of that organization | Refused (`not_member`, CAT-M-04) with a feedback line |
| The leader, while other members remain | Refused (`leader_cannot_leave`, D-ORG12); the organization's state is re-sent and a line explains why |
| The last member | The organization is disbanded, unless the vault holds anything (`vault_not_empty`, D-ORG20, with the state re-sent) |
| Anyone else | The member row goes; the leaver gets `onOrganizationLeft` [36] with `Requested`, and the online members `onMemberLeftOrganization` [39] |

### Disband

`.org_disband <orgId>` is a GM console command. The cell forwards it (`OrgCellToBase::GmDisband`); the base re-reads the caller's access level from its own session (GameMaster or above), locks the organization, and refuses while the vault holds anything. It also disbands a memberless organization (D-ORG20's recovery case) once its vault is empty. Every online member gets `onOrganizationLeft` [36] with `Disbanded`, and the GM gets a line with the member count.

Beside every `onOrganizationLeft` [36] to an online player (a leave, a disband, a kick), the base sends the cell `OrgBaseToCell::OrgMembershipEnded { player_id, entity_id, org_id, reason }` after the commit. The cell logs `org.membership_ended` and ends an open Team or Command vault session of that organization (`vault_session_closed reason=org_left`, bank-vault BV-07).

The vault predicate reads the real vault and treasury (bank-vault BV-07); its live-DB guards are in `persistence/tests/vault.rs`. The ORG-06 refusal tests still drive `vault_not_empty` through the test-only `VAULT_EMPTY_OVERRIDE`, so they need no vault rows.

### Telemetry (ORG-06)

Everything logs on the `org` target, and each action counts once on `org_actions_total{action, outcome, reason}` (`action` = `login_restore`, `leave`, `disband`):

| Question | SigNoz Logs filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|
| A character's world-entry restore | `event = 'org.login_restore' AND player_id = <id>` (`org_count`), then `event = 'org.state_push'` per organization (`roster_size`, `online_members`) |
| Presence | `event IN ('member_online', 'member_offline') AND player_id = <id>` (`recipients`, `disconnect_reason`) |
| A leave, and why it was refused | `event = 'org.leave' AND player_id = <id>` (`outcome`, `reason`) |
| A GM disband | `event = 'org.disband' AND org_id = <id>`, and `event = 'org.gm_action'` for the GM's identity |
| A message a member did not get | `event = 'org.send_failed'` (`reason`, `target_player_id`) |
| A forwarded call from a stale session | `event = 'org.actor_mismatch'` |

## Invite, kick and rank change (ORG-07)

The handlers are in [`crates/base-session/src/base/organization/handlers/`](../../crates/base-session/src/base/organization/handlers/) (`invite.rs`, `invite_response.rs`, `kick.rs`, `rank.rs`, `gm.rs`, `broadcast.rs`). The base methods 0xCF-0xD2 reach them from [`crates/base/src/base/dispatch/organization.rs`](../../crates/base/src/base/dispatch/organization.rs); the invite response arrives as a forwarded CM 8. Every check that authorizes runs under ORG-LOCK (D-ORG04): one transaction, the organization row locked first, the actor's rank and permissions read inside it. Every refusal gets a feedback line (`answer.rs`); `onErrorCode` is not sent, because the client has no organization text for it (ORG-E1 Q4).

### Invite

`organizationInvite` (0xCF, an org id) and `organizationInviteByType` (0xD0, type 1 or 2). Invite-by-type only ever finds the inviter's existing Team or Command; it never creates one (CAT-M-02; only type 0, a squad, founds on accept).

1. The inviter's rate limit, from their own session: five invites in any 30 s.
2. The invitee: an online character other than the inviter (exact name, then a unique case-insensitive match), not mid world entry or gate travel, and not ignoring the inviter (SS-C1's cached Ignore list, by character id and name).
3. Under the lock: the inviter is a member whose rank holds `Invite` (CAT-M-01), and the invitee is in no organization of that type (D-ORG18).
4. The invite is recorded **on the invitee's session** (`organization::invites`), keyed by the invitee's character and the request id (D-ORG06). Request ids carry `BASE_INVITE_REQUEST_FLAG` (bit 29) and count up from one process-wide counter, never reused. One pending invite per inviter and invitee, five per invitee, 60 s to answer. The session dies with its invites on every teardown path; `logOff` (either variant) drops them too, since a return to character select keeps the session.
5. The invitee gets `onOrganizationInvite` [34] (inviter name, type, request id, organization name); the inviter gets a line.

The base checks the Ignore list before it forwards a **squad** invite (0xD0 type 0) to the cell as well (carried from ORG-03), and refuses it the same way.

### Invite response

`organizationInviteResponse` (CM 8) with bit 29 set is forwarded by the cell. The base takes the entry from the responder's own session in one step, so the first response consumes it, accept or decline (CAT-M-18); a replay, an expired entry and another player's id all read "That invitation is no longer valid." (the log tells them apart). A decline tells an online inviter. An accept re-validates under the lock: the organization still exists, the inviter is still a member whose rank holds `Invite` (so an inviter kicked or demoted in between no longer vouches), and the responder is in no organization of that type. Teams and Commands have no member cap in the ledger, so there is no room check. The responder joins at the type's entry rank (D-ORG07: Team `Member` 2, Command `Initiate` 1), gets the organization's full state (`push_org_state`, `onOrganizationJoined` [35] with `aNewMember = 1` first), and every other online member gets `onMemberJoinedOrganization` [37] with `aNewMember = 1`.

`add_member` takes the joining character's creation advisory lock (ORG-05's key), after its membership check. An accept and a Team creation by the same character therefore serialise, and the losing creation is refused before it draws an organization id.

### Kick and rank change

| Check (all under the lock) | Kick (0xD1) | Rank change (0xD2) |
|---|---|---|
| The actor is a member | `not_member` | `not_member` |
| The target, by name among the organization's members (offline members included) | `target_not_member`, `target_ambiguous` | same |
| Not the actor | `self_target` | `self_target` |
| The rank is one the type uses: 0, Team rank 5 and anything above 8 are refused (D-ORG09 (5)) | | `rank_not_in_type` |
| Never `Leader` (D-ORG09 (4)) | | `leader_not_assignable` |
| A different rank from the current one | | `rank_unchanged` |
| The bit: `Eject`; `Promote` to raise, `Demote` to lower (D-ORG09 (1)) | `missing_permission` | `missing_permission` |
| The actor's rank strictly above the target's, and above the rank assigned (D-ORG09 (2)) | `rank_too_low` | `rank_too_low` |

After a kick commits, the kicked player (if online) gets `onOrganizationLeft` [36] with `Kicked` and a line, and the cell gets `OrgBaseToCell::OrgMembershipEnded` with `Kicked` (the Bank's vault-session hook); every remaining online member gets `onMemberLeftOrganization` [39] with the kicked player's live entity id, or 0 when offline. After a rank change, every online member, the target included, gets `onMemberRankChangedOrganization` [40]. A rank change with a squad id keeps the "not available yet" answer: whether `/squadpromote` uses 0xD2 is unconfirmed (ORG-E1 follow-up 1).

The client reads rank permissions from the login push ([49]); no member-side cache of a rank's permissions exists on the server, so a demotion takes effect at the next authorization, which always reads under the lock.

### Cell routing

The cell router forwards every Team and Command cell method to the base: CM 8 with a base request id, and CM 9, 10 and 13-17 with a Team or Command id (`OrgCellToBase::ForwardCellCall`), and CM 19 as `OrgCellToBase::TransferCash`. CM 13-17 go to the ORG-08 handlers there; CM 10 (the minimap ping) is answered "not available yet"; CM 19 is the treasury transfer ([The treasury](#the-treasury-bank-vault-bv-08), bank-vault BV-08). CM 11 `strikeTeamResponse` and CM 12 `pvpOrganizationLeaveResponse` are refused on the cell as unsolicited, since no strike-team or PvP-leave request is ever sent (CAT-M-16, CAT-M-17). Squad-range ids on methods squads do not have keep the cell's "not available yet" answer.

### `broadcast_to_org`

`api::broadcast_to_org(ctx, org_id, method_idx, args, required)` sends one client method to every online member of a Team or Command, or, with `required`, only to members whose rank holds all of those bits (the Bank uses `ViewBankLogs`). Call it after the commit: it reads the roster and the rank table without the lock. Kick and rank change use it.

### GM commands

`.org_join <orgId> [player]` and `.org_rank <player> <rank> [orgId]` are forwarded to the base (`OrgCellToBase::GmJoin`, `GmRank`), which re-reads the GM's access level from the session playing that character on that entity (a recycled entity id finds nobody; `.org_disband` now checks the same way). They skip the member permission and rank checks, and keep every data rule: `.org_join` adds an online player at the entry rank, or as `Leader` to a memberless organization, and refuses a second organization of the type; `.org_rank` refuses `Leader` and ranks the type does not use, and without an org id acts on the member's only Team or Command.

### Telemetry (ORG-07)

Everything logs on the `org` target; each action counts once on `org_actions_total{action, outcome, reason}` (`action` = `invite` \| `invite_response` \| `kick` \| `rank_change` \| `gm_org_join` \| `gm_org_rank` \| `strike_team_response` \| `pvp_leave_response`).

| Question | SigNoz Logs filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|
| Who invited whom, and why it was refused | `event = 'org.invite' AND player_id = <id>` (`outcome`, `reason`, `target_player_id`, `request_id`, `actor_rank`) |
| What happened to an invite | `request_id = <id> AND event IN ('invite_created', 'invite_consumed', 'invite_expired', 'org.invite_response')` |
| Kicks and rank changes | `event IN ('org.kick', 'org.rank_change') AND org_id = <id>` (`actor_rank`, `target_rank`, `to_rank`), then `event IN ('member_left', 'rank_changed')` for the before and after |
| GM joins and rank sets | `event IN ('org.gm_join', 'org.gm_rank')`, or `event = 'org.gm_action' AND action IN ('gm_org_join', 'gm_org_rank')` |
| Where a cell call went | `event = 'org.forward' AND method_index = <n>` (`route` = `squad` \| `base` \| `rejected`) |
| A fanout that missed someone | `event IN ('org.send_failed', 'org.broadcast_failed')` (`what`, `reason`) |

## Team, command and officer chat (ORG-09)

`sendPlayerCommunication` on team (3), command (5) or officer (6) is handled on the base, which holds the memberships; these lines never reach the cell. The code is [`crates/base-session/src/base/organization/handlers/chat.rs`](../../crates/base-session/src/base/organization/handlers/chat.rs), called from the base chat dispatch ([`crates/base/src/base/dispatch/chat.rs`](../../crates/base/src/base/dispatch/chat.rs)) after the flood limit, the channel allowlist, the GM mute and the text rules, none of which it repeats. See [chat-system.md § Organization channels](chat-system.md#organization-channels) for the player's view.

- **Who hears it.** The speaker's Team for 3, their Command for 5 and 6, read from the database per line. `broadcast_to_org`'s fanout sends `onPlayerCommunication` [28] to every member in the world now, in any space, and the speaker gets their own copy.
- **Officer is a Command channel.** Only the Command's members whose rank holds `OfficerChat` get an officer line, and a speaker without it is refused. `OfficerChat` is in the Command rank editor only (audit A-12), the officer ranks exist only in a Command, and the legacy mail enum has `MAIL_ToCommandOfficers` with no Team twin. A Team's `SeniorMember` default mask includes the bit, but a Team has no officer channel.
- **No registration.** Nothing sends `onChatJoined`: the client hardcodes 3, 5 and 6 (D-ORG14, ORG-E1 Q5).
- **Refusals.** Each is one feedback line: "You are not in a team.", "You are not in a command.", "Your rank cannot speak on the officer channel.", or "Organization chat is unavailable right now. Try again later." (no database, a database error).
- **Authorization read.** The speaker's rank and mask come from the unlocked display read `load_memberships`. A chat line changes nothing, so it takes no org lock; a demotion committing while a line is in flight can let that one line through, the same window the fanout's recipient filter has.

### Telemetry (ORG-09)

| Question | SigNoz Logs filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|
| Did a player's org line go out, and to how many | `event = 'org.chat' AND player_id = <id>` (`outcome`, `channel`, `org_id`, `recipients`, `text_units`) |
| Why a line was refused | `event = 'org.chat' AND outcome = 'rejected'` (`reason` = `not_in_org` \| `missing_permission` \| `rate_limited` \| `text_invalid` \| `no_db` \| `db_error` \| `not_in_world`) |
| A member or the speaker who missed a line | `event = 'org.send_failed' AND what IN ('chat', 'chat_echo')` (`reason`) |

A muted speaker's line writes no `org.chat` row: look for `event = 'chat.muted_refused'` on the `chat` target. Every row counts on `org_actions_total{action = "chat"}`.

## GM suite (ORG-10)

Every GM organization command is GameMaster-gated twice: the cell's `.` console runs only a GameMaster's line, and the base re-reads the access level from the session that plays the forwarded character on the forwarded entity (D-ORG13). No privilege bit travels in a cell-to-base message.

| Command | What it does |
|---|---|
| `.org_info [player]` | Lists every Team and Command the character (default: you) belongs to: type, name, org id, rank and that rank's permission mask in hex. The character may be offline; an exact name wins, otherwise a case-insensitive match must be unique. |
| `.org_list` | Lists every Team and Command, oldest first, with its member count and leader; at most 50 lines. |
| `.org_set_perms <orgId> <rank> <mask>` | Sets a rank's permission mask (decimal or `0x` hex). The mask goes through the same `OrgPermission::apply_edit` a member's rank editor uses (D-ORG22): only the bits the type's editor shows (12 for a Team, 14 for a Command) take your value, every other bit keeps what is stored, and the GM line names the bits it ignored (the D-ORG09 (6) clamp). The `Leader` row, a rank the type does not use and an edit that changes nothing are refused. The edit runs ORG-08's `rank_permissions_locked` (the one permission-edit path, also used by CM 16) under ORG-LOCK and the organization's order guard; every online member then gets the rank table [49], and when the edit moved `OfficerNotes` that rank's online members get the officer-note sync [47] ([below](#motd-notes-and-the-rank-editor-org-08)). |
| `/ReloadOrganizations` (`gmReloadOrganizations`, `SGWGmPlayer` cell method 164) | Re-sends your own organization state, the same bundle as the world-entry push ([35], [43], [45], [48], [44], [49], [50], [38], then [37] per online member), for every Team and Command you belong to. Nobody else is told anything. Index 164 is in the `SGWGmPlayer` tail, which the dispatch gate refuses to non-GMs before any handler runs. |

`.org_create`, `.org_disband`, `.org_join`, `.org_rank` and the three `.squad_*` commands are described with their packets above and in [group-system.md](group-system.md). No GM command sets organization text, so the D-ORG10 caps have nothing to bypass; `.org_create`'s name goes through the same `org_text::validate` as the registrar's.

**Audit row.** Every GM organization command, refused or not, ends in exactly one INFO `org.gm_action` row on `org` with `action`, `outcome`, `reason` on a refusal, the GM's `account_id` / `player_id` / `entity_id`, the target's `target_account_id` / `target_player_id` where there is one, and `org_id`. For `.org_disband`, `.org_join` and `.org_rank` it is a twin of the command's own row (`org.disband`, `org.gm_join`, `org.gm_rank`), written from the same data and not counted again; for the ORG-10 commands and `.org_create` it is the outcome row. Before ORG-10, a refusal ahead of the organization lock (`not_gm`, a malformed id) left no `org.gm_action` row at all.

| Question | SigNoz Logs filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|
| Everything a GM did to organizations | `event = 'org.gm_action' AND player_id = <GM's player id>`, by time |
| One command's refusals | `event = 'org.gm_action' AND action = 'gm_org_set_perms' AND outcome = 'rejected'`, grouped by `reason` |
| A permission edit's before and after | `event = 'permissions_changed' AND org_id = <id>` (`rank`, `from_mask`, `to_mask`, `wire_mask`, `ignored_bits`; DEBUG) |
| A reload | `event = 'org.gm_action' AND action = 'gm_reload_organizations'` (`count` = organizations re-sent), then `event = 'org.state_push' AND player_id = <id>` |

## MOTD, notes and the rank editor (ORG-08)

The handlers are `texts.rs` (CM 13-15) and `rank_editor.rs` (CM 16-17) in [`crates/base-session/src/base/organization/handlers/`](../../crates/base-session/src/base/organization/handlers/), reached from the cell as forwarded calls ([`org_dispatch.rs`](../../crates/base-world-entry/src/base/world_entry/cell_dispatch/org_dispatch.rs)). Text is checked first (D-ORG10 and D-ORG23: rejected, never truncated). Then each edit runs under ORG-LOCK and under the organization's **order guard** (`order.rs`, an in-process lock per organization taken before the transaction and held until the last send), so two edits' post-commit sends go out in commit order. The rank change (0xD2) and `.org_rank` take the same guard. Every outcome ends in one feedback line, the success included.

| Check (all under the lock) | MOTD (13) | Note (14) | Officer note (15) | Rank permissions (16) | Rank name (17) |
|---|---|---|---|---|---|
| The actor is a member | `not_member` | `not_member` | `not_member` | `not_member` | `not_member` |
| The rank is one the type uses (D-ORG09 (5)) | | | | `rank_not_in_type` | `rank_not_in_type` |
| Not the `Leader` row (D-ORG08) | | | | `leader_row_pinned` | |
| The bit (D-ORG09 (1)) | `MOTD` | `RosterNotes` | `OfficerNotes` | `AlterPerms` | `RankNames` |
| The target: by name among **this** organization's members, not the actor, strictly below the actor (D-ORG09 (2)) | | | `target_not_member`, `target_ambiguous`, `self_target`, `rank_too_low` | | |
| Not the actor's own rank (D-ORG09 (3)) and strictly below it (D-ORG09 (2)) | | | | `own_rank`, `rank_too_low` | `own_rank`, `rank_too_low` |
| Only bits the actor holds move (D-ORG09 (6), D-ORG22) | | | | `changes_unheld_bits` | |

A missing bit is `missing_permission`; bad text is the text rule's own reason (`too_long`, `too_short`, `bidi_control`, `zero_width`, `format_char`, `control_char`, `line_separator`). `RosterNotes` is not in either editor's set, so in practice every rank except `Initiate` may write its own note (the D-ORG08 defaults). The Leader cannot rename rank 8: D-ORG09 (3) and (2) refuse it for everyone, including a GM.

A permission edit stores `(old & !editable_for(type)) | (wire & editable_for(type))`, so a bit the client's editor does not show (`RosterNotes`, `ViewBankLogs`, ...) keeps its value, and only the bits that actually change must be held by the editor. `rank_permissions_locked` is the one permission-edit path; the GM `.org_set_perms` (ORG-10) calls it with a system access, which passes the authority checks but still meets the rank-in-type rule, the `Leader` pin and the clamp. An edit that changes nothing (the same text, the same mask) is `ok` with `after = unchanged`: no write and no fanout, but the line is sent.

After the commit: [45] (MOTD) and [46] (the member's stored name and note) to every online member; [47] only to the members whose rank held `OfficerNotes` in the edit's transaction; [49] with the whole rank table and [50] with every custom rank name, as read under the lock, to every online member.

**Officer-note visibility.** Officer notes reach only ranks holding `OfficerNotes` (CAT-M-10):

- The login push ([38] in `push.rs`) blanks every officer note for a recipient whose rank lacks the bit in the push's own rank read, or has no rank row (fail closed).
- A rank-permission edit that moves `OfficerNotes` sends that rank's online members one [47] per stored officer note, after the [49]: the text on a grant, an empty note on a revoke.
- A rank change (0xD2 or `.org_rank`) between two ranks that differ in the bit does the same for the moved member, after the [40].

### Telemetry (ORG-08)

One INFO span per entrypoint (`org.set_text`, `org.set_rank_permissions`, `org.set_rank_name`) and one INFO outcome row with the same `event`, counted on `org_actions_total` with `action` = `set_text` \| `set_rank_permissions` \| `set_rank_name`.

| Question | SigNoz Logs filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|
| Who changed a text, and why it was refused | `event = 'org.set_text' AND org_id = <id>` (`field`, `from_units`, `to_units`, `target_player_id` for an officer note, `actor_rank`, `reason`) |
| Rank-mask edits | `event = 'org.set_rank_permissions' AND org_id = <id>` (`rank`, `from_mask`, `to_mask`, `wire_mask`; `unheld_mask` on `changes_unheld_bits`) |
| Rank renames | `event = 'org.set_rank_name' AND org_id = <id>` (`rank`, `from_units`, `to_units`) |
| Officer notes shown or hidden after a rank or mask change | `event = 'org.officer_note_sync' AND org_id = <id>` (DEBUG: `show`, `notes`, `members`, `recipients`) |
| A fanout that missed someone | `event = 'org.send_failed' AND what IN ('officer_note', 'officer_note_sync', 'broadcast')` |

## The treasury (bank-vault BV-08)

A Team's or a Command's `sgw_organizations.cash` takes deposits from, and pays withdrawals to, a member's wallet (`sgw_player.naquadah`). The client's vault window sends `organizationTransferCash` (CM 19) with a signed amount: `Team.lua` and `Command.lua` pass the spinner value for a deposit and its negation for a withdrawal, and never zero. The cell forwards a Team or Command id as `OrgCellToBase::TransferCash`; a zero amount is refused on the cell. The base handler is [`crates/base-session/src/base/org_cash/`](../../crates/base-session/src/base/org_cash/).

**Rules.**

- **Bits.** A deposit needs `DepositCash`, a withdrawal `WithdrawCash`; they are separate bits. The defaults give every rank below `Leader` `DepositCash` and not `WithdrawCash`, and the leader everything (D-BV12). There is no cap on withdrawals (D-BV15).
- **No negative or wrapped balance.** The wallet is an `i32` and the treasury an `i64` with `CHECK (cash >= 0)`. A deposit larger than the wallet, a withdrawal larger than the treasury, and a transfer that would take either past its maximum are refused with nothing moved. The client's `i32::MIN` is a withdrawal of 2,147,483,648, which no wallet can take.
- **No Banker check.** The treasury is not the vault. The client shows the buttons only in the vault window, but the server cannot tell which window sent the call. Membership and the bit are the whole authorization, as for every other organization call; the server-authority review cleared this.

**The transaction.** One transaction, in the order `organization::api` § "Lock order" gives a cash change: the actor's `sgw_player` row `FOR KEY SHARE`, then `lock_org` and `member_access_locked` (rank and bits read under the lock), then a plain `UPDATE` of the wallet guarded on the row as it is now (`naquadah >= amount`, or `naquadah + amount <= i32::MAX`), then the treasury `UPDATE` (guarded again behind the Rust checks on the locked balance), then a `sgw_organization_cash_log` row, then the commit. The wallet's balance before and after comes from the guarded `UPDATE`'s `RETURNING`, never from the first read, because `KEY SHARE` does not stop another plain writer of `naquadah`.

**After the commit** every online member gets `onOrganizationCashUpdate` [48] with the new treasury (`broadcast_to_org`), and the actor gets `onCashChanged` (75) with the new wallet and a line ("You deposited 300 naquadah into the team treasury. It now holds 300."). The actor's sends are addressed to the character, not to the entity id the cell sent (D-BV33).

**Refusals.** Each one sends the actor a line, except `actor_mismatch` (no session to tell). A refusal on the wallet resends the wallet, and one on the treasury resends the treasury to a member, so a stale spinner maximum corrects itself. An unknown organization and one the actor is not in get the same line, so a non-member cannot probe which ids exist.

| `reason` | When | Line |
|---|---|---|
| `zero_amount` (cell) | the amount is 0 | "Enter an amount of naquadah to transfer." |
| `actor_mismatch` | the forward no longer names this session's character in the world | none |
| `db_unavailable`, `query_failed`, `player_missing` | no database, a failed statement, or no character row | "The transfer failed. No naquadah was moved." |
| `no_such_org`, `not_a_member` | the organization is gone, or the actor is not in it | "You are not a member of that organization." |
| `no_permission` | the rank lacks the direction's bit (`perm` names it) | "Your rank may not deposit naquadah." / "... withdraw naquadah." |
| `insufficient_player_cash` | a deposit larger than the wallet | "You cannot deposit N naquadah: you have M." |
| `insufficient_org_cash` | a withdrawal larger than the treasury | "You cannot withdraw N naquadah: the treasury holds M." |
| `player_cash_overflow` | the wallet would pass `i32::MAX` | "You cannot carry N more naquadah." |
| `org_cash_overflow` | the treasury would pass `i64::MAX` | "The treasury cannot hold N more naquadah." |

**The log.** Every committed treasury change is one `sgw_organization_cash_log` row (`db/sgw/Organizations/Tables/sgw_organization_cash_log.sql`): the actor (`account_id`, `player_id`, `rank`), the organization, `direction` (`deposit`, `withdraw`, or BV-09's `vault_expansion`), `amount`, both balances before and after, and `tx_id`. CHECKs pin the arithmetic for each direction. It is a sibling of the vault log rather than more columns on it: the vault log's item and slot columns are `NOT NULL` for every row, and a cash row has none. A `ViewBankLogs` reader merges the two with `UNION ALL`, ordered by `(logged_at, tx_id, log_id)`. No foreign keys, so the trail outlives a disband and a character delete.

**Telemetry** (target `bank`): INFO `org_cash_transfer` with `account_id`, `player_id`, `entity_id`, `org_id`, `org_type`, `rank`, `direction`, `amount`, `player_cash_before`/`after`, `org_cash_before`/`after` and `recipients`; WARN `org_cash_rejected` with the same ids, the `reason` above, `perm` and `permissions` on `no_permission`, the balances it read (the same before and after, since nothing moved), and `error` on `query_failed`. INFO span `bank.org_cash_transfer`. The live-DB and `LogCapture` guards are in `org_cash/tests/`, the zero-amount guard in `cimmeria-cell-methods` `organization/tests/router.rs`.

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
| `organizationInviteByType` | YES | OrgType, PlayerName | Invite into the inviter's organization of that type; only a squad (type 0) is founded on accept, a Team or Command never (CAT-M-02) |
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
- [chat-system.md](chat-system.md) - Organization channels (team, squad, command, officer)
- [inventory-system.md](inventory-system.md) - Organization vault
