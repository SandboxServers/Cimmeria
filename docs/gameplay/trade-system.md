---
title: "Trade System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Trade System

> **Last updated**: 2026-09-27
> **Status**: Implemented and wired end-to-end. Not yet verified with two live clients.

## Overview

The trade system enables direct player-to-player item and currency exchange through a proposal-based trade window. Each player builds a proposal (items + cash), then both parties lock and confirm. The system uses version tracking to prevent race conditions and a three-state lock machine (None -> Locked -> LockedAndConfirmed) to ensure both parties agree before executing.

The Rust implementation lives in [`crates/cell-methods/src/cell/cell_methods/player/trade/`](../../crates/cell-methods/src/cell/cell_methods/player/trade/). Session state hangs off the two `CellEntity`s (`trade_partner_entity_id` + `trade_proposal`); the atomic swap happens base-side, where the cell hands the to-be-executed proposals over as `CellToBaseMsg::ExecuteTrade` and the base wraps the whole exchange in a single sqlx transaction ([`base/world_entry/methods/trade/execute/`](../../crates/base-methods/src/base/world_entry/methods/trade/execute/)).

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
| Source bags | DONE | The backpack (1) and the crafting bag (15); everything else is refused. See [Which items can be traded](#which-items-can-be-traded) |
| Destination bag | DONE | Chosen per item from its `container_sets`; a full destination bag refuses the whole trade |
| Refusal feedback | DONE | A feedback line to each player naming the cause, because the client shows nothing for the space and cash codes |
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
  |-> Single sqlx transaction:
  |     advisory locks (both players, lower player_id first: keys 0, 1, 15)
  |     -> offered item rows (owner, bound, source bag)
  |     -> destination bag rows (slot reservation)
  |     -> sgw_player rows, ascending (cash check)
  |     -> move items both ways, adjust both cash balances
  |-> Success: onTradeResults(Completed) to both
  |-> Failure: asymmetric per-side result codes
     (NoLocalCash / NoRemoteCash / NoLocalSpace / NoRemoteSpace),
     plus a feedback line to each player naming the cause
```

The lock order is the shared inventory order (`crates/base-session/src/base/crafting/inventory_locks.rs`): every advisory lock, then inventory rows, then player rows. Crafting completions, vendor purchase, sale and buyback, the move path, item use and gate mail take the player-wide key 0 first too, so a trade and any of them on the same player wait for each other instead of deadlocking. The live-DB guard is `trade::tests::crafting_bag::trade_and_crafting_completion_on_one_player_serialize`.

## Which items can be traded

The server decides this from the item rows; the wire carries only instance ids.

| Source | Tradeable | Why |
|--------|-----------|-----|
| Backpack (1) | Yes | |
| Crafting bag (15) | Yes | Crafting components (`container_sets` `{17,15}`) live here and cannot sit in the backpack. Owner decision 2026-09-27 (crafting campaign); mail took the same rule |
| Mission bag (2), bandolier (3), equipment (4-14) | No | Unequip or unload first. Trading equipped gear would strip it while the cell keeps its stats |
| Buyback (16) | No | Only the seller may buy it back |
| Vaults (17-20) | No | Reachable only through a banker |

Bound items are refused from any bag. An item whose type has no `resources.items` row is refused rather than placed by guesswork.

Each item lands in the recipient's bag given by its `container_sets`, with the bag it came from as the request (`item_placement::grant_container`, the rule grants use):

- a crafting component goes to the crafting bag, from either bag;
- a backpack item goes to the backpack;
- an item that lists no carried bag (a `{2}` mission type) keeps the bag it was traded from, as before.

Slots are reserved per recipient and bag, lowest free slot first. Slots the recipient is trading away in the same bag count as free. The backpack holds 40, the crafting bag 100. A destination bag without room refuses the whole trade; it never spills into the other bag. Items are moved, not merged into the recipient's existing stacks.

After the commit both players get a full inventory update, which also refreshes their crafting options when a Field Crafting Tool entered or left a crafting bag.

A traded component that a queued craft named is caught when the craft completes: the completion re-checks ownership and refuses with `component_missing`. The station and tool gate is checked only when a craft is requested, so a Field Crafting Tool traded away still covers crafts already in the queue.

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

The shipped client acts on only two of these. `Trade.lua` `TradeMod.onTradeResult` (client `Content/UI/Core/Trade/Trade.lua:305-315`) closes the window and prints "Trade Completed" or "Trade Cancelled" for 1 and 2, and does nothing for 3-6: no line, and the window stays open. The server therefore follows a space or cash code with a feedback line to each player ("Trade cancelled: your crafting bag does not have room ...", "... your trade partner does not have the naquadah they offered."), and a bound, untradeable-bag or unknown-type item with "one of the items you offered cannot be traded" to both. Whether to send `Cancelled` instead of the specific codes, so the window closes, is an open question.

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
5. **Trade logging** — every committed move logs `trade.item_moved` (bags and slots before and after) and every refusal `trade.refused`, but there is no GM-facing audit view
6. **Window stays open on a space or cash refusal** — see [Trade Result Codes](#trade-result-codes)

## Related Docs

- [inventory-system.md](inventory-system.md) - Items exchanged in trades
