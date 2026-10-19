---
name: sgc-w1-crossings
description: SGC_W1 (world 58) story crossings - content has no dial action, instanced world means unvalidated arrival, constants corrected (dial 100 ms, hold 1.5 s)
metadata:
  type: project
---

Content-engine crossings for SGC_W1 (M1570, M1651, M1654, M1655) can only be `cross_world_teleport` from a dialog or interact chain. The engine has no dial or kawoosh action, and `stargate_dialed` / `stargate_crossed` fire only from the native `onDialGate` flow.

- World 58 is instanced, so `validate_gate_arrival` returns `Unvalidated` for any arrival there, and `cross_world_teleport` never validates. A wrong Z at the SGC is silent; coordinates must be measured.
- World 58 has a gate volume (point set 1008) but no DHD spawn and no respawner. The native player-dial path is unavailable there as seeded.
- A return to an instanced world always gets a fresh space. `player_loaded` re-fires on every return; chain 3001 stays quiet only because a completed 1559 maps to "completed", not "not_active".
- Constants (read 2026-10-18): `GATE_DIAL_DURATION` = 100 ms (`cell-world/.../gate_dial_state.rs`), `CROSSING_CINEMATIC_HOLD` = 1.5 s (`crossing_hold_state.rs`). An earlier belief of 4 s was wrong.
- Gate 27 "17 m under the floor" conflicts with the NA28 figure 3.4 m / 1.3 m in `docs/gameplay/gate-travel.md`. Unresolved without the SGC_W1 .nav.
- Chain-driven hops are invisible to telemetry unless `transport.rs` logs; no named event was found.

**Why:** the sealed note for the SGC campaign needed these before Haiku coders could write chains.
**How to apply:** put `step_status` + `world` conditions on every crossing chain, advance the step before the teleport, and never grant gate-27 or Omega addresses in an unconditional `player_loaded` chain. Related: [[cross-world-teleport-arrival-path]], [[arrival-coordinate-offnavmesh]], [[region-trigger-world-scoping]].
