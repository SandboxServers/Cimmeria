---
title: "Trade System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Trade System

> **Last updated**: 2026-07-25
> **Status**: Implemented and wired end-to-end. Not yet verified with two live clients.

## Overview

The trade system enables direct player-to-player item and currency exchange through a proposal-based trade window. Each player builds a proposal (items + cash), then both parties lock and confirm. The system uses version tracking to prevent race conditions and a three-state lock machine (None -> Locked -> LockedAndConfirmed) to ensure both parties agree before executing.

The Rust implementation lives in [`crates/services/src/cell/cell_methods/player/trade/`](../../crates/services/src/cell/cell_methods/player/trade/). Session state hangs off the two `CellEntity`s (`trade_partner_entity_id` + `trade_proposal`); the atomic swap happens base-side, where the cell hands the to-be-executed proposals over as `CellToBaseMsg::ExecuteTrade` and the base wraps the whole exchange in a single sqlx transaction ([`base/world_entry/methods/trade/execute/`](../../crates/services/src/base/world_entry/methods/trade/execute/)).

## Wire Methods

| Direction | Index | Method | Purpose |
|-----------|-------|--------|---------|
| C → S | 104 | `tradeRequest` | Open a session with a partner |
| C → S | 105 | `tradeRequestCancel` | Close an open session |
| C → S | 106 | `tradeUpdateProposal` | Push a new offer |
| C → S | 107 | `tradeLockState` | Transition the lock state |
| S → C | 144 | `onTradeState` | Broadcast both proposals to one player |
| S → C | 145 | `onTradeResults` | Terminal notification (commit / cancel) |

The real client method names are `onTradeState` / `onTradeResults`. An earlier revision of this doc guessed `onTradeProposalUpdated` / `onTradeCompleted`; those names do not exist.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Trade proposal model | DONE | `TradeProposal` in `crates/entity/src/trade.rs`, with version tracking |
| Session lifecycle | DONE | `trade/state.rs` — `begin_trading`, `apply_proposal`, `cancel_session`, `clear_trade_state` |
| Proposal update | DONE | `tradeUpdateProposal` (106) → `trade/handlers.rs` |
| Lock state machine | DONE | None -> Locked -> LockedAndConfirmed with partner reset |
| Trade confirmation | DONE | `trade/handoff.rs:request_execute_trade` → base-side single-transaction swap |
| Trade cancellation | DONE | Sends `onTradeResults` to both parties |
| Disconnect teardown | DONE | `cancel_trade_on_disconnect` closes the session with `Cancelled` |
| Range gate | DONE | `partners_in_range` enforces `MAX_INTERACT_DISTANCE = 5.0`, the same gate as vendor / dialog interactions |
| Negative-cash guard | DONE | Base rejects a proposal carrying negative cash before the swap |
| Entity method wiring | DONE | Methods 104–107 dispatch through `player/dispatch.rs` |
| Remote item detail | PARTIAL | The cell does not own full inventory state (base does), so `onTradeState` pads the partner's `RemoteTradeProposal` with sentinel-bearing `InvItem` stubs rather than real item detail — see `trade/wire.rs:stub_inv_items_for` |
| Two-client verification | UNVERIFIED | Covered by unit + validation tests; no recorded live playtest with two connected clients |

## Data Model

### TradeProposal

Represents one player's side of a trade (`crates/entity/src/trade.rs`).

| Field | Type | Purpose |
|-------|------|---------|
| `version` | i32 | Monotonic proposal version counter |
| `lock_state` | i8 | `ETRADELOCKSTATE_*` value |
| `naquadah` | i32 | Cash offered |
| `items` | `Vec<TradeItem>` | `{ instance_id, slot_id }` per offered slot |

`TradeItem` carries only the runtime inventory instance id and slot index. The partner-facing `InvItem` payload is meant to be reconstructed server-side from the sender's inventory; see the "Remote item detail" gap under [Remaining Work](#remaining-work).

### Session state

There is no standalone transaction object. A session is represented by the pair of `CellEntity` fields on the two participants:

| Field | Purpose |
|-------|---------|
| `trade_partner_entity_id` | `Some(partner)` while a session is open; the session-membership check |
| `trade_proposal` | This player's current `TradeProposal` |

Lifecycle helpers live in `trade/state.rs`: `begin_trading`, `apply_proposal`, `cancel_session`, `clear_trade_state`, `partners_in_range`, and the disconnect hook `cancel_trade_on_disconnect`.

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
Both sides reach LockedAndConfirmed
  |-> Cell: request_execute_trade (trade/handoff.rs) — final cell-side checkpoint
  |-> CellToBaseMsg::ExecuteTrade { both proposals }
  |
  v
Base (world_entry/methods/trade/execute/):
  |-> Reject if either proposal carries negative cash
  |-> Single sqlx transaction: move items both ways, adjust both cash balances
  |-> Success: onTradeResults(Completed) to both
  |-> Failure: asymmetric per-side result codes
     (NoLocalCash / NoRemoteCash / NoLocalSpace / NoRemoteSpace)
```

## Trade Result Codes

`ETradeResults`, from `entities/defs/enumerations.xml`. Value 0 is intentionally unused.

| Value | Name | Meaning |
|-------|------|---------|
| 1 | `Completed` | Trade successful — **also sent on a user-initiated cancel** |
| 2 | `Cancelled` | Disconnect, distance-break, or atomic-commit failure |
| 3 | `NoLocalSpace` | You don't have inventory space |
| 4 | `NoRemoteSpace` | Partner doesn't have inventory space |
| 5 | `NoLocalCash` | You don't have enough cash |
| 6 | `NoRemoteCash` | Partner doesn't have enough cash |

## Wire-Format Traps

Two quirks the implementation preserves byte-for-byte, both of which cause silent client-side failures if got wrong:

1. **`onTradeResults.Result` is INT32**, even though `ETradeResults` is declared INT8 in `enumerations.xml`. `SGWPlayer.def` declares the field as INT32. Serializing it as one byte produces a silent client parse failure — the trade UI never closes and never updates.
2. **`tradeRequestCancel` sends `Completed` (1), not `Cancelled` (2).** Both players get a clean shutdown notification regardless of who aborted. `Cancelled` is reserved for paths where the trade was never confirmed.

## Data References

- **Lock-state enum**: `ETradeLockState` — `None` (0), `Locked` (1), `LockedAndConfirmed` (2)
- **Result enum**: `ETradeResults` — see table above
- **Aliases**: `LocalTradeProposal`, `RemoteTradeProposal`, `LocalTradeItem` in `entities/defs/alias.xml`
- **Rust types**: [`crates/entity/src/trade.rs`](../../crates/entity/src/trade.rs)

## Remaining Work

1. **Two-client playtest** — the flow has unit and validation coverage but has not been exercised with two live clients
2. **Remote item detail** — `onTradeState` currently pads the partner-facing proposal with sentinel `InvItem` stubs because the cell doesn't hold full inventory state; the partner therefore can't see real item detail in the trade window
3. **Proposal rate limiting** — version monotonicity rejects replay but does not cap throughput; a malicious client can spam `tradeUpdateProposal` and force an `onTradeState` broadcast per message. A per-session minimum interval is deferred (see the note in `trade/handlers.rs`)
4. **Combat / busy-state gate** — distance is enforced, but nothing blocks opening a trade mid-combat
5. **Trade logging** — no audit trail for GM review

## Related Docs

- [inventory-system.md](inventory-system.md) - Items exchanged in trades
