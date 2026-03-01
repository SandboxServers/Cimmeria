# Minigame System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `MinigamePlayer.def`, `alias.xml`

---

**Interface**: `MinigamePlayer` (implemented by `SGWPlayer`)

Minigames are web-based (Flash/browser) games embedded in the client. The server manages matchmaking, call requests, and result processing.

### Client → Server (Exposed Cell Methods)

#### `startMinigame` — Begin a Minigame

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `endCurrentMinigame` — End Current Game

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `debugStartMinigame` — Debug: Start Specific Game

| Field | Type | Size |
|-------|------|------|
| `aGameId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `requestSpectateList` — Request Spectatable Players

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `spectateMinigame` — Spectate a Player

| Field | Type | Size |
|-------|------|------|
| `playerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `registerToMinigameHelp` — Register as Helper

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `note` | `WSTRING` | 4B len + N×2B |
| `inRangeOnly` | `UINT8` | 1B |

#### `updateRegisterToMinigameHelp` — Update Registration

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `note` | `WSTRING` | 4B len + N×2B |
| `inRangeOnly` | `UINT8` | 1B |
| `wantsRequests` | `UINT8` | 1B |

#### `minigameCallAccept` / `minigameCallDecline` — Respond to Call

| Field | Type | Size |
|-------|------|------|
| `CallingPlayerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

#### `minigameCallAbort` — Cancel Active Call

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `minigameStartCancel` — Cancel Starting Game

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `minigameContactRequest` — Request NPC Contact

| Field | Type | Size |
|-------|------|------|
| `ContactId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

### Client → Server (Exposed Base Methods)

#### `minigameCallRequest` — Request Help from Player

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `RemotePlayerName` | `WSTRING` | 4B len + N×2B |
| `TipAmount` | `INT32` | 4B |

### Server → Client

#### `onStartMinigame` — Launch Game URL

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `URL` | `WSTRING` | 4B len + N×2B — Flash/browser game URL |

#### `onStartMinigameDialog` — Show Game Selection

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Name` | `WSTRING` | 4B len + N×2B |
| `Difficulty` | `WSTRING` | 4B len + N×2B |
| `TCLevel` | `INT32` | 4B |
| `Verb` | `WSTRING` | 4B len + N×2B |
| `ArchetypeBitfield` | `INT32` | 4B |
| `CanPlay` | `UINT8` | 1B |
| `CanCall` | `UINT8` | 1B |
| `CanSpectate` | `UINT8` | 1B |

#### `onStartMinigameDialogClose` / `onEndMinigame`

*(No arguments)* — 1 byte each

#### `onSpectateList` — Available Players to Watch

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `playerIds` | `ARRAY<INT32>` | 4B count + N×4B |
| `playerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `onMinigameRegistrationPrompt` — Registration Cost

| Field | Type | Size |
|-------|------|------|
| `Cost` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `minigameRegistrationInfo` — Current Registration State

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Registered` | `UINT8` | 1B |
| `InRangeOnly` | `UINT8` | 1B |
| `WantsRequests` | `UINT8` | 1B |
| `Note` | `WSTRING` | 4B len + N×2B |

#### `addOrUpdateMinigameHelper` — Helper List Entry

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `PlayerId` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Note` | `WSTRING` | 4B len + N×2B |
| `Level` | `UINT8` | 1B |
| `Archetype` | `UINT8` | 1B |
| `Friend` | `UINT8` | 1B |

#### `removeMinigameHelper`

| Field | Type | Size |
|-------|------|------|
| `PlayerId` | `INT32` | 4B |

#### `minigameCallDisplay` — Incoming Call Request

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `CallingPlayerId` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Archetype` | `INT32` | 4B |
| `Level` | `INT32` | 4B |
| `TipAmount` | `INT32` | 4B |
| `ExpiresAt` | `INT32` | 4B |
| `GameName` | `WSTRING` | 4B len + N×2B |
| `GameDifficulty` | `WSTRING` | 4B len + N×2B |
| `GameVerb` | `WSTRING` | 4B len + N×2B |
| `GameTC` | `INT32` | 4B |
| `NPCTitle` | `WSTRING` | 4B len + N×2B |

#### `minigameCallResult` — Call Outcome

| Field | Type | Size |
|-------|------|------|
| `ResultCode` | `INT32` | 4B |
| `StartTime` | `FLOAT` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `minigameCallAbort` — Call Cancelled

| Field | Type | Size |
|-------|------|------|
| `CallingPlayerId` | `INT32` | 4B |

#### `showMinigameContact` — NPC Contact Card

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Id` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Title` | `WSTRING` | 4B len + N×2B |
| `Icon` | `WSTRING` | 4B len + N×2B |
| `Time` | `INT32` | 4B |
| `Success` | `INT32` | 4B |
| `Cost` | `INT32` | 4B |

---

## Implementation Notes

- **Minigame URLs**: Games are launched via web URLs — the client embeds a browser/Flash player.
