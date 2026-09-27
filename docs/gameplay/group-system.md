---
title: "Group System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Group System

> **Last updated**: 2026-09-27
> **Status**: `GroupAuthority` is not implemented: entity definitions only. Squads are implemented without it, as cell state ([Squads](#squads-org-03)).

## Overview

The group system manages persistent and temporary player groupings through a `GroupAuthority` entity. This entity acts as a central coordinator for group creation, membership, and cross-entity method dispatch. Organizations (guilds, squads, teams) are built on top of this group infrastructure.

The `GroupAuthority` interface is defined in `entities/defs/interfaces/GroupAuthority.def`. The entity type `SGWPlayerGroupAuthority` extends `SGWEntity` and implements this interface.

## Implementation Status

Everything below is definition-only. There is no `SGWPlayerGroupAuthority` instance in the Rust server, no group registry, and no handler for any of the four base methods. The blocked [organization system](organization-system.md) sits on top of this, so both are gated on the same missing infrastructure.

| Feature | Status | Notes |
|---------|--------|-------|
| Group authority entity | DEFINED | `SGWPlayerGroupAuthority` entity type exists in the defs; never instantiated |
| Group join | NOT IMPL | `joinGroup` base method defined; no handler |
| Group leave | NOT IMPL | `leaveGroup`, `leaveGroupByName` defined; no handler |
| Method dispatch | NOT IMPL | `callMethodOnGroup` defined; no handler |
| Group ID allocation | DEFINED | `lastTempID` counter property; nothing increments it |
| Organization integration | NOT IMPL | Organization types would use groups as backing store |

## Squads (ORG-03)

Cimmeria does not build squads on `GroupAuthority`. A squad is ephemeral, so it lives only in the cell's memory and is never persisted (D-ORG03 in the [organizations campaign](../analysis/organizations/README.md)). One `SquadRegistry` serves every space: it is the `squads` field of `SpaceManager`, the container every cell handler already receives. A member who gates to another world keeps their squad.

| Feature | Status | Where |
|---------|--------|-------|
| Registry | DONE | [`crates/cell-world/src/cell/squad/`](../../crates/cell-world/src/cell/squad/): squads, membership, pending invites, limits. Pure state, `now` injected |
| Invite (`/squadinvite`) | DONE | Base 0xD0 `organizationInviteByType` with type 0 → `OrgBaseToCell::SquadInvite` → [`squad/invite.rs`](../../crates/cell-methods/src/cell/cell_methods/organization/squad/invite.rs) |
| Accept / decline | DONE | CM 8 `organizationInviteResponse` with a cell request id |
| Leave | DONE | CM 9 `organizationLeave` with the caller's own squad id |
| Kick (leader) | DONE | Base 0xD1 `organizationKick` with a squad id → `OrgBaseToCell::SquadKick` |
| Loot mode (leader) | DONE | CM 18 `squadSetLootMode`, 0 or 1 only |
| Disconnect | DONE | The cell's `DisconnectEntity` arm removes the member with `Logout` |
| World entry | DONE | `InitPlayerState` re-sends the squad after a gate trip |
| Promote to leader | NOT IMPL | No client UI sends it; `/squadpromote` is assumed to use `organizationRankChange` (unconfirmed, ORG-E1 Q2) |
| Squad chat, minimap ping | NOT IMPL | ORG-04 |
| Ignore-list check on invite | NOT IMPL | Ignore lists are base-side database rows; the cell has no copy |

### Rules

- **Ids.** Squad ids count up from `SQUAD_ORG_ID_MIN` (`0x4000_0000`) and are never reused in a server run (D-ORG05). Invite request ids count up from 1 with bit 29 clear, so CM 8 can tell a squad invite from a Team or Command one (D-ORG06). Everything is keyed by the character's `player_id`, never the entity id, because entity ids are recycled and gate travel re-creates the entity.
- **Invite.** The cell resolves the name across every space. A name that matches nobody, a player in gate transit, a name that matches two entities, the inviter themselves and a player who has not finished entering the world are refused with feedback. The invitee must be in no squad; the inviter must lead a squad with room (six members at most) or have none. There is at most one pending invite per inviter and invitee, at most five pending per invitee, and at most five invites per inviter per 30 seconds.
- **Response.** An invite is found only under the invitee's own `player_id` **and** the request id, answers for 60 seconds, and is consumed by the first response, accept or decline. An accept is re-validated: the squad still exists, the inviter is still in it and still leads it, there is room, and the invitee is still squadless. The first accept of a squadless inviter creates the squad with the inviter as leader. A decline tells the inviter.
- **Leaving.** A leave, a kick and a disconnect all remove the member. If one member is left, the squad dissolves. Otherwise, if the leader left, the longest-standing member leads (D-ORG12).
- **Loot mode.** Only the leader may change it, and only to RoundRobin (0) or FreeForAll (1) (D-ORG16).
- **Feedback.** Every refusal sends `onErrorCode` and a line on the feedback channel; a refused loot change also re-sends the current mode so the menu snaps back. The client has no text for an organization error code (ORG-E1 Q4), so the line is what the player reads.

### What each client is sent

| Event | To whom | Client methods, in order |
|-------|---------|--------------------------|
| Invite | Invitee | `onOrganizationInvite` [34] (type 0, empty org name) |
| Join (and both founders on creation) | Newcomer | `onOrganizationJoined` [35] (`aNewMember` 1), `onOrganizationRosterInfo` [38], one `onMemberJoinedOrganization` [37] per other member with their live entity id, `onSquadLootType` [51] |
| Join | Every other member | One [37] for the newcomer (`aNewMember` 1) |
| Leave, kick | The member who left | `onOrganizationLeft` [36] (`Requested` 0 or `Kicked` 1) |
| Leave, kick, disconnect | The rest | `onMemberLeftOrganization` [39] with the reason (`Logout` 3 on a disconnect) |
| Leader left | The rest | `onMemberRankChangedOrganization` [40] naming the new leader with rank 8 |
| Squad of one | The last member | [39] for the member who left, then [36] `Disbanded` 2 |
| Loot mode | Every member | [51] |
| World entry (gate arrival) | The arriving member | [35] (`aNewMember` 0), [38], [37] per other member, [51] |

The roster comes before the [37]s because the client stores every roster row with member id 0 ("Offline"), and only [37] sets the id (ORG-E1 Q1). The squad unit frames follow entity presence, so a member in another space shows a blank frame (ORG-E1 Q6). A member removed while in gate transit cannot be told then; their [36] is queued and sent on their world entry.

## Entity Definition (GroupAuthority.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `authGroups` | PYTHON | BASE | Dictionary of all managed groups |
| `authorityID` | INT8 | BASE | Unique ID of this authority instance |
| `lastTempID` | INT32 | BASE | Next group ID to allocate |

### Base Methods

| Method | Args | Purpose |
|--------|------|---------|
| `joinGroup` | GroupType (WSTRING), GroupID (PYTHON), CallerBase (MAILBOX), Name (PYTHON), DbID (PYTHON) | Add entity to a group, creating it if needed |
| `leaveGroup` | GroupID (PYTHON), LeavingBase (MAILBOX), InvokerDbID (PYTHON), InvokerBase (MAILBOX), Reason (INT8) | Remove entity from group |
| `leaveGroupByName` | GroupID (PYTHON), LeavingName (WSTRING), InvokerDbID (PYTHON), InvokerBase (MAILBOX), Reason (INT8) | Remove entity by name |
| `callMethodOnGroup` | GroupID (INT32), MethodName (STRING), Args (PYTHON) | Invoke a method on all group members |

## Architecture

```text
SGWPlayerGroupAuthority (BaseApp entity)
  |
  |-- authGroups: { groupId: GroupData }
  |     |-- GroupData contains member list, type, metadata
  |
  |-- Receives joinGroup/leaveGroup from player Base entities
  |-- Dispatches callMethodOnGroup to all members
  |
  |-- Organization system builds on top:
       |-- Command (guild) = persistent group
       |-- Squad (party) = temporary group
       |-- Team (strike team) = PvP group
```

## Group Types

| Type | Persistence | Purpose |
|------|-------------|---------|
| Command | Persistent | Guild / clan |
| Squad | Session | Party / dungeon group |
| Team | Session | Strike team (PvP) |

## Leave Reasons

The `Reason` parameter (INT8) in `leaveGroup`/`leaveGroupByName` corresponds to `EReasons` enumeration values used in `onOrganizationLeft` client notifications.

## Data References

- **Entity type**: `SGWPlayerGroupAuthority` (extends SGWEntity, implements GroupAuthority)
- **Enumerations**: `EReasons` (leave reasons), group type strings
- **Related entity**: `OrganizationMember.def` -- player-side organization interface

## RE Priorities

1. **Group data structure** - Format of `authGroups` dictionary entries
2. **Group lifecycle** - How groups are created, persisted, and destroyed
3. **Cross-entity dispatch** - `callMethodOnGroup` serialization and delivery
4. **Organization mapping** - How organization types map to group authority groups
5. **Authority distribution** - How multiple `GroupAuthority` entities partition groups

## Related Docs

- [organization-system.md](organization-system.md) - Organizations built on groups
- [Organizations campaign](../analysis/organizations/README.md) - Decisions D-ORG03 to D-ORG16 behind the squad rules
- [chat-system.md](chat-system.md) - Group-based chat channels
