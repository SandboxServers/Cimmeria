# Contact List Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `ContactListManager.def`, `alias.xml`

---

**Interface**: `ContactListManager` (implemented by `SGWPlayer`)

### Client → Server (Exposed Cell Methods)

#### `contactListCreate` — Create List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aName` | `WSTRING` | 4B len + N×2B |
| `aFlags` | `UINT32` | 4B |

#### `contactListDelete` — Delete List

| Field | Type | Size |
|-------|------|------|
| `aListId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `contactListRename` — Rename List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aName` | `WSTRING` | 4B len + N×2B |

#### `contactListFlagsUpdate` — Update List Flags

| Field | Type | Size |
|-------|------|------|
| `aListId` | `INT32` | 4B |
| `aFlags` | `UINT32` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `contactListAddMembers` — Add Players to List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `contactListRemoveMembers` — Remove Players from List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

### Server → Client

#### `onContactListUpdate` — List Created/Updated

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aName` | `WSTRING` | 4B len + N×2B |
| `aFlags` | `UINT32` | 4B |

#### `onContactListDelete` — List Deleted

| Field | Type | Size |
|-------|------|------|
| `aListId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onContactListAddMembers` — Members Added

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `onContactListRemoveMembers` — Members Removed

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `onContactListEvent` — Online/Offline Notification

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aPlayerName` | `WSTRING` | variable | Player who triggered event |
| `aEventId` | `UINT32` | 4B | Event type (login, logout, etc.) |
| `aDataValue` | `INT32` | 4B | Additional data |

---

## Implementation Notes

- **Contact lists**: Generic list system (not just friends) — supports custom lists with flags.
