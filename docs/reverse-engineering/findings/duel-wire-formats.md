# Duel System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `SGWPlayer.def`, `alias.xml`

---

Defined directly on `SGWPlayer.def`.

### Client → Server

#### `sendDuelChallenge` (Base Method, Exposed) — Issue Challenge

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aSquadDuel` | `INT8` | 1B — 1 for squad duel |

#### `sendDuelResponse` (Cell Method, Exposed) — Accept/Decline

| Field | Type | Size |
|-------|------|------|
| `aResponse` | `INT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `duelForfeit` (Cell Method, Exposed) — Surrender

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

### Server → Client

#### `onDuelChallenge` — Incoming Challenge

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aEntityId` | `INT32` | 4B — challenger entity ID |
| `aSquadList` | `ARRAY<INT32>` | 4B count + N×4B — challenger's squad |

#### `onDuelEntitiesSet` — Duel Participants

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aEntityList` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onDuelEntitiesRemove` — Participant Defeated

| Field | Type | Size |
|-------|------|------|
| `aEntityId` | `INT32` | 4B |

#### `onDuelEntitiesClear` — Duel Ended

*(No arguments)* — 1 byte
