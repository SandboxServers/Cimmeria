---
title: "Contact List System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Contact List System

> **Last updated**: 2026-07-25
> **Status**: Implemented and confirmed working in-game

## Overview

The contact list system manages player-curated lists of other players -- friends, ignore lists, and custom-named lists with configurable flags. It provides online/offline notifications for listed contacts and supports game event notifications (level-up, death, gate travel). The system is generic: rather than a hardcoded "friends list," it supports arbitrary named lists identified by integer IDs, each with a flags bitmask.

The `ContactListManager` interface is defined in `entities/defs/interfaces/ContactListManager.def`. It is implemented by `SGWPlayer`. All list management methods (create, delete, rename, add/remove members) are exposed cell methods invoked by the client. Server-to-client notifications use client methods for list state sync and contact events.

Two internal base methods (`sendEventToPlayers` and `sendLoginStatusMessages`) handle server-side event broadcasting -- these are never called by the client.

The Rust implementation splits across the two services. The six inbound cell methods (indices 55–60) live in [`crates/services/src/cell/cell_methods/contact_list/mod.rs`](../../crates/services/src/cell/cell_methods/contact_list/mod.rs) — they parse the wire payload, resolve `player_id`, and forward to the base via `CellToBaseMsg`. The base side ([`crates/services/src/base/contact_list/`](../../crates/services/src/base/contact_list/)) owns all DB mutations, the client echo responses, and the presence fanout.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| List creation | DONE | `contactListCreate` (CM 55) → `handlers::header_ops::handle_create` |
| List deletion | DONE | `contactListDelete` (CM 56) → `handle_delete` |
| List renaming | DONE | `contactListRename` (CM 57) → `handle_rename` |
| List flags update | DONE | `contactListFlagsUpdate` (CM 58) → `handle_flags_update` |
| Add members | DONE | `contactListAddMembers` (CM 59) → `handlers::member_ops::handle_add_members` |
| Remove members | DONE | `contactListRemoveMembers` (CM 60) → `handle_remove_members` |
| List sync to client | DONE | `onContactListUpdate` (CM 85), built by `wire::build_on_contact_list_update` |
| Delete sync to client | DONE | `onContactListDelete` (CM 86) |
| Member sync to client | DONE | `onContactListAddMembers` (CM 87), `onContactListRemoveMembers` (CM 88) |
| Contact events | DONE | `onContactListEvent` (CM 89) — all four event types fire (see below) |
| Login status broadcast | DONE | `handlers::presence_fanout::fanout_login_status`, called from `dispatch/session.rs` on disconnect and `world_entry_appearance/client_ready.rs` on login |
| Event broadcast | DONE | `handlers::presence_fanout::fanout_contact_event` |
| Persistence | DONE | `sgw_contact_list` (headers) + `sgw_contact_list_member` (members by name) |
| Login push | DONE | `push_contact_lists_on_login` sends all lists + members after the world-entry burst bundle |
| System lists | DONE | `ensure_system_lists` guarantees every character has Friends / Ignore on first login |

### Event Wiring

All four `EContactListEvent` bits are fired by real game-state changes:

| Event | Fired from |
|-------|-----------|
| `LoggedInStatus` (1) | `base/dispatch/session.rs` (logout), `base/world_entry_appearance/client_ready.rs` (login) |
| `GainLevel` (2) | `base/world_entry/methods/progression/mod.rs` |
| `Death` (4) | `cell/abilities/death.rs` |
| `GateTravel` (8) | `base/world_entry/gate_travel/mod.rs` |

## Entity Definition (ContactListManager.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `contactLists` | PYTHON | CELL_PRIVATE | Dictionary of all contact lists (not synced to client) |

### Cell Methods (Client -> Server)

All cell methods are `<Exposed/>` -- callable by the owning client.

