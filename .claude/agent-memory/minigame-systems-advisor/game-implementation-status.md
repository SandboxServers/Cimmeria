---
name: game-implementation-status
description: Which of the 10 SGW minigames are implemented server-side in Cimmeria, which Python references actually have logic, and which client SWF packages ship
metadata:
  type: project
---

# Minigame implementation status (verified 2026-09-17)

## Rust server side — `crates/services/src/minigame/games/mod.rs:10-25`

Only **two** dispatch outcomes exist:

- `"Livewire"` → `livewire::LivewireGame` — the ONLY real implementation
  (`games/livewire/mod.rs` 317 L + `setup.rs` 410 L + `tests.rs` 150 L).
- `"Hack" | "Activate" | "Analyze" | "Bypass" | "Converse" | "ConverseBasicHumanoid"`
  → `placeholder::PlaceholderGame` (`games/placeholder.rs:16-22`), which accepts
  exactly one client command, `victory`, and instantly returns `GameOutput::Victory`.
- Anything else → falls through to Placeholder with a `warn!`.

`Alignment` and `GoauldCrystals` are **commented-out TODOs** at
`games/mod.rs:13-15` — they fall into the `_` arm and auto-win as placeholders.
Do not describe them as "implemented".

## Correction to a common misreading of the Python reference

`deprecated/python/base/minigame/` has 10 files, but they are **NOT** all
`Placeholder` subclasses:

- Real `Atrea.Minigame` subclasses **with game logic**: `Livewire.py`,
  `Alignment.py`, `GoauldCrystals.py`.
- `Placeholder` subclasses (no logic): `Activate.py`, `Analyze.py`,
  `Bypass.py`, `Converse.py`, `Hack.py`.

So Alignment + GoauldCrystals have a Python reference to port from; Converse /
Hack / Bypass / Activate / Analyze genuinely never had server logic — their
rules lived in the client SWF only.

`Placeholder.py` docstring: *"Placeholder handler for minigames that are not
implemented **on the client**"* — and its `message()` accepts only `victory`.

## Client SWFs DO ship for all of them

`docs/client/ui-layout-inventory.md:286,325` — `Flash/*.upk` contains 11
packages: Activate, Alignment, Analyze, Bypass, Converse,
ConverseBasicHumanoid, CrystalGame, DHD, GoauldCrystals, Hack, Livewire.

This **contradicts** `docs/reverse-engineering/findings/minigame-architecture.md:34`
("no SWF files exist for them"). The packages exist; what is unverified is
whether the placeholder ones contain a real game or just a shell with a win
button. Treat "placeholder SWF presents a usable win affordance" as an
**untested assumption**, not a fact.

Audio evidence that the placeholder games were at least partly built:
`docs/client/audio-voice-inventory.md:515` — `activateMG.fev` has 22 FMOD events
(choose/ready/rotate/timeout/match for activate/repair/**search** variants).
So Activate was the "search a container" game family.

See also [[converse-minigame-evidence]], [[chain-wiring-and-gaps]].
