---
name: reference_lab_relog_chat_2026_10_13
description: Chat capture behavior across a client relog and chat-window recreation
metadata:
  type: reference
---

LR-04b (#1679), 2026-10-13: the Lua ring wrapper is re-subscribed when
`Inst1ChatWin` changes, and repeated pumps on the same window do not add a
subscription. A gap token survives keeper and unrelated reader pumps until a
non-peek chat reader acknowledges that exact token. Failed re-subscription
returns the Lua error in `capture.lua_install` and retries later. Since the
keeper skips character select and stalled world loads, a welcome sent before
the first post-load hook install cannot be reconstructed; `capture_warning`
identifies that possible gap. The chat HUD tail is not copied into the cursor
store. The Lua 5.1 lifecycle simulation covers a welcome before installation,
window destruction/recreation, a later feedback line, single delivery and
gap acknowledgement; live relog behavior remains unverified.
