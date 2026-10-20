---
name: goauld-crystals-extension-rules
description: GoauldCrystals minigame rules recovered from the SFS AS1 extension embedded in the GFX (MG-R3); victory test location, RNG truth, no shared core with CrystalGame
metadata:
  type: project
---

GoauldCrystals rules live in the embedded SFS 1.x extension (frame 3 DoAction of `GoauldCrystals.gfx`, 248 functions). Full spec: `docs/analysis/minigames/spec-goauldcrystals.md`.

- No `Swap`, `Crystal` or `checkForVictory` by name. Victory = finish-goal cell `power == true`, tested in `updatePowerStatus` and again in `sendFullGameState`.
- RNG is unseeded `Math.random`; `seed == 1` on a 4x4 board only selects the tutorial board. spec-protocol-common Q3's java.util.Random claim is wrong for this game.
- Victory ratio is `timeRemaining/maxTime` (spec-protocol-common writes `initialTime`).
- `deprecated/python/base/minigame/GoauldCrystals.py` is a Livewire copy with a match-3 table; nothing in it matches the client.
- CrystalGame is a mod-6 lights-out; it shares only SFS scaffolding, so no shared rules core.

**Why:** the packet list assumed function names and a seeded RNG that the movie does not have.
**How to apply:** for any SGW Flash minigame, decompile the frame-3 extension before trusting docs or the Python; use the scratch stack decompiler on top of `tools/minigame-swf/as2dis.py`.
