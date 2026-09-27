---
title: "Group System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Group System

> **Last updated**: 2026-09-27
> **Status**: `GroupAuthority` is not implemented: entity definitions only. Squads are implemented without it, as cell state ([Squads](#squads-org-03)), and so are Teams and Commands, on the base ([organization-system.md](organization-system.md)). Everything below is server-side and tested, and **not yet client-verified**: the owner's two-client UAT ([organizations-uat.md](../guides/organizations-uat.md), steps 1-4 for squads) is next.

## Overview

The group system manages persistent and temporary player groupings through a `GroupAuthority` entity. This entity acts as a central coordinator for group creation, membership, and cross-entity method dispatch. Organizations (guilds, squads, teams) are built on top of this group infrastructure.

The `GroupAuthority` interface is defined in `entities/defs/interfaces/GroupAuthority.def`. The entity type `SGWPlayerGroupAuthority` extends `SGWEntity` and implements this interface.

## Implementation Status

Everything in this table is definition-only. There is no `SGWPlayerGroupAuthority` instance in the Rust server and no handler for any of the four `GroupAuthority` base methods. Cimmeria does not need one: the [organization system](organization-system.md) keeps squads in a cell registry and Teams and Commands in the database (D-ORG03, D-ORG04), so nothing is gated on this infrastructure.

| Feature | Status | Notes |
|---------|--------|-------|
| Group authority entity | DEFINED | `SGWPlayerGroupAuthority` entity type exists in the defs; never instantiated |
| Group join | NOT IMPL | `joinGroup` base method defined; no handler |
| Group leave | NOT IMPL | `leaveGroup`, `leaveGroupByName` defined; no handler |
| Method dispatch | NOT IMPL | `callMethodOnGroup` defined; no handler |
| Group ID allocation | DEFINED | `lastTempID` counter property; nothing increments it |
| Organization integration | NOT USED | Squads, Teams and Commands are implemented without `GroupAuthority` ([Squads](#squads-org-03), [organization-system.md](organization-system.md)) |

## Squads (ORG-03)

Cimmeria does not build squads on `GroupAuthority`. A squad is ephemeral, so it lives only in the cell's memory and is never persisted (D-ORG03 in the [organizations campaign](../analysis/organizations/README.md)). One `SquadRegistry` serves every space: it is the `squads` field of `SpaceManager`, the container every cell handler already receives. A member who gates to another world keeps their squad.

| Feature | Status | Where |
|---------|--------|-------|
| Registry | DONE | [`crates/cell-world/src/cell/squad/`](../../crates/cell-world/src/cell/squad/): squads, membership, pending invites, limits. Pure state, `now` injected |
| Invite (`/squadinvite`) | DONE | Base 0xD0 `organizationInviteByType` with type 0 → `OrgBaseToCell::SquadInvite` → [`squad/invite.rs`](../../crates/cell-methods/src/cell/cell_methods/organization/squad/invite.rs) |
| Accept / decline | DONE | CM 8 `organizationInviteResponse` with a cell request id |
| Leave | DONE | CM 9 `organizationLeave` with the caller's own squad id |
| Kick (leader) | DONE | Base 0xD1 `organizationKick` with a squad id → `OrgBaseToCell::SquadKick` |
| Loot mode (leader) | DONE | CM 18 `squadSetLootMode`, 0 or 1 only. The mode is stored and shown to every member; no loot path reads it yet |
| Disconnect | DONE | The cell's `DisconnectEntity` arm removes the member with `Logout`, including a member in gate transit whose cell entity is gone (found by the last entity id the registry recorded on join or world entry) |
| World entry | DONE | `InitPlayerState` re-sends the squad after a gate trip |
| Promote to leader | NOT IMPL | No client UI sends it; `/squadpromote` is assumed to use `organizationRankChange` (unconfirmed, ORG-E1 Q2) |
| Squad chat | DONE | `sendPlayerCommunication` on `CHAN_SQUAD` (4), past the base's chat flood limit and text rules → [`chat/squad.rs`](../../crates/cell-console/src/cell/console/chat/squad.rs) (ORG-04) |
| Minimap ping | DONE (validated, not relayed) | CM 10 `BroadcastMinimapPing` with a squad id → [`squad/ping.rs`](../../crates/cell-methods/src/cell/cell_methods/organization/squad/ping.rs). No client method shows another member's ping (ORG-E1 Q3), so nothing is sent (ORG-04) |
| GM console | DONE | `.squad_invite <name>`, `.squad_join <name>`, `.squad_info [name]` → [`console/squad.rs`](../../crates/cell-console/src/cell/console/squad.rs) (the GM check, name resolution, the listing and the audit row), which calls the thin [`squad/gm.rs`](../../crates/cell-methods/src/cell/cell_methods/organization/squad/gm.rs) for the invite and the join (ORG-04) |
| Ignore-list check on invite | DONE | The base checks the invitee's cached Ignore list (SS-C1) before it forwards `organizationInviteByType` type 0 to the cell, and refuses with a line and a `squad.invite` row (`reason = ignored`) (ORG-07) |

### Rules

- **Ids.** Squad ids count up from `SQUAD_ORG_ID_MIN` (`0x4000_0000`) and are never reused in a server run (D-ORG05). Invite request ids count up from 1 with bit 29 clear, so CM 8 can tell a squad invite from a Team or Command one (D-ORG06). Everything is keyed by the character's `player_id`, never the entity id, because entity ids are recycled and gate travel re-creates the entity.
- **Invite.** The cell resolves the name across every space. A name that matches nobody, a player in gate transit, a name that matches two entities, the inviter themselves and a player who has not finished entering the world are refused with feedback. The invitee must be in no squad; the inviter must lead a squad with room (six members at most) or have none. There is at most one pending invite per inviter and invitee, at most five pending per invitee, and at most five invites per inviter per 30 seconds.
- **Response.** An invite is found only under the invitee's own `player_id` **and** the request id, answers for 60 seconds, and is consumed by the first response, accept or decline. An accept is re-validated: the squad still exists, the inviter is still in it and still leads it, there is room, and the invitee is still squadless. The first accept of a squadless inviter creates the squad with the inviter as leader. A decline tells the inviter.
- **Leaving.** A leave, a kick and a disconnect all remove the member. If one member is left, the squad dissolves. Otherwise, if the leader left, the longest-standing member leads (D-ORG12).
- **Loot mode.** Only the leader may change it, and only to RoundRobin (0) or FreeForAll (1) (D-ORG16).
- **Chat.** A line on channel 4 goes to every member of the speaker's squad whose entity is live, in any space, the speaker included (the legacy channel sent every line to every member). A member in gate transit misses it. The base applies the chat flood limit (`RateCategory::Chat`) and the text rules (`org_text::validate(TextField::ChatText)`) before forwarding, as for every channel. A speaker in no squad reads "You are not in a squad." on the feedback channel.
- **Minimap ping.** CM 10 is accepted only for the caller's own squad id, and at most once a second per member (a refused ping does not restart the second). An accepted ping is logged and sends nothing: the pinging client draws its own ping, and no client method shows another member's. A ping from a player in no squad, or naming another squad, is refused with `onErrorCode` and a line. A ping over the limit is dropped without a line, since the client already drew it and a held ping key would flood the chat window. A Team or Command id keeps ORG-01's "not available yet" answer.
- **GM console.** `.squad_invite <name>` is `/squadinvite` with the GM as inviter. `.squad_join <name>` puts the GM into that player's squad with no invite or answer, founding one the named player leads if they have none; it skips the handshake and the leader check, never the membership rules (the GM must be squadless, the squad must have room). `.squad_info [name]` lists a squad: id, size, loot mode, and each member's rank, level and live entity (or "in transit"). All three are GM-only, like every `.` command. With `.squad_join` one tester can build a squad with a sentinel character.
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
| Squad chat | Every member with a live entity, the speaker included | `onPlayerCommunication` [28] on channel 4, with the speaker's name and flags |
| Minimap ping | Nobody | Nothing (ORG-E1 Q3) |
| `.squad_join` | As a join, for the GM (and the named player, when it founds the squad) | As a join above, then a feedback line to the GM |
| World entry (gate arrival) | The arriving member | [35] (`aNewMember` 0), [38], [37] per other member, [51] |

### Telemetry

Everything logs on the `squad` target. Each action (invite, invite response, leave, kick, loot mode) runs in an INFO span of the same name and ends in exactly one INFO outcome row: `event = squad.<action>`, `outcome = ok` or `rejected`, a `reason` on a refusal, the actor's `account_id` and `player_id`, and the second player's `target_account_id` / `target_player_id` for an invite, a response or a kick. State changes (`squad_created`, `member_joined`, `member_left`, `leader_changed`, `loot_mode_changed`, `disbanded`, `invite_created`, `invite_consumed`, `invite_expired`) are DEBUG rows, and the counter is `squad_actions_total{action, outcome, reason}`. The closed reason list is in the [observability target catalog](../architecture/observability.md).

ORG-04 adds `squad.chat` and `squad.ping` outcome rows. Both carry `recipients` (0 for a ping); `squad.chat` also carries `text_units` (never the text). The cell logs the chat rows it decides (`ok`, `not_in_squad`); the base logs the ones it refuses before the forward (`rate_limited`, only once per feedback notice, and `text_invalid`). Ping refusals are `not_in_squad`, `wrong_squad` and `rate_limited`. A chat line that cannot be queued is WARN `squad.send_failed`. The GM commands log one INFO `org.gm_action` row on the `org` target with the GM and the named player, and count on `squad_actions_total` as `gm_squad_invite`, `gm_squad_join` and `gm_squad_info`.

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
