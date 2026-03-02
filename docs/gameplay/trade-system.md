# Trade System

> **Last updated**: 2026-03-01
> **Status**: ~0% implemented (logic written, not wired)

## Overview

The trade system enables direct player-to-player item and currency exchange through a proposal-based trade window. Each player builds a proposal (items + cash), then both parties lock and confirm. The system uses version tracking to prevent race conditions and a three-state lock machine (None -> Locked -> LockedAndConfirmed) to ensure both parties agree before executing.

The `TradeProposal` and `TradeTransaction` classes in `python/cell/Trade.py` implement the full trade logic (274 lines). The trade UI protocol is not yet wired to entity methods.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Trade proposal model | DONE | `TradeProposal` with version tracking, validation |
| Trade transaction model | DONE | `TradeTransaction` with two-party management |
| Proposal update | DONE | Version validation, item existence check, unsellable filtering |
| Lock state machine | DONE | None -> Locked -> LockedAndConfirmed with partner reset |
| Trade confirmation | DONE | Validates both sides, swaps items and cash |
| Trade cancellation | DONE | `cancel()` notifies both parties |
| Space validation | DONE | `canAddItems()` checks before swap |
| Cash validation | DONE | Checks naquadah balance |
| Entity method wiring | NOT IMPL | No entity methods reference Trade.py |
| Trade request/accept UI | NOT IMPL | Client trade initiation protocol unknown |
| Trade window protocol | NOT IMPL | `onTradeProposalUpdated` / `onTradeCompleted` need entity defs |

## Trade Classes

### TradeProposal

Represents one player's side of a trade.

| Property | Type | Purpose |
|----------|------|---------|
| `player` | SGWPlayer | Owning player reference |
| `version` | int | Monotonic proposal version counter |
| `lockState` | int | `ETRADELOCKSTATE_*` enum value |
| `naquadah` | int | Cash offered |
| `items` | list | List of (itemId, slotId) tuples |

| Method | Purpose |
|--------|---------|
| `isValid()` | Verify sufficient cash and all items still exist |
| `update(proposal)` | Apply client proposal (version check, item validation) |
| `removeItems()` | Remove all proposed items from inventory |
| `makeItemList()` | Convert to {designId: quantity} dict for inventory add |
| `toLocalProposal()` | Serialize for owning player's view |
| `toRemoteProposal()` | Serialize for partner's view (includes item details) |

### TradeTransaction

Manages a two-player trade session.

| Method | Purpose |
|--------|---------|
| `hasPlayer(entityId)` | Check if entity is in this trade |
| `getProposal(entityId)` | Get player's own proposal |
| `getPartnerProposal(entityId)` | Get trade partner's proposal |
| `updateProposal(entityId, proposal)` | Update a player's proposal, reset partner lock |
| `updateLockState(entityId, localVer, remoteVer, lockState)` | Advance lock state machine |
| `cancel()` | Cancel trade, notify both parties |
| `confirm()` | Execute item/cash swap |

## Lock State Machine

```
ETRADELOCKSTATE_None
  |-> Player clicks "Lock"
  |-> Validates: localVersionId matches, remoteVersionId matches
  v
ETRADELOCKSTATE_Locked
  |-> Partner also locks
  |-> Player clicks "Confirm"
  v
ETRADELOCKSTATE_LockedAndConfirmed
  |-> When BOTH players reach LockedAndConfirmed:
       |-> confirm() executes the trade

Reset rules:
  - If either player updates their proposal: both lock states reset to None
  - If either player unlocks: partner's lock resets to None
  - Version mismatch prevents locking
```

## Trade Confirmation Flow

```
TradeTransaction.confirm()
  |-> Validate p1 proposal (isValid: cash + items)
  |-> Validate p2 proposal (isValid: cash + items)
  |-> Check p2 has space for p1's items
  |-> Check p1 has space for p2's items
  |-> p1.removeItems() + p2.removeItems()
  |-> p1.addItems(p2Items) + p2.addItems(p1Items)
  |-> Notify both: onTradeCompleted(Completed)

Failure codes:
  - NoLocalCash / NoRemoteCash: insufficient naquadah
  - NoLocalSpace / NoRemoteSpace: insufficient inventory space
```

## Trade Result Codes

| Code | Enum | Meaning |
|------|------|---------|
| Completed | `Atrea.enums.Completed` | Trade successful |
| NoLocalCash | `Atrea.enums.NoLocalCash` | You don't have enough cash |
| NoRemoteCash | `Atrea.enums.NoRemoteCash` | Partner doesn't have enough cash |
| NoLocalSpace | `Atrea.enums.NoLocalSpace` | You don't have inventory space |
| NoRemoteSpace | `Atrea.enums.NoRemoteSpace` | Partner doesn't have inventory space |

## Data References

- **Enumerations**: `ETRADELOCKSTATE_None`, `ETRADELOCKSTATE_Locked`, `ETRADELOCKSTATE_LockedAndConfirmed`
- **Trade result enums**: `Completed`, `NoLocalCash`, `NoRemoteCash`, `NoLocalSpace`, `NoRemoteSpace`

## RE Priorities

1. **Entity method wiring** - Connect Trade.py to SGWPlayer entity methods
2. **Trade initiation** - Client-side trade request/accept protocol
3. **Trade window UI** - `onTradeProposalUpdated` wire format
4. **Anti-exploit** - Distance checks, combat checks, busy state during trade
5. **Trade logging** - Audit trail for GM review

## Related Docs

- [inventory-system.md](inventory-system.md) - Items exchanged in trades
