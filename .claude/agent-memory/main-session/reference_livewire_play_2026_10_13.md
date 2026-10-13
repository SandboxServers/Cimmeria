---
name: Livewire native lab play
description: Stage mapping and SmartFox rollover verification for lab client input
type: reference
---

# Livewire native lab play (2026-10-13)

- Source: `docs/analysis/cellblock-autoplay/livewire-autosolve.md` §2.2 and `crates/lab/src/supervisor/minigame_play.rs`. The server's current board uses `state=ready|running|finished`; there is no separate `phase=playing` field. The client flow waits for a visible MinigameWin and a ready/running board before input.
- Source: `crates/lab/src/supervisor/input.rs` and `crates/minigame/src/minigame/session_snapshot.rs`. Native CEGUI injection produces MouseMove, while Lua setPosition does not. The play flow refuses a Lua fallback before claiming N1. Tap rows are non-consuming and carry sequence, direction, command and wire name.
- Stage mapping and rollover-state unit guards were added. A real-client Livewire victory is pending lab availability; compile and unit tests are not in-game verification.