| Method | Args | Purpose |
|--------|------|---------|
| `contactListCreate` | aName (WSTRING), aFlags (UINT32) | Create a new contact list |
| `contactListDelete` | aListId (INT32) | Delete a contact list |
| `contactListRename` | aListId (INT32), aName (WSTRING) | Rename an existing list |
| `contactListFlagsUpdate` | aListId (INT32), aFlags (UINT32) | Update list behavior flags |
| `contactListAddMembers` | aListId (INT32), aPlayerNames (ARRAY\<WSTRING\>) | Add players to a list |
| `contactListRemoveMembers` | aListId (INT32), aPlayerNames (ARRAY\<WSTRING\>) | Remove players from a list |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onContactListUpdate` | aListId (INT32), aName (WSTRING), aFlags (UINT32) | List created or updated |
| `onContactListDelete` | aListId (INT32) | List deleted |
| `onContactListAddMembers` | aListId (INT32), aPlayerNames (ARRAY\<WSTRING\>) | Members added to list |
| `onContactListRemoveMembers` | aListId (INT32), aPlayerNames (ARRAY\<WSTRING\>) | Members removed from list |
| `onContactListEvent` | aPlayerName (WSTRING), aEventId (UINT32), aDataValue (INT32) | Contact event notification |

### Base Methods (Server Internal)

These are not exposed to the client -- they are called by other server entities.

| Method | Args | Purpose |
|--------|------|---------|
| `sendEventToPlayers` | aEventId (UINT32), aDataValue (INT32), aPlayerNames (ARRAY\<WSTRING\>) | Broadcast event to listed players |
| `sendLoginStatusMessages` | aPlayerNames (ARRAY\<WSTRING\>) | Send login/logout status to contacts |

## Contact List Events (EContactListEvent)

The `aEventId` in `onContactListEvent` is a bitmask from the `EContactListEvent` enumeration:

| Name | Value | Purpose |
|------|-------|---------|
| `ECONTACT_LIST_EVENT_LoggedInStatus` | 1 | Player logged in or out |
| `ECONTACT_LIST_EVENT_GainLevel` | 2 | Player gained a level |
| `ECONTACT_LIST_EVENT_Death` | 4 | Player died |
| `ECONTACT_LIST_EVENT_GateTravel` | 8 | Player traveled through a stargate |

The `aDataValue` field (INT32) provides context-specific additional data for each event type (e.g., new level number for `GainLevel`, online/offline status for `LoggedInStatus`).

## Architecture

```
Client                          CellService                        BaseService
  |                                  |                                   |
  |-- contactListCreate(name,flags)->|                                   |
  |                                  |-- CellToBaseMsg ----------------->|
  |                                  |                    INSERT sgw_contact_list
  |  <----------------------------------- onContactListUpdate(id, ...) --|
  |                                  |                                   |
  |-- contactListAddMembers(id,[..])>|                                   |
  |                                  |-- CellToBaseMsg ----------------->|
  |                                  |             INSERT sgw_contact_list_member
  |  <------------------------------- onContactListAddMembers(id, [..]) -|
  |                                  |                                   |
  |                                  |                    (on player login/logout)
  |                                  |                        fanout_login_status
  |  <------------------------------------ onContactListEvent(name, 1, 1) |
```

## List Flags

The `aFlags` parameter (UINT32) on `contactListCreate` and `contactListFlagsUpdate` is **not** a notification bitmask mirroring `EContactListEvent`, as an earlier revision of this doc speculated. It carries the list's `EMoniker` text moniker -- the id the client uses to look up the list's display label.

The two system lists every character gets on first login use fixed monikers:

| List | `flags` value |
|------|--------------|
| Friends | 300 |
| Ignore | 301 |

See `ensure_system_lists` in [`base/contact_list/persistence/mod.rs`](../../crates/services/src/base/contact_list/persistence/mod.rs). Player-created lists carry whatever moniker the client sends. The server stores and echoes the value without interpreting it; contact-event delivery is driven by list *membership*, not by flags.

## Relationship to Chat System

The chat system (`Communicator.def`) has its own friend and ignore mechanisms:

- `chatFriend` (base method) -- adds/removes a friend with a nickname, triggers `onNickChanged`
- `chatIgnore` (base method) -- adds/removes from the `ignoredList` property

These are **separate from the contact list system**. The chat friend/ignore list is stored in `Communicator` properties (`ignoredList`), while contact lists are stored in the `ContactListManager` property (`contactLists`). The chat friends system provides nickname support and is integrated with the chat channel manager; the contact list system provides event notifications and custom list categorization.

## Persistence

The original design stored contact lists in the `contactLists` CELL_PRIVATE PYTHON property. The Rust server persists them relationally instead, so lists survive restarts without any explicit save step:

| Table | Columns | Purpose |
|-------|---------|---------|
| `sgw_contact_list` | `list_id`, `player_id`, `name`, `flags` | List headers. `list_id` is server-assigned from `sgw_contact_list_list_id_seq`. Unique on `(player_id, name)`. |
| `sgw_contact_list_member` | `list_id`, `player_name` | Members, stored by name string rather than character id. |

`ADD`/`REMOVE` requests are capped at `MAX_MEMBERS_PER_REQUEST` (100) names, enforced on both the cell-side parser and the base-side handler.

## Data References

- **Interface**: `ContactListManager` (implemented by `SGWPlayer`)
- **Enumerations**: `EContactListEvent` (login, level, death, gate travel); `EMoniker` (list display labels — 300 Friends, 301 Ignore)
- **Related entity**: `SGWPlayer.def` -- implements `ContactListManager` alongside 9 other interfaces
- **Client RE**: [contact-list-restoration.md](../reverse-engineering/findings/contact-list-restoration.md) and [contact-list-wire-formats.md](../reverse-engineering/findings/contact-list-wire-formats.md) — the SGW contact-list UI is CEGUI + Lua (`Social.lua`), not compiled UnrealScript

## Remaining Work

1. **GateTravel `dataValue` id-space** -- the server sends the destination `world_id` from `resources.worlds`, which the client passes to `getWorldInfo(value).Name`. The exact id-space has not been confirmed by send-and-observe in playtest.
2. **Ignore-list enforcement** -- membership in the `Ignore` system list is stored and synced, but nothing yet consults it to suppress tells or chat.

## Related Docs

- [chat-system.md](chat-system.md) -- Chat friends/ignore (separate system)
- [organization-system.md](organization-system.md) -- Organization membership (separate from contact lists)
- [group-system.md](group-system.md) -- Group membership (server-internal, no client visibility)
