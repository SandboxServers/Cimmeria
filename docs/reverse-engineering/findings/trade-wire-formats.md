# Trade System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `SGWPlayer.def`, `alias.xml`

---

Defined directly on `SGWPlayer.def`.

### Client → Server (Exposed Cell Methods)

#### `tradeRequest` — Initiate Trade

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `EntityId` | `INT32` | 4B — target entity |
| `LocalProposal` | `LocalTradeProposal` | FIXED_DICT |

**`LocalTradeProposal` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `version` | `INT32` | 4B |
| `items` | `ARRAY<LocalTradeItem>` | 4B count + N×8B |
| `cash` | `INT32` | 4B |
| `lockState` | `INT8` | 1B |

**`LocalTradeItem` FIXED_DICT layout** (8 bytes):

| Field | Type | Size |
|-------|------|------|
| `instanceId` | `INT32` | 4B |
| `slotId` | `INT32` | 4B |

#### `tradeUpdateProposal` — Update Trade Offer

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `EntityId` | `INT32` | 4B |
| `LocalProposal` | `LocalTradeProposal` | FIXED_DICT |

#### `tradeRequestCancel` — Cancel Trade

| Field | Type | Size |
|-------|------|------|
| `EntityId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `tradeLockState` — Lock/Unlock/Confirm

| Field | Type | Size |
|-------|------|------|
| `LocalVersionId` | `INT32` | 4B |
| `RemoteVersionId` | `INT32` | 4B |
| `LockState` | `INT8` | 1B |

**Total wire size**: 1B header + 9B = **10 bytes**

### Server → Client

#### `onTradeState` — Trade Window Update

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `EntityId` | `INT32` | 4B |
| `LocalProposal` | `LocalTradeProposal` | FIXED_DICT |
| `RemoteProposal` | `RemoteTradeProposal` | FIXED_DICT |

**`RemoteTradeProposal` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `version` | `INT32` | 4B |
| `items` | `ARRAY<InvItem>` | 4B count + N×InvItem (variable — ammoTypes array) |
| `cash` | `INT32` | 4B |
| `lockState` | `INT8` | 1B |

**Note**: `RemoteTradeProposal` uses `InvItem` (full item data) while `LocalTradeProposal` uses `LocalTradeItem` (just instance + slot IDs). This is because you already have your own items in your inventory — you only need full data for the other player's items.

#### `onTradeResults` — Trade Complete/Cancelled

| Field | Type | Size |
|-------|------|------|
| `EntityId` | `INT32` | 4B |
| `Result` | `INT32` | 4B — success/failure code |

**Total wire size**: 1B header + 8B = **9 bytes**

---

## Implementation Notes

- **Trade asymmetry**: Your items are `LocalTradeItem` (instance+slot), other player's items are `InvItem` (full data). This reduces bandwidth since you already have your own item data.
