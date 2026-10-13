---
name: reference_lab_relog_chat_2026_10_13
description: Chat capture behavior across a client relog and chat-window recreation
metadata:
  type: reference
---

LR-04b (#1679), 2026-10-13: the Lua ring wrapper is re-subscribed when
`Inst1ChatWin` changes, and repeated pumps on the same window do not add a
subscription. A gap marker survives keeper installs (`max = 0`) until a reader
pump reports it. Since the keeper skips stalled world loads, a welcome sent
before the first post-load hook install cannot be reconstructed; `capture_warning`
identifies that possible gap. The chat HUD tail is not copied into the cursor
store. The Lua 5.1 lifecycle simulation covers wrapper install, window
destruction/recreation, a feedback line, and single delivery; live relog
behavior remains unverified.
