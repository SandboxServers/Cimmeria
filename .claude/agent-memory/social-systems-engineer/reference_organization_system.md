---
name: reference-organization-system
description: Organization/Guild system Phases 1-3 — rank/permission model, DB schema layout, wire values, OrgAuthority design, persistence, fanout
metadata:
  type: reference
---

## Rust model — `crates/game/src/social/guilds.rs`

`OrgRank` (#[repr(u8)], derives Ord): None=0, Initiate=1, Member=2, SeniorMember=3, Veteran=4, SeniorVeteran=5, Officer=6, SeniorOfficer=7, Leader=8.

`OrgType` (#[repr(u8)]): Squad=0, Team=1, Command=2. `is_persistent()` is true for Team/Command only.

`OrgPermission` — plain `u32` newtype (bitflags crate is NOT in game crate deps). 26 flags from 0x1 (DoNotUse) to 0x2000000 (AllianceCmds). ALL = 0x3FFFFFF, fits in positive i32. Methods: `contains`, `insert`, `remove`, `union`, `bits`, `from_bits`.

`OrgRankConfig { custom_name: Option<String>, permissions: OrgPermission }` — stored per (org_id, rank_value) in DB.

`OrgMember { player_id: i32, character_name: String, rank: OrgRank, notes: Option<String> }` — keyed by player_id in `Org.members`.

`Org { org_id, org_type, name, leader_player_id, motd, cash: i64, members: HashMap<i32, OrgMember>, rank_configs: HashMap<u8, OrgRankConfig> }`.

Member ref uses player_id (FK → sgw_player), matching the contact-list convention.

## DB schema — `db/sgw/Organizations/`

Three tables:
- `sgw_organizations`: org_id (PK, serial), org_type smallint CHECK(1,2), name, leader_player_id (FK RESTRICT), motd, cash bigint, created_at
- `sgw_organization_ranks`: (org_id, rank_value smallint 0-8) PK, custom_name nullable, permission_mask integer
- `sgw_organization_members`: (org_id, player_id) PK, rank_value smallint 0-8, notes, officer_note, joined_at

FKs: organizations.leader_player_id → sgw_player ON DELETE RESTRICT (must transfer/disband first). organization_ranks.org_id → organizations CASCADE. organization_members.org_id → organizations CASCADE. organization_members.player_id → sgw_player ON DELETE CASCADE (invariant: no orphaned roster rows).

Index: `sgw_organization_members_player_id_idx` supports login-time "which orgs does this player belong to?" query.

## database.sql include wiring pattern

For a new subsystem under `db/sgw/`, add `\ir` lines in four places in `db/database.sql`:
1. Sequences block (after existing Sequences)
2. Tables block (after relevant Tables group)
3. Seed block (after existing Seed)
4. The four aggregate files (`sgw/_sequence_ownership.sql`, `sgw/_primary_keys.sql`, `sgw/_foreign_keys.sql`, `sgw/_indexes.sql`) are edited in-place — append to each.

## Wire values confirmed from findings docs + RE

EORG_RANK_* wire UINT8: 0-8 per OrgRank above. EORG_PERM_* 26-bit bitmask per OrgPermission consts. EOrganizationType UINT8: Squad=0, Team=1, Command=2. EPersistentOrganizationType = {Team, Command} only.

Cash field is UINT64 on wire (8B LE); stored as i64/bigint in DB — covers full positive range.

## Phase 3 design: OrgAuthority + Persistence + Fanout

### OrgAuthority (`base/organization/authority.rs`)
- `tokio::sync::Mutex` (NOT std::sync::Mutex) — holds lock across `.await` points in async mutation methods.
- Threaded as `Option<Arc<tokio::sync::Mutex<OrgAuthority>>>` from `service.rs` → `handle_cell_message` → `DispatchCtx` → `organization_dispatch` → handlers.
- Field in DispatchCtx: `pub org_authority: &'a Option<Arc<tokio::sync::Mutex<OrgAuthority>>>`
- service.rs initializes on startup: `OrgAuthority::load_all(pool)` or falls back to `OrgAuthority::empty()` on DB failure.

### DEFAULT_RANK_PERMISSIONS
- Canonical definition in `cimmeria_game::social::guilds::DEFAULT_RANK_PERMISSIONS: [u32; 9]`
- `Org::new()` uses this to seed all 9 rank_configs at creation time.
- `persistence::DEFAULT_PERMISSION_LADDER` is a `pub use` re-export of the game crate constant.
- Values: None=0, Initiate=0, Member=ROSTER_NOTES, SeniorMember=ROSTER_NOTES, Veteran=ROSTER_NOTES|OFFICER_CHAT, SeniorVeteran=ROSTER_NOTES|OFFICER_CHAT, Officer=INVITE|EJECT|ROSTER_NOTES|OFFICER_NOTES|OFFICER_CHAT|MOTD, SeniorOfficer=all-Officer+PROMOTE|DEMOTE|RANK_NAMES, Leader=ALL.

### Fan-out pattern
- `broadcast_to_org(entity_ids, method_index, args, transport, connected, entity_to_addr)` in `fanout.rs`.
- Fire-and-forget: one tokio::spawn per recipient.
- Must always be called AFTER DB write + in-memory update. Never before.

### New wire methods (Phase 3)
- CM 13: `organizationMOTD` — INT32 org_id + WSTRING motd.
- S→C CM 40: `onMemberRankChangedOrganization` — INT32 player_id + UINT8 rank + INT32 org_id + WSTRING name.
- S→C CM 45: `onOrganizationMOTDUpdate` — INT32 org_id + WSTRING motd.
- SGWPlayer 134: `onOrganizationCreationResult` — UINT8 result + UINT8 ret_code (2 bytes total).
- method_idx constants: `ON_ORGANIZATION_CREATION_RESULT = 134`, `LAUNCH_ORGANIZATION_CREATION = 135`.

### CellToBaseMsg variants (Phase 3)
- `OrgPersistentCreate { entity_id, player_id, player_name, org_type: u8, name }`
- `OrgPersistentSendInvite { actor_entity_id, actor_player_id, actor_name, org_id, target_player_name }`
- `OrgPersistentInviteAccepted { new_member_entity_id, new_member_player_id, new_member_name, actor_player_id, org_id }`
- `OrgPersistentKick { actor_entity_id, actor_player_id, org_id, target_player_name }`
- `OrgPersistentRankChange { actor_entity_id, actor_player_id, org_id, target_player_name, new_rank: u8 }`
- `OrgPersistentSetMotd { actor_entity_id, actor_player_id, org_id, motd }`

### Dispatch guard pattern
```rust
let (Some(auth), Some(pool)) = (ctx.org_authority.as_ref(), ctx.db_pool.as_ref()) else {
    tracing::warn!("...: no OrgAuthority or db_pool, skipping");
    return;
};
```

### Stale-test pitfall (22+ call sites)
When `handle_cell_message` signature gains a new arg, ALL test call sites need updating.
In this PR: added `org_authority: &Option<Arc<tokio::sync::Mutex<OrgAuthority>>>` as 10th arg.
Tests in `tests_dispatch_arms/` + `cell_dispatch/tests.rs` + `base/world_entry/methods/missions.rs` all got `&None,` appended.

## Open issues (Phase 4+)

- Wire `OrgAuthority::on_player_login` and `on_player_logout` from base connect/disconnect path (Phase 4).
- `REASON_KICKED: u8 = 2` — provisional, needs x64dbg confirmation.
- EReasons enum for onOrganizationLeft values not yet recovered — need x64dbg.
- RosterInfo `isOnline` bool presence unconfirmed — need x64dbg at 0x00d8ade0.
- CM 17 (organizationSetRankName) Rust stub drops trailing WSTRING — fix in Phase 4.
- `OrgPersistentSendInvite` dispatch is a no-op stub — needs player name→entity_id lookup.
- member_ref: player_id (int FK), NOT name string. Different from contact_list_member (uses name strings).
