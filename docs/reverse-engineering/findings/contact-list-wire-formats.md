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

**Handler address**: `0x00e5f9b0` (per RTTI; see *Cyclic-shift name-misassignment* note below).
**Wire format**: not yet decompiled — fields below derived from `.def`.

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

---

## Cyclic-shift name-misassignment (V5 Documentation Campaign session 1, 2026-05-12)

The V5 Documentation Campaign session 1 surfaced a pre-existing labelling bug: four adjacent functions at `0x00e5f990`–`0x00e5f9f0` had their pre-session-1 labels cyclically shifted by one slot relative to their RTTI-canonical names. The prior annotation script appears to have picked up the wrong string xref for adjacent functions in this contiguous TypedEmitInfo block.

| Address | Pre-session-1 label | Post-session-1 RTTI-canonical name |
|---------|---------------------|------------------------------------|
| `0x00e5f990` | `ContactListAddMembers` | `contactListRename` |
| `0x00e5f9b0` | `ContactListRemoveMembers` | `contactListFlagsUpdate` |
| `0x00e5f9d0` | `ContactListRename` | `contactListAddMembers` |
| `0x00e5f9f0` | `ContactListSetFlag` | `contactListRemoveMembers` |

The canonical names use camelCase (`contactList*`) per the binary's RTTI, while other event categories elsewhere in this codebase use PascalCase. **Do not normalize** — RTTI is authoritative for these names. Workers who notice the casing inconsistency in adjacent docs should leave the canonical names alone.

**Pattern of concern:** similar cyclic misassignments may exist in other adjacent-function clusters that share a TypedEmitInfo block. A session-2 sweep comparing RTTI names to annotation-script-assigned labels in every contiguous TypedEmitInfo block is recommended (see `docs/reverse-engineering/v5-campaign/CAMPAIGN_STATUS.md`).

### Open follow-ups

- **Doc-hygiene sweep (separate from session 2 RE work):** Other docs may reference these four contact-list functions by their pre-session-1 labels or with PascalCase normalization. A follow-up audit should grep the repo for `ContactListAddMembers`, `ContactListRemoveMembers`, `ContactListRename`, `ContactListSetFlag` and reconcile against the table above. **Do not edit those references as part of the V5 campaign consolidation** — that is dedicated doc-hygiene work.
- **Wire format for `contactListFlagsUpdate`:** field list above is `.def`-derived; not yet confirmed by decompiling the handler at `0x00e5f9b0`.
