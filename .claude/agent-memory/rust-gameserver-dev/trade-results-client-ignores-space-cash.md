---
name: trade-results-client-ignores-space-cash
description: The shipped client's trade window reacts only to onTradeResults Completed(1)/Cancelled(2); codes 3-6 (space/cash) show nothing and leave the window open
metadata:
  type: project
---

Client `Content/UI/Core/Trade/Trade.lua:305-315` (`TradeMod.onTradeResult`) handles only `TradeResults.Completed` and `TradeResults.Cancelled`: each closes the window and prints a local line. `NoLocalSpace`/`NoRemoteSpace`/`NoLocalCash`/`NoRemoteCash` (3-6), which the server sends per side for Python parity, reach the handler and do nothing: no line, window stays open. Verified 2026-09-27 (crafting CR-17). The trade window's drop handler (`onDragReceived`, :208-225) accepts an item dragged from any container, so the server whitelist is the only source-bag gate.

**Why:** a refusal carried only by codes 3-6 is invisible to the player, which breaks the visible-feedback rule.

**How to apply:** any trade refusal the player should understand needs a feedback line (the base sends one per side, `trade/execute/abort.rs` `refusal_lines`). Whether to send `Cancelled` instead so the window closes is an open owner question. Related: [[inventory-lock-keys-and-failure-injection]].
