---
name: minigame-launch-flow-facts
description: Minigame start dialog (client 53/54, cell 24/30) contract, bit 4/6 client reactions, techcomp range, README M7/M8 confirmed; from MG-R7
metadata:
  type: project
---

Facts confirmed in MG-R7 (`docs/analysis/minigames/spec-launch-flow.md`):

- Client 53 `onStartMinigameDialog` needs no client patch: client
  `UI/Core/StartMinigame/StartMinigame.lua` Play → `playMinigame()` → cell 24;
  Cancel/close → `minigameStartCancel()` → cell 30. Both cell methods take no
  args, so the server must hold the pending offer.
- A server-sent 54 `onStartMinigameDialogClose` runs the same handler as Cancel,
  so the client **echoes cell 30**. Cell 30 without a pending offer must be a no-op.
- `ArchetypeBitfield` (arg 5 of 53) is a display-only hint; it is NOT joinOK's
  `abilityBitfield`. The bit → ability map is only in the SWFs.
- Client reacts to state flag bit 4 (`FUN_00e31aa0`, minigame UI lock; others see
  it via `targetIsPlayingMinigame`) and bit 6 (`FUN_00dfff70`, movement input off).
  Original Python set both at dialog time, not at cell 24. Movement locks are counted.
- `techcomp` range 1-55 (`Net.py:175`); item `tech_comp` 3-55; original always sent 1.
- M8: minigame consumables are unobtainable in seeds; only instrument 2907 is a reward.
- M7: ability 802 has no effects; event set 1132 is client VFX only, referenced nowhere.
- Effect 1939 (Prison Boot) is a no-op today; prison boot holds nobody.

**Why:** these decide whether minigame launch work needs a client patch.
**How to apply:** cite the spec before touching minigame launch or state flags.
