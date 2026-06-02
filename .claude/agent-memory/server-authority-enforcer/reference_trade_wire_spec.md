---
name: reference-trade-wire-spec
description: Authoritative wire shapes for the four client-callable trade RPCs and the two server-to-client trade callbacks
metadata:
  type: reference
---

**Authoritative source:** `entities/defs/SGWPlayer.def` (RPC signatures) plus
`entities/defs/alias.xml` (FIXED_DICT payloads) plus
`entities/defs/enumerations.xml` (lock-state and result enums).

## Client→Server (Exposed RPCs — cell methods 104-107)

- `tradeRequest(INT32 EntityId, LocalTradeProposal LocalProposal)` —
  `SGWPlayer.def:1034-1038`, cell method index 104.
- `tradeRequestCancel(INT32 EntityId)` — `SGWPlayer.def:1047-1050`, cell method
  index 105.
- `tradeUpdateProposal(INT32 EntityId, LocalTradeProposal LocalProposal)` —
  `SGWPlayer.def:1053-1057`, cell method index 106. Note: the
  `Event_NetOut_TradeProposal` C++ class name on the client is shortened from
  the RPC name `tradeUpdateProposal`.
- `tradeLockState(INT32 LocalVersionId, INT32 RemoteVersionId, INT8 LockState)` —
  `SGWPlayer.def:1067-1072`, cell method index 107.

## FIXED_DICT payloads

`LocalTradeProposal` (sent client→server in `tradeRequest` and
`tradeUpdateProposal`):

```
version    : INT32
items      : ARRAY<LocalTradeItem>
cash       : INT32          (naquadah delta)
lockState  : INT8           (ETradeLockState enum)
```

`LocalTradeItem` (element in the items array):

```
instanceId : INT32          (server-side item-row primary key)
slotId     : INT32
```

Note: no quantity field — quantity is server-resolved from the item row.

`RemoteTradeProposal` (sent server→client in `onTradeState`) replaces the
`LocalTradeItem` array with a full `InvItem` array; the client gets the rich
item view of the partner's offer.

## Enums

`ETradeLockState` (INT8) — `entities/defs/enumerations.xml:1784-1791`:

- 0 = `ETRADELOCKSTATE_None`
- 1 = `ETRADELOCKSTATE_Locked`
- 2 = `ETRADELOCKSTATE_LockedAndConfirmed`

`ETradeResults` (INT8) — `entities/defs/enumerations.xml:1793-1803`:

- 1 = `Completed`
- (2, 3 not enumerated in the file? — verify if needed)
- 4 = `Cancelled` (inferred from deprecated Python use)
- 5 = `NoLocalCash`
- 6 = `NoRemoteCash`
- (NoLocalSpace, NoRemoteSpace also referenced — verify exact values from
  enumerations.xml when implementing)

## Server→Client callbacks (cell methods 144-145, defined but never emitted)

- `onTradeState(INT32 EntityId, LocalTradeProposal LocalProposal,
  RemoteTradeProposal RemoteProposal)` — `SGWPlayer.def:1378-1382`, method 144.
- `onTradeResults(INT32 EntityId, INT32 Result)` — `SGWPlayer.def:1385-1388`,
  method 145.

Both are decoded by `crates/services/src/wire_log/decoders/generated.rs` and
constant-defined in `crates/services/src/cell/client_methods/player.rs:96,98`
but **no production code path in `crates/` emits them today**.

## Non-Exposed server-internal RPCs (not client-callable, do NOT add to dispatch)

- `tradeRequestFromEntity(INT32, RemoteTradeProposal)` —
  `SGWPlayer.def:1041-1044`.
- `updateTradeState(MAILBOX, INT32, RemoteTradeProposal)` —
  `SGWPlayer.def:1060-1064`.
- `updateTradeLockState(INT32, INT32, INT32)` — `SGWPlayer.def:1080-1084`.
- `tradeCancel(INT32 EntityId)` — `SGWPlayer.def:1087-1089`.

These are cell-to-cell or base-to-cell internal RPCs invoked by the *server*
during trade orchestration. They should never accept a client-originated packet.

## Ghidra anchors (for x64dbg follow-up)

- Outbound emit class strings:
  - `019d898c` `Event_NetOut_TradeRequestCancel`
  - `019d89c0` `Event_NetOut_TradeLockState`
  - `019d89f0` `Event_NetOut_TradeProposal` (this is `tradeUpdateProposal` on
    the wire)
  - `01e2c224` `.?AVEvent_NetOut_TradeRequest@@` (mangled type name)
- Mercury method-name strings: `019c2f20` `tradeRequest`, `019c2f3c`
  `tradeRequestCancel`, `019c2f5c` `tradeUpdateProposal`, `019c2f7c`
  `tradeLockState`.
- Client `SGWNetworkManager::EventHandler` vfunc destructors: `00d68330`,
  `00d68350`, `00d68370`, `00d68390`.
- UI verb strings (button-handler entry points): `01952a70` `requestTrade`,
  `01952a58` `cancelTrade`, `01952a3c` `setTradeItem`, `019529b4`
  `setTradeCash`, `01952944` `setTradeLockState`, `019529d0`
  `removeTradeItem`. All wired into `00ad5b7c` (CEGUI_ButtonBase_3).
