---
name: reference-squad-phase2
description: Squad (ephemeral org) Phase 2 implementation: state machine, wire formats, file layout, method indices, open questions for Phase 3
metadata:
  type: reference
---

## Scope

Phase 2 shipped in worktree `Cimmeria-org568`, branch `feat/org-system-568`.
Implements `OrgType::Squad` (ephemeral, no DB) end-to-end: state machine,
cell-side dispatch, base-side fanout, CHAN_SQUAD chat routing.

## File layout

```
crates/services/src/
  cell/
    social/
      mod.rs                         — pub mod squad_manager
      squad_manager.rs               — SquadManager + Squad + all SM tests
    cell_methods/organization.rs     — CM 8/9/10/18 implemented; 11–19 stub
    chat.rs                          — CHAN_SQUAD arm added, 2 new tests
    messages/cell_to_base.rs         — 5 new OrgSquad* variants
    space_manager/mod.rs             — `pub squads: SquadManager` field added
  base/
    organization/
      mod.rs                         — pub mod handlers, wire
      wire.rs                        — S→C builders 34/35/36/37/38/39/51 + tests
      handlers/mod.rs                — fanout handlers (accepted/left/loot/ping)
    world_entry/cell_dispatch/
      mod.rs                         — OrgSquad* variants added to router
      organization_dispatch.rs       — thin router to base::organization::handlers
  mercury/mod.rs                     — method_idx constants 34–51 added
```

## Method indices (confirmed from .def)

C→S cell methods: 8 INVITE_RESPONSE, 9 LEAVE, 10 BROADCAST_MINIMAP_PING,
11 STRIKE_TEAM_RESPONSE, 12 PVP_LEAVE_RESPONSE, 13 MOTD, 14 NOTE,
15 OFFICER_NOTE, 16 SET_RANK_PERMISSIONS, 17 SET_RANK_NAME,
18 SQUAD_SET_LOOT_MODE, 19 TRANSFER_CASH.

S→C client methods (OrganizationMember interface, indices 34–51):
34 ON_ORGANIZATION_INVITE, 35 ON_ORGANIZATION_JOINED, 36 ON_ORGANIZATION_LEFT,
37 ON_MEMBER_JOINED_ORGANIZATION, 38 ON_ORGANIZATION_ROSTER_INFO,
39 ON_MEMBER_LEFT_ORGANIZATION, 40 ON_MEMBER_RANK_CHANGED_ORGANIZATION,
41 ON_STRIKE_TEAM_UPDATE, 42 ON_PVP_ORGANIZATION_LEAVE_REQUEST,
43 ON_ORGANIZATION_NAME_UPDATE, 44 ON_ORGANIZATION_EXPERIENCE_UPDATE,
45 ON_ORGANIZATION_MOTD_UPDATE, 46 ON_ORGANIZATION_NOTE_UPDATE,
47 ON_ORGANIZATION_OFFICER_NOTE_UPDATE, 48 ON_ORGANIZATION_CASH_UPDATE,
49 ON_ORGANIZATION_RANK_UPDATE, 50 ON_ORGANIZATION_RANK_NAME_UPDATE,
51 ON_SQUAD_LOOT_TYPE.

CHAN_SQUAD = 4 (confirmed from python/Atrea/enums.py EChannel).

## State machine summary

Invite: `SquadManager::record_invite` → `OrgSquadSendInvite` (CM 8) →
`accept_invite` → `OrgSquadAccepted` → fanout [37 existing] + [35 + 38 new member].

Leave: `organizationLeave` (CM 9) → `remove_member` → `OrgSquadMemberLeft`
→ fanout [36 leaver] + [39 remaining]. Auto-promotion fires when leader leaves.

Loot: `squadSetLootMode` (CM 18) → `set_loot_mode` → `OrgSquadLootMode`
→ fanout [51 all members].

Minimap ping: `BroadcastMinimapPing` (CM 10) → `OrgSquadMinimapPing`
→ deferred (receivedMinimapPing method index unconfirmed — x64dbg needed).

## squad CELL_PUBLIC property

`entities/defs/interfaces/OrganizationMember.def` declares `squad: INT32` as
CELL_PUBLIC. Callers of `accept_invite` / `remove_member` must set
`entity.properties["squad"] = PropertyValue::Int(org_id)` (or 0 on leave).
This was NOT done in Phase 2 — it requires `SpaceManager::get_entity_mut` in
the cell-method handler after the squad state transition. Phase 3 task.

## MAX_SQUAD_SIZE = 6

Confirmed from binary RTTI: TargetSquadMember1..6 at 0x0184094c–0x018409ec.

## Open questions for Phase 3+

1. **EReasons enum values** for onOrganizationLeft / onMemberLeftOrganization:
   UNCONFIRMED. Using DISBANDED=0, LEFT=1, KICKED=2. x64dbg BP needed.
2. **receivedMinimapPing method index**: UNCONFIRMED. Fanout stub in place.
3. **RosterInfo isOnline field**: .def doesn't show it; x64dbg BP at 0x00d8ade0
   needed to confirm layout. Currently omitted.
4. **squad CELL_PUBLIC property sync**: not yet written to `entity.properties`.
5. **organizationInviteByType base method**: Phase 3 (creates OrgSquadSendInvite).
   `build_on_organization_invite` [34] is stubbed with `#[allow(dead_code)]`.

## Pre-existing test failures in worktree

`cell::interactions::trainer::tests::outcome_split_player_missing_vs_no_archetype`
and `cell::service::tests::npc_ai::state_machine::stationary_no_los_or_range`
fail in Phase 1 commit `e8b30c74` — not introduced by Phase 2.

See [[reference-organization-system]] for Phase 1 context.
