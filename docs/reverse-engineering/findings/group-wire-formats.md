# Group System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `GroupAuthority.def`, `DistributionGroupMember.def`, `SGWPlayerGroupAuthority.def`, `SGWMob.def`, `alias.xml`

---

## Overview

The group system is a server-internal coordination layer. Unlike most systems documented here, the `GroupAuthority` and `DistributionGroupMember` interfaces have **no client-exposed methods and no client methods** -- all communication is between BaseApp and CellApp entities. The group system is not directly visible on the client wire; instead, higher-level systems (organizations, loot distribution, mob AI) use groups as a backing store and expose their own client-facing wire formats.

The `SGWPlayerGroupAuthority` entity (extends `SGWEntity`, implements `GroupAuthority`) acts as a central coordinator. Individual entities implement `DistributionGroupMember` to participate in groups. The `SGWSpawnSet` entity also implements `GroupAuthority` for mob group coordination.

---

## Server-Internal Messages (BaseApp)

**Interface**: `GroupAuthority` (implemented by `SGWPlayerGroupAuthority`, `SGWSpawnSet`)

These are base methods invoked by other server entities on the `GroupAuthority`. They are **never sent by the client**.

### `joinGroup` — Add Entity to Group

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `GroupType` | `WSTRING` | 4B len + N×2B | Type of group to create (e.g., "Squad", "Command") |
| `GroupID` | `PYTHON` | pickled | Group ID -- can be None to create new |
| `CallerBase` | `MAILBOX` | internal | Caller's base mailbox |
| `Name` | `PYTHON` | pickled | Entity name |
| `DbID` | `PYTHON` | pickled | Entity database ID |

### `leaveGroup` — Remove Entity from Group

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `GroupID` | `PYTHON` | pickled | Group ID (must be valid) |
| `LeavingBase` | `MAILBOX` | internal | Leaving entity's base mailbox |
| `InvokerDbID` | `PYTHON` | pickled | Invoker's database ID |
| `InvokerBase` | `MAILBOX` | internal | Invoker's base mailbox |
| `Reason` | `INT8` | 1B | EReasons enum |

### `leaveGroupByName` — Remove Entity by Name

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `GroupID` | `PYTHON` | pickled | Group ID (must be valid) |
| `LeavingName` | `WSTRING` | 4B len + N×2B | Leaving entity's name |
| `InvokerDbID` | `PYTHON` | pickled | Invoker's database ID |
| `InvokerBase` | `MAILBOX` | internal | Invoker's base mailbox |
| `Reason` | `INT8` | 1B | EReasons enum |

### `callMethodOnGroup` — Cross-Group RPC Dispatch

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `GroupID` | `INT32` | 4B | ID of target group |
| `MethodName` | `STRING` | 4B len + N×1B | Name of method to invoke on all members |
| `Args` | `PYTHON` | pickled | List of arguments for the method |

---

## Server-Internal Messages (CellApp)

**Interface**: `DistributionGroupMember` (implemented by `SGWPlayer` via `SGWEntity`)

These are cell methods invoked by the `GroupAuthority` on member entities. They are **never sent by the client**.

### `remoteJoinGroup` — Initiate Remote Group Join

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aType` | `PYTHON` | pickled | Group type |
| `aGroupID` | `PYTHON` | pickled | Group ID |
| `aAuthority` | `MAILBOX` | internal | GroupAuthority mailbox |
| `aMemberName` | `STRING` | 4B len + N×1B | Name of method to call on join |

### `onGroupJoined` — Group Join Confirmed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aType` | `PYTHON` | pickled | Group type |
| `aGroupID` | `PYTHON` | pickled | Group ID |
| `aAuthority` | `MAILBOX` | internal | GroupAuthority mailbox |
| `aState` | `PYTHON` | pickled | Group state data |
| `aJoinData` | `PYTHON` | pickled | Join-specific data |

### `onGroupJoinFail` — Group Join Failed

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aType` | `PYTHON` | pickled | Group type |
| `aGroupID` | `PYTHON` | pickled | Group ID |
| `aReason` | `INT32` | 4B | Failure reason code |

### `onGroupMemberJoined` — Member Added Notification

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aGroupID` | `PYTHON` | pickled | Group ID |
| `aEntity` | `MAILBOX` | internal | New member's mailbox |
| `aJoinData` | `PYTHON` | pickled | Join-specific data |

### `onGroupMemberLeft` — Member Removed Notification

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `aGroupID` | `PYTHON` | pickled | Group ID |
| `aEntity` | `MAILBOX` | internal | Departing member's mailbox |
| `aEntityName` | `WSTRING` | 4B len + N×2B | Departing member's name |
| `aReason` | `INT32` | 4B | EReasons enum (as INT32) |

