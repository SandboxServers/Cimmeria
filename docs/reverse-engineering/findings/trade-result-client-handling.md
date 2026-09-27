---
title: "Finding: Trade Result Client Handling"
type: reference
audience: contributors doing RE, trade and crafting workers
last_updated: 2026-09-27
---

# Finding: Trade Result Client Handling

> **Status**: The Lua side is confirmed from the client's UI source. The native `onTradeResults` handler is believed to decode and forward (medium confidence) but has not been live-traced.
>
> **Sources**: client Lua and localization under `Content/UI/Core/Trade/` (the client is not in git; paths are relative to `SGWGame/`), `entities/defs/enumerations.xml`, `entities/defs/SGWPlayer.def`.
>
> **Found by**: the crafting campaign (CR-17, trading from the crafting bag), with a game-archaeology-specialist pass for the coordinator. Server change that followed: every trade refusal now sends `Cancelled`.

## Question

The server sent per-side `onTradeResults` codes on a failed trade (`NoLocalSpace`, `NoRemoteSpace`, `NoLocalCash`, `NoRemoteCash`, following the legacy Python `Trade.py`). What does the shipped client do with each code?

## Evidence

### The result enum

`entities/defs/enumerations.xml:1793-1803`, `ETradeResults`, declared `INT8`:

| Value | Token |
|---|---|
| 1 | `Completed` |
| 2 | `Cancelled` |
| 3 | `NoLocalSpace` |
| 4 | `NoRemoteSpace` |
| 5 | `NoLocalCash` |
| 6 | `NoRemoteCash` |

On the wire the `Result` argument of `onTradeResults` (SGWPlayer client method 145) is **INT32**, as `SGWPlayer.def` declares it, not the enum's INT8. Serializing one byte makes the client's parse fail silently: the trade UI never closes and never updates. This trap is recorded in [trade-system.md](../../gameplay/trade-system.md#wire-format-traps).

### The Lua handler

`Content/UI/Core/Trade/Trade.lua`:

- `:344` subscribes the handler: `TradeWin:subscribe( Events.TradeResult, 'TradeMod.onTradeResult' )`.
- `:305-315`, `TradeMod.onTradeResult(this, result)`:
  - `result == TradeResults.Completed`: `TradeMod.endTrade()`, local feedback `localize("Trade", 'Trade Completed')`, sound `ui/cue/submitG`.
  - `result == TradeResults.Cancelled`: `TradeMod.endTrade()`, local feedback `'Trade Cancelled'`, sound `ui/cue/submitB`.
  - There is **no `else` branch.** Codes 3-6 run nothing: `endTrade` (`:51`) never runs, no line is printed, and the window stays open in whatever lock state it had.

The trade window's other result-adjacent paths do not help. `onDragReceived` (`:208-225`) and `onItemBackgroundDrop` (`:323-338`) accept an item from any container: the client does not filter the trade window's source bag, so the server's whitelist is the only gate.

### Localization

`Content/UI/Core/Trade/Trade.int`, section `[Lua]`, has exactly four strings: `Incoming Trade`, `Trade Cancelled`, `Trade Completed` and `TradeInvitation`. There is no string for a space or cash failure, so nothing in the client could show one.

### Other channels

- `onErrorCode` (SGWPlayer client method 121) has no confirmed Lua consumer. [ability-trainer-ui.md](ability-trainer-ui.md) (AT-E1) and [crafting-client-ui.md](crafting-client-ui.md) (CR-E1) both left its rendering unresolved.
- `onPlayerCommunication` on the feedback channel (`CHAN_FEEDBACK`, 9) is the confirmed visible channel for a server-authored line, and `Trade.lua` writes its own status through `writeLocalFeedback`, which lands in the same place.

### Native dispatch

The native `onTradeResults` handler (the `Event_NetIn_TradeResults` CME event, [event-net-mapping.md](../../analysis/event-net-mapping.md)) appears to decode the two INT32 arguments and raise `Events.TradeResult` unchanged. This is **medium confidence**: read statically, not live-traced.

## Conclusion

The client closes its trade window only on `Completed` (1) or `Cancelled` (2). Codes 3-6 leave it open and locked on a session the server has already ended (the cell clears both players' trade state before it hands the swap to the base). The server therefore:

- sends `onTradeResults(Cancelled)` to **both** players for every refusal, and
- carries the cause in a per-side `onPlayerCommunication` feedback line ("your crafting bag does not have room ...", "your trade partner does not have the naquadah they offered", ...).

The internal abort reasons, the `trade.refused` `reason` labels and the `trade_swaps_total{outcome}` labels keep their detail. Code: `crates/base-methods/src/base/world_entry/methods/trade/execute/abort.rs` (`REFUSAL_RESULT`, `refusal_lines`).

## Open item

Live-trace the native `onTradeResults` handler (x64dbg, non-freezing breakpoint) to confirm it forwards codes 3-6 to Lua unchanged and has no native-side string or UI of its own. If it does have one, the specific codes could come back for that side.
