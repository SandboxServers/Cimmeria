---
name: chat-channel-client-display
description: How the SGW client displays each EChannel id (server 8 = modal prompt, 7 = Lua error) and why built-in onChatJoined is wrong; read before choosing a channel for any server->client line
metadata:
  type: project
---

Client chat Lua (`<client>/Working/SGWGame/Content/UI/Core/ChatWindow/ChatWindow.lua`, not in git) decides what a channel byte looks like (SS-C4, 2026-09-27):

- **8 (server)**: bright red line AND `PromptMod.showPrompt("Server Message")`, a modal with OK (`:160-162`). Only for broadcasts meant to interrupt (`/gmshout`, `.announce`). Old comments calling this "the red unknown-channel splash popup" were misreading this.
- **9 (feedback)**: sky-blue Info-tab line, no popup. Use for every one-player system line (welcome, GM feedback, refusals).
- **7**: no `ChatMod.ChannelMap` entry, so `onMessageReceived` calls nil and shows NOTHING. SS-C2's GM broadcast was on 7 until SS-C4.
- **onChatJoined(name, id)**: the Lua treats every join as a USER channel (`UIChannel.Chat + id`) and prints "You have joined channel". The legacy python sent it only for ids >= 12, with id - 12. Built-in ids need no registration (ORG-E1 Q5). SS-C4 removed the login burst of eight built-in joins (`on_client_ready_burst_registers_no_built_in_channel`).

**Why:** the Rust constants drifted from `enumerations.xml` for a year because nobody read the Lua. The client Lua is the fastest ground truth for display questions, faster than Ghidra.

**How to apply:** pick channels by the `CHAN_*` constants in `crates/wire/src/cell/chat.rs`, which are pinned to the XML. Never pass a literal channel to `serialize_on_player_communication`: a workspace scan test fails on one. Related: [[gm-feedback-cell-base]], [[tell-channel-and-ignore-copies]].