### `callMethodOnGroup` — Receive Cross-Group RPC

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `GroupID` | `INT32` | 4B | ID of target group |
| `MethodName` | `STRING` | 4B len + N×1B | Method name to invoke |
| `Args` | `PYTHON` | pickled | Method arguments |

---

## Mob Group Messages (CellApp)

**Entity**: `SGWMob` (NPC/enemy entities)

These cell methods support mob group AI coordination -- when one mob in a group enters combat or generates threat, it notifies other group members.

### `onGroupMateEnteredCombat` — Group Combat Alert

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `GroupMate` | `MAILBOX` | internal | Mob that entered combat |
| `Target` | `MAILBOX` | internal | Entity the mob is fighting |

### `onGroupMateThreatTransfer` — Group Threat Sharing

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `GroupMate` | `MAILBOX` | internal | Mob that gained threat |
| `ThreatSource` | `MAILBOX` | internal | Entity that caused threat |
| `ThreatValue` | `INT32` | 4B | Threat amount |

### `mobJoinGroup` — Mob Joins Its Spawn Group

No arguments. Called to associate a mob with its `mobGroup` property.

---

## Properties

### GroupAuthority Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `authGroups` | `PYTHON` | `BASE` | Dictionary of all managed groups: `{ groupId: GroupData }` |
| `authorityID` | `INT8` | `BASE` | Unique ID of this authority instance (default: -1) |
| `lastTempID` | `INT32` | `BASE` | Next group ID to allocate |

### DistributionGroupMember Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `groups` | `PYTHON` | `CELL_PRIVATE` | Groups this member belongs to: `{ groupId: ... }` |
| `groupInfoUpdateTimers` | `PYTHON` | `CELL_PRIVATE` | Timer controllers for group info updates |
| `pendingJoin` | `PYTHON` | `CELL_PRIVATE` | Pending join operations: `{ ... }` |

### SGWMob Group Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `mobGroup` | `PYTHON` | `CELL_PRIVATE` | Mob's spawn group membership data |

**Note**: All group properties are `BASE` or `CELL_PRIVATE`. None have `ALL_CLIENTS` or `OWN_CLIENT` flags -- no group data is sent to the client via property sync. All client-visible group state is communicated through higher-level systems (organizations, chat channels).

---

## EReasons Enumeration

Used in `leaveGroup`, `leaveGroupByName`, and `onGroupMemberLeft`:

| Name | Value | Meaning |
|------|-------|---------|
| `REAS_requested` | 0 | Player voluntarily left |
| `REAS_kicked` | 1 | Kicked by leader/officer |
| `REAS_disbanded` | 2 | Group was disbanded |
| `REAS_logout` | 3 | Player logged out |

---

## Implementation Notes

1. **No client wire exposure**: The group system is entirely server-internal. The client never sends group-related messages directly -- it interacts with organizations (squads, guilds, teams) which internally use the group system as a backing store.
2. **Heavy PYTHON type usage**: Many arguments use `PYTHON` (pickled Python objects) rather than strongly-typed primitives. This makes the wire format flexible but opaque -- the actual data structures are defined in Python code, not in `.def` files.
3. **MAILBOX arguments**: `MAILBOX` types are internal BigWorld entity references. They are not serialized on the client wire -- they exist only for inter-entity communication within the server cluster.
4. **Dual callMethodOnGroup**: Both `GroupAuthority` (base method) and `DistributionGroupMember` (cell method) define `callMethodOnGroup` with identical signatures. The authority receives the call, then forwards it to each member's cell entity.
5. **Mob groups**: `SGWMob` entities have separate group coordination methods (`onGroupMateEnteredCombat`, `onGroupMateThreatTransfer`) for AI aggro linking. These are distinct from the player group system but share the concept.
6. **Stub implementation**: Both `python/cell/SGWPlayerGroupAuthority.py` and `python/base/SGWPlayerGroupAuthority.py` contain only empty `__init__` methods -- the group authority has no Python logic implemented.
7. **SGWSpawnSet**: Also implements `GroupAuthority`, used for NPC spawn set coordination rather than player groups.

---

## Related Documents

- [organization-wire-formats.md](organization-wire-formats.md) -- Organizations (squads, guilds, teams) built on top of groups
- [combat-wire-formats.md](combat-wire-formats.md) -- Universal RPC dispatcher architecture
- [contact-list-wire-formats.md](contact-list-wire-formats.md) -- Contact lists (separate from group membership)
- [../../../docs/gameplay/group-system.md](../../gameplay/group-system.md) -- Group system gameplay documentation
- [../../../docs/gameplay/organization-system.md](../../gameplay/organization-system.md) -- Organization gameplay documentation
