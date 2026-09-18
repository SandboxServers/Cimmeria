---
name: difficulty-ranges
description: Minigame difficulty is 1-5 at the content/def layer but every per-game table only has tiers 1-4 — the mismatch is original, not a Cimmeria bug
metadata:
  type: project
---

Two different ranges, both correct, at two different layers. Confirmed from `deprecated/`
on 2026-09-17.

## Content / def layer asserts 1-5

- `python/cell/Minigame.py:9` docstring `@param difficulty: Difficulty level (1-5)`
- `python/cell/Minigame.py:16` `assert(1 <= difficulty <= 5)`
- `python/Atrea/__init__.py:305` `registerMinigameSession` — `@param difficulty: Difficulty level (1-5)`
- `python/base/SGWPlayer.py:80` `startMinigame` — `@param difficulty: Game difficulty (1-5)`

So a content-engine loader that range-checks `1..=5` matches the original.

## Every per-game table only has keys 1-4

`difficultyLevels` dict top-level keys, all three implemented games:

| Game | Keys | Table at |
|---|---|---|
| Livewire | 1,2,3,4 | `python/base/minigame/Livewire.py:52` |
| Alignment | 1,2,3,4 | `python/base/minigame/Alignment.py:14` |
| GoauldCrystals | 1,2,3,4 | `python/base/minigame/GoauldCrystals.py:52` |

Consequence: difficulty 5 passed the original's content-layer assert and then
**KeyError'd** in the game. Cimmeria's `LivewireGame::new` does
`session.difficulty.clamp(1, 4)` instead — a hardening, not a port. A seed row authored
with difficulty 5 is therefore silently downgraded to tier 4 rather than crashing.

Difficulty is one of the `joinOK` game params handed to the SWF
(`cpp/src/baseapp/minigame_connection.cpp:432`), alongside `techcomp`, `seed`,
`abilityBitfield`, `pclevel`, `intelligence`, `instcc`, `CA0`-`CA4`.

Note `techCompetency` is the *other* knob and it is not tiered — the per-game tables
multiply it in continuously (`Livewire.py:179-193`). Both were hardcoded to 1 in Cimmeria;
difficulty was made a `start_minigame` loader param in CA04, tech_competency is still 1.

See also [[session-lifecycle-original]], [[chain-wiring-and-gaps]].
