---
name: Lab chat metadata lookup
description: How client_chat_log resolves built-in chat display metadata, including feedback channel 9.
type: reference
---

2026-10-13: The stock chat reference documents channel 9 as Feedback, sky blue, Info tab, and channel 7 as lacking a `ChatMod.ChannelMap` entry. The lab metadata chunk should iterate numeric built-in IDs below `UIChannel.Chat` and call `ChannelMap[id](id, 0)` under `pcall`; resolve colour and tabs from `channelColors` and `channel2tabs`, rather than baking a color value into Rust. If `UIChannel` has no name for id 9, use the documented `Feedback` fallback. Sources: `docs/gameplay/chat-system.md` (System Channels, citing `ChatWindow.lua:1250,1274` and `:1297-1312`); `crates/lab/src/supervisor/ui/chat_log.rs`.
