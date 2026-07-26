---
title: "Group System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Group System

> **Last updated**: 2026-07-25
> **Status**: Not implemented. Entity definitions only — no Rust code creates, joins, or dispatches to a group.

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

```
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
- [chat-system.md](chat-system.md) - Group-based chat channels
