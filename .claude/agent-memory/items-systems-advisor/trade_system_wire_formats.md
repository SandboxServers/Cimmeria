---
name: trade-system-wire-formats
description: Player-to-player trade system: verified wire formats, enum values, state machine, and key traps (issue #54)
metadata:
  type: project
---

## Cell method indices (inbound, client → server)

All four are extended-encoding (index >= 61, sub-index = index - 61):

| Index | Sub-index | Method | Args |
|---|---|---|---|
| 104 | 43 | `tradeRequest` | `INT32 entityId`, `LocalTradeProposal proposal` |
| 105 | 44 | `tradeRequestCancel` | `INT32 entityId` |
| 106 | 45 | `tradeUpdateProposal` | `INT32 entityId`, `LocalTradeProposal proposal` |
| 107 | 46 | `tradeLockState` | `INT32 localVersionId`, `INT32 remoteVersionId`, `INT8 lockState` |

## Client method indices (outbound, server → client)

| Constant | Index | Method | Args |
|---|---|---|---|
| `ON_TRADE_STATE` | 144 | `onTradeState` | `INT32 EntityId`, `LocalTradeProposal`, `RemoteTradeProposal` |
| `ON_TRADE_RESULTS` | 145 | `onTradeResults` | `INT32 EntityId`, `INT32 Result` |

Already defined in `crates/services/src/cell/client_methods/player.rs` lines 96–98.
**NOT yet** in `crates/services/src/mercury/mod.rs` method_idx module — must add.

## Type definitions (from alias.xml lines 356–379)

```
LocalTradeItem:    FIXED_DICT { instanceId: INT32, slotId: INT32 }
LocalTradeProposal: FIXED_DICT { version: INT32, items: ARRAY<LocalTradeItem>, cash: INT32, lockState: INT8 }
RemoteTradeProposal: FIXED_DICT { version: INT32, items: ARRAY<InvItem>, cash: INT32, lockState: INT8 }
```

LocalTradeProposal minimum size (empty items): 4 + 4 + 4 + 1 = 13 bytes.

## ETradeLockState (enumerations.xml lines 1784–1791, wire type INT8)

```
ETRADELOCKSTATE_None              = 0
ETRADELOCKSTATE_Locked            = 1
ETRADELOCKSTATE_LockedAndConfirmed = 2
```

## ETradeResults (enumerations.xml lines 1793–1803, wire type INT8 enum but INT32 on wire)

```
Completed    = 1    ← cancel() sends THIS (not Cancelled!)
Cancelled    = 2
NoLocalSpace = 3
NoRemoteSpace = 4
NoLocalCash  = 5
NoRemoteCash = 6
```

Value 0 is unassigned.

## TRAPS

1. **onTradeResults.Result is INT32 on the wire, not INT8.** SGWPlayer.def line 1387 declares INT32 despite the enum being INT8. Serialize as 4 bytes.

2. **cancel() sends Completed (1), not Cancelled (2).** Both Python cancel paths (`cancelTrading` and `cancel()`) call `onTradeCompleted(Atrea.enums.Completed)`. This is intentional — both players get a clean close notification.

3. **QA 0.8384 client skips tradeRequest.** Python workaround at SGWPlayer.py line 1785: the QA client jumps straight to `tradeUpdateProposal`. The `tradeUpdateProposal` handler must call `beginTrading()` if no session is open, or the trade UI never works. THIS IS THE HIGHEST-RISK CORRECTNESS DETAIL.

4. **canSell() is the trade item gate** (not a separate canTrade()). Item.py line 189: `not self.bound and self.type.sellable`. Rust equivalent: `ITEM_FLAG_CAN_BE_SOLD = 1 << 10` (defined in vendor/data/mod.rs). Failed items are silently skipped, not rejected.

5. **Distance only checked at session open** (beginTrading()). Python does not poll distance during trade. Rust implementation should add a distance check in update_proposal and lock_state handlers as an enhancement.

6. **No item-level lock during trade.** Items stay in inventory during proposal phase. Re-validation happens inside confirm() / sqlx transaction commit.

## State machine summary

None → OPEN (both players get TradeTransaction ref)
OPEN ↔ PROPOSING (tradeUpdateProposal, resets partner lock)
PROPOSING → ONE_LOCKED (tradeLockState with Locked=1, version check passes)
ONE_LOCKED → BOTH_LOCKED (partner also locks)
BOTH_LOCKED → ONE_CONFIRMED (tradeLockState with LockedAndConfirmed=2)
ONE_CONFIRMED → EXECUTE (partner also confirms → sends ExecuteTrade to base)
Any state → CANCELLED (cancel, disconnect, or distance break)

Lock upgrade is blocked if localVersionId != my proposal.version OR remoteVersionId != partner.version (silently downgraded to None).

## Atomicity (Python is NOT atomic — Rust must be)

Python calls removeItems() then addItems() sequentially with no DB transaction. Rust must use a single sqlx transaction with FOR UPDATE on both sgw_inventory and sgw_player rows. Shape matches vendor/purchase/mod.rs pattern.

No new DB tables needed.

## Disconnect cleanup hook

`BaseToCellMsg::DisconnectEntity` / `DestroyEntity` in `cell/service/base_messages/mod.rs`. After bandolier flush, before `destroy_entity()`: check `entity.trade_partner_entity_id`, cancel trade, send `onTradeResults(Cancelled)` to partner. Requires new field `trade_partner_entity_id: Option<u32>` on CellEntity.

## Ghidra confirmation addresses

- `register_NetIn_TradeState` @ `0x00d80790`
- `register_NetIn_TradeResults` @ `0x00d80a30`
- `register_NetOut_TradeProposal` @ `0x00e29d30`
- `register_NetOut_TradeRequestCancel` @ `0x00e299d0`
- `register_NetOut_TradeLockState` @ `0x00e29b80`
- `SGWNetworkManager_VEvent_NetOut_TradeRequest` @ `0x00d68330`
- `SGWNetworkManager_VEvent_NetOut_TradeProposal` @ `0x00d68370`
- `SGWNetworkManager_VEvent_NetOut_TradeRequestCancel` @ `0x00d68350`
- `SGWNetworkManager_VEvent_NetOut_TradeLockState` @ `0x00d68390`
- UI pipeline: `SGWScriptedWindow_X_UEvent_UI_TradeLocalUpdate` @ `0x00ce3830`, `TradeRemoteUpdate` @ `0x00ce3850`, `TradeResult` @ `0x00ce3870`

**Why:** Full deep-dive on issue #54, 2026-05-27. All wire shapes confirmed from .def + Ghidra RTTI + Python reference.
**How to apply:** When implementing or reviewing trade system code — cross-check all serializers against these specs before approving.
