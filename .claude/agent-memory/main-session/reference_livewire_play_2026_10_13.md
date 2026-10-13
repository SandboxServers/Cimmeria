---
name: Livewire native lab play
description: Stage mapping and SmartFox rollover verification for lab client input
type: reference
---

# Livewire native lab play (2026-10-13)

- Source: `docs/analysis/cellblock-autoplay/livewire-autosolve.md` §2.2 and `crates/lab/src/supervisor/minigame_play.rs`. The server's current board uses `state=ready|running|finished`; there is no separate `phase=playing` field. The client flow waits for a visible MinigameWin and a ready/running board before input.
- Source: `crates/lab/src/supervisor/input.rs` and `crates/minigame/src/minigame/session_snapshot.rs`. Native CEGUI injection produces MouseMove, while Lua setPosition does not. The play flow refuses a Lua fallback before claiming N1. Tap rows are non-consuming and carry sequence, direction, command and wire name.
- Stage mapping and rollover-state unit guards were added. A real-client Livewire victory is pending lab availability; compile and unit tests are not in-game verification.
- Review correction (2026-10-13): a running board must resume its remaining plan without a second Open click. The N1 flow uses a native-only cursor placement API and returns `effect_ok: false` for unverified outcomes, so a failed native injection cannot yield an N1 UAT pass.
- Review correction (2026-10-13): `pe1` can spawn at stage x=375–440 under the mouse-opaque wireCover (source: `docs/analysis/cellblock-autoplay/livewire-autosolve.md` §1.1). Calibration now uses a solver candidate, whose exposed-cell filter excludes the cover, then moves off that wire before the first planned rollover.
- Review correction (2026-10-13): placeholder SWFs publish no board/tap, so their outcome remains unverified. The lab action can still pass after a native `win_btn` click and client window close; the UAT row must assert the resulting content effect.
