---
title: "Finding: Trade Result Client Handling"
type: reference
audience: contributors doing RE, trade and crafting workers
last_updated: 2026-09-27
---

# Finding: Trade Result Client Handling

> **Status**: The Lua side is confirmed from the client's UI source. A native C++ `Trade` class subscribes a member function to `Event_NetIn_TradeResults` (confirmed from RTTI, 2026-09-27); what that method does is unknown. It is believed to decode and forward (medium confidence) but has not been live-traced.
>
> **Sources**: client Lua and localization under `Content/UI/Core/Trade/` (the client is not in git; paths are relative to `SGWGame/`), `entities/defs/enumerations.xml`, `entities/defs/SGWPlayer.def`, and a headless-Ghidra pass over `SGW.exe` ([tools/re/ghidra-headless/](../../../tools/re/ghidra-headless/README.md)).
>
> **Related findings**: [cme-event-signal.md](cme-event-signal.md) (the `MemberCallback`/`FreeCallback` subscriber mechanism), [ability-trainer-ui.md](ability-trainer-ui.md) and [crafting-client-ui.md](crafting-client-ui.md) (the matching native `onErrorCode` subscriber).
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

- `onErrorCode` (SGWPlayer client method 121) has no Lua consumer. A native free-function subscriber bound to a `Communicator*` does exist (RTTI, 2026-09-27), but whether it renders anything is unresolved. See [ability-trainer-ui.md §2](ability-trainer-ui.md#2-onerrorcode-rendering--unresolved) (AT-E1) and [crafting-client-ui.md §3](crafting-client-ui.md) (CR-E1).
- `onPlayerCommunication` on the feedback channel (`CHAN_FEEDBACK`, 9) is the confirmed visible channel for a server-authored line, and `Trade.lua` writes its own status through `writeLocalFeedback`, which lands in the same place.

### Native dispatch

The native `onTradeResults` handler (the `Event_NetIn_TradeResults` CME event, [event-net-mapping.md](../../analysis/event-net-mapping.md)) appears to decode the two INT32 arguments and raise `Events.TradeResult` unchanged. This is **medium confidence**: read statically, not live-traced.

#### A native `Trade` subscriber exists (RTTI, 2026-09-27)

A headless-Ghidra pass (read-only, `-noanalysis`, using the script in [tools/re/ghidra-headless/](../../../tools/re/ghidra-headless/README.md)) found this MSVC RTTI type-name string at `0x01e5e800`, TypeDescriptor at `0x01e5e7f8`:

```text
.?AV?$MemberCallback@UNoSubject@EventSignal@CME@@VTrade@@P84@AEXPBVEvent_NetIn_TradeResults@@PAX@ZV5@@EventSignal@CME@@
```

Demangled by hand (Ghidra's demangler does not accept raw RTTI type-name strings), it reads:

```cpp
CME::EventSignal::MemberCallback<
    NoSubject,
    Trade,
    void (__thiscall Trade::*)(Event_NetIn_TradeResults const*, void*)>
```

The `P84@AE` fragment is the member-function pointer: `P8` plus a back-reference to `Trade`, `A` for a non-`const` `this`, `E` for `__thiscall`. `X` is the `void` return, and the argument list is `PBV…` (`const Event_NetIn_TradeResults*`) then `PAX` (`void*`).

So a native C++ `Trade` class subscribes one of its own member functions directly to `Event_NetIn_TradeResults`, through the same `MemberCallback` mechanism described in [cme-event-signal.md](cme-event-signal.md#cmemembercallback-struct-layout). It sits upstream of the `UEvent_UI_TradeResult` that `TradeWin`'s Lua binding consumes; that UI-side subscriber's destructor is `SGWScriptedWindow_X_UEvent_UI_TradeResult___GameEventHandler__vfunc_0` at `0x00ce3870`. Strictly, RTTI proves the callback type is compiled into the client, not that it is subscribed at runtime; since the type exists only to be subscribed, a live subscription is near-certain, and the live trace under Open item would confirm it.

The rest of the event's boilerplate:

| Symbol | Address | Role |
|---|---|---|
| `register_NetIn_TradeResults` | `0x00d80a30` | RTTI type-name accessor |
| `CME_EventSignal_VEvent_NetIn_TradeResults___TypedEmitInfo__vfunc_0` | `0x00d80b10` | MSVC scalar destructor |

**What was not resolved: the bound method's address and body.** A raw pointer scan for the type's Complete Object Locator returned hits at `0x01bd3bb8` and `0x01bd3be0`. The candidate vtable they led to, `0x019d8928`, turned out to be a **false positive**: it belongs to an unrelated `MemberCallback` specialization, not to `Trade`'s. Don't cite `0x019d8928` as the `Trade` callback's vtable.

What this changes: the open question is no longer "is there anything native between the wire and Lua?". There is, a `Trade` member function. What remains unknown is what it does. The reading that it decodes the two INT32s and forwards them as `UEvent_UI_TradeResult` stays **medium confidence**; it could also keep native trade state, or show something of its own for codes 3-6.

## Conclusion

The client closes its trade window only on `Completed` (1) or `Cancelled` (2). Codes 3-6 leave it open and locked on a session the server has already ended (the cell clears both players' trade state before it hands the swap to the base). The server therefore:

- sends `onTradeResults(Cancelled)` to **both** players for every refusal, and
- carries the cause in a per-side `onPlayerCommunication` feedback line ("your crafting bag does not have room ...", "your trade partner does not have the naquadah they offered", ...).

The internal abort reasons, the `trade.refused` `reason` labels and the `trade_swaps_total{outcome}` labels keep their detail. Code: `crates/base-methods/src/base/world_entry/methods/trade/execute/abort.rs` (`REFUSAL_RESULT`, `refusal_lines`).

The server's behaviour is correct whatever the native `Trade` handler turns out to do: `Cancelled` is the one code that both closes the window and prints a line, and the cause travels on `CHAN_FEEDBACK`, which the client is known to display.

## Open item

**Status: open, re-scoped.** It was "is there anything native?"; it is now "a native `Trade` handler exists, and its behaviour is unknown". The working assumption (decode and forward) is medium confidence.

To close it, find the bound method and read it. Any one of these works:

1. **Live x64dbg trace.** Put a **non-freezing** breakpoint (condition `0` plus a log command, fast resume off; see [sgw-live-debugging.md](../../guides/sgw-live-debugging.md)) on `CmeEventSignal_Subscribe` at `0x00a5c150`, filtered to the `Event_NetIn_TradeResults` signal. The logged callback object's method pointer is the handler.
2. **Read live memory.** Once you have the `Trade` subscriber's `MemberCallback` object, the bound method pointer is at `this+0x8` ([cme-event-signal.md](cme-event-signal.md#cmemembercallback-struct-layout)).
3. **Full GUI RTTI re-analysis** in Ghidra, so the Complete Object Locator chain resolves to the right vtable.

Then decompile the method and check whether it forwards codes 3-6 to Lua unchanged or has a native string or UI of its own. If it does have one, the specific codes could come back for that side.
