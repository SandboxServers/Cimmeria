# Chat / Communication Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `Communicator.def`, `alias.xml`

---

**Interface**: `Communicator` (implemented by `SGWPlayer`)

### Client → Server (Exposed Base Methods)

#### `sendPlayerCommunication` — Send Chat Message

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `Channel` | `UINT8` | 1B | EChannel enum (say, tell, party, etc.) |
| `Target` | `WSTRING` | 4B len + N×2B | Target player (for tells) or empty |
| `Text` | `WSTRING` | 4B len + N×2B | Message text |

#### `chatJoin` — Join Chat Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelName` | `WSTRING` | 4B len + N×2B |
| `aChannelPassword` | `WSTRING` | 4B len + N×2B |

#### `chatLeave` — Leave Chat Channel

| Field | Type | Size |
|-------|------|------|
| `aChannelID` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `chatSetAFKMessage` / `chatSetDNDMessage`

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `message` | `WSTRING` | 4B len + N×2B |

#### `chatIgnore` — Ignore/Unignore Player

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B — 1=ignore, 0=unignore |

#### `chatFriend` — Add/Remove Friend

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aPlayerNick` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatList` — Request Channel Member List

| Field | Type | Size |
|-------|------|------|
| `aChannelID` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `chatMute` — Mute/Unmute in Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatKick` — Kick from Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |

#### `chatOp` — Grant Operator Status

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |

#### `chatBan` — Ban/Unban from Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatPassword` — Set Channel Password

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aChannelPassword` | `WSTRING` | 4B len + N×2B |

#### `petition` / `announcePetition` — GM Petition

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aMessage` | `WSTRING` | 4B len + N×2B |

### Server → Client

#### `onSystemCommunication` — System Message

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `TextType` | `INT32` | 4B | Message type enum |
| `StringId` | `INT32` | 4B | Localized string ID |
| `Speaker` | `WSTRING` | 4B len + N×2B | NPC name (for NPC speech) |
| `tokenList` | `ARRAY<StringToken>` | 4B count + N×StringToken | Token substitutions |

**`StringToken` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `stringID` | `INT32` | 4B |
| `literal` | `WSTRING` | 4B len + N×2B |

#### `onPlayerCommunication` — Player Chat Message

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Speaker` | `WSTRING` | 4B len + N×2B |
| `SpeakerFlags` | `UINT8` | 1B — ESpeakerFlags |
| `Channel` | `UINT8` | 1B — EChannel |
| `Text` | `WSTRING` | 4B len + N×2B |

#### `onLocalizedCommunication` — Localized Player Chat

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Speaker` | `WSTRING` | 4B len + N×2B |
| `SpeakerFlags` | `UINT8` | 1B |
| `Channel` | `UINT8` | 1B |
| `Text` | `WSTRING` | 4B len + N×2B |
| `tokenList` | `ARRAY<StringToken>` | 4B count + N×StringToken |

#### `onTellSent` — Tell Confirmation

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aTarget` | `WSTRING` | 4B len + N×2B |
| `aText` | `WSTRING` | 4B len + N×2B |

#### `onChatJoined` — Channel Join Notification

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ChannelName` | `WSTRING` | 4B len + N×2B |
| `ChannelID` | `UINT8` | 1B |

#### `onChatLeft` — Channel Leave Notification

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ChannelName` | `WSTRING` | 4B len + N×2B |

#### `onNickChanged` — Nickname Update

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aPlayerNickname` | `WSTRING` | 4B len + N×2B |
| `aAddRemoveFlag` | `UINT8` | 1B |

---

## Implementation Notes

- **Chat routing**: Most chat goes through base methods (name-based lookup needed). Only `processPlayerCommunication` is a cell method for local area chat.
