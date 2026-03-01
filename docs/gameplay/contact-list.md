# Contact List System

> **Last updated**: 2026-03-01
> **Status**: 0% implemented

## Overview

The contact list system manages player-curated lists of other players -- friends, ignore lists, and custom-named lists with configurable flags. It provides online/offline notifications for listed contacts and supports game event notifications (level-up, death, gate travel). The system is generic: rather than a hardcoded "friends list," it supports arbitrary named lists identified by integer IDs, each with a flags bitmask.

The `ContactListManager` interface is defined in `entities/defs/interfaces/ContactListManager.def`. It is implemented by `SGWPlayer`. All list management methods (create, delete, rename, add/remove members) are exposed cell methods invoked by the client. Server-to-client notifications use client methods for list state sync and contact events.

Two internal base methods (`sendEventToPlayers` and `sendLoginStatusMessages`) handle server-side event broadcasting -- these are never called by the client.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| List creation | STUB | `contactListCreate` defined, Python stub only (`pass`) |
| List deletion | STUB | `contactListDelete` defined, Python stub only |
| List renaming | STUB | `contactListRename` defined, Python stub only |
| List flags update | STUB | `contactListFlagsUpdate` defined, Python stub only |
| Add members | STUB | `contactListAddMembers` defined, Python stub only |
| Remove members | STUB | `contactListRemoveMembers` defined, Python stub only |
| List sync to client | DEFINED | `onContactListUpdate` client method |
| Delete sync to client | DEFINED | `onContactListDelete` client method |
| Member sync to client | DEFINED | `onContactListAddMembers`, `onContactListRemoveMembers` |
| Contact events | DEFINED | `onContactListEvent` client method |
| Login status broadcast | DEFINED | `sendLoginStatusMessages` base method |
| Event broadcast | DEFINED | `sendEventToPlayers` base method |

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
Client                              CellApp (SGWPlayer)                    BaseApp
  |                                      |                                    |
  |-- contactListCreate(name, flags) --> |                                    |
  |                                      |-- stores in contactLists dict      |
  |                                      |                                    |
  |  <-- onContactListUpdate(id, ...) --|                                    |
  |                                      |                                    |
  |-- contactListAddMembers(id, [...])-->|                                    |
  |                                      |-- updates contactLists             |
  |  <-- onContactListAddMembers(...) --|                                    |
  |                                      |                                    |
  |                                      |                     (on player login)
  |                                      |                                    |
  |                                      |            sendLoginStatusMessages |
  |  <-- onContactListEvent(name, 1, 1) ----------------------------------- |
```

## List Flags

The `aFlags` parameter (UINT32) on `contactListCreate` and `contactListFlagsUpdate` is a bitmask controlling list behavior. The exact flag values are not defined in the `.def` files or enumerations -- they are likely defined in client-side code or a data table. Probable flags include:

- **Notify on login/logout** -- trigger `onContactListEvent` when listed players come online/offline
- **Notify on level** -- trigger event on level-up
- **Notify on death** -- trigger event on death
- **Notify on gate travel** -- trigger event on stargate passage

These likely correspond 1:1 with the `EContactListEvent` bitmask values.

## Relationship to Chat System

The chat system (`Communicator.def`) has its own friend and ignore mechanisms:

- `chatFriend` (base method) -- adds/removes a friend with a nickname, triggers `onNickChanged`
- `chatIgnore` (base method) -- adds/removes from the `ignoredList` property

These are **separate from the contact list system**. The chat friend/ignore list is stored in `Communicator` properties (`ignoredList`), while contact lists are stored in the `ContactListManager` property (`contactLists`). The chat friends system provides nickname support and is integrated with the chat channel manager; the contact list system provides event notifications and custom list categorization.

## Python Implementation

All six cell methods are defined as stubs in `python/cell/SGWPlayer.py` (lines 2285-2306):

```python
def contactListCreate(self, aName, aFlags):
    pass

def contactListDelete(self, aListId):
    pass

def contactListRename(self, aListId, aName):
    pass

def contactListFlagsUpdate(self, aListId, aFlags):
    pass

def contactListAddMembers(self, aListId, aPlayerNames):
    pass

def contactListRemoveMembers(self, aListId, aPlayerNames):
    pass
```

No base method implementations exist for `sendEventToPlayers` or `sendLoginStatusMessages`.

## Data References

- **Interface**: `ContactListManager` (implemented by `SGWPlayer`)
- **Enumerations**: `EContactListEvent` (login, level, death, gate travel)
- **Python enum name map**: `EContactListEvent = {}` in `python/Atrea/enumNames.py` (empty -- not populated)
- **Related entity**: `SGWPlayer.def` -- implements `ContactListManager` alongside 9 other interfaces

## RE Priorities

1. **List flags** -- Determine the exact flag bitmask values and their correspondence to event types
2. **contactLists data structure** -- Format of the PYTHON dictionary stored in the `contactLists` property
3. **List ID allocation** -- How list IDs are assigned (auto-increment, hash, etc.)
4. **Login status flow** -- How `sendLoginStatusMessages` is triggered on player login/logout
5. **Event routing** -- How game events (level-up, death, gate travel) are detected and routed to `sendEventToPlayers`
6. **Persistence** -- Whether contact lists survive server restarts (CELL_PRIVATE flag suggests they may need explicit DB persistence)

## Related Docs

- [chat-system.md](chat-system.md) -- Chat friends/ignore (separate system)
- [organization-system.md](organization-system.md) -- Organization membership (separate from contact lists)
- [group-system.md](group-system.md) -- Group membership (server-internal, no client visibility)
