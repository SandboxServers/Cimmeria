---
name: colo-combat-forensics
description: How to prove/disprove combat playtest reports from SigNoz wire logs — decoding onStatUpdate yourself, zombie-NPC detector, beam-fire-vs-death race, corpse re-create trap
metadata:
  type: reference
---

# Proving combat reports from colo wire logs (learned 2026-09-26)

- **`wire.out` onStatUpdate `decoded.sample[].value` is WRONG** — it prints the
  `Min` field (always 0). Decode `args_hex` yourself: `u32 count`, then per stat
  `StatId, Min, Current, Max` (i32 LE, alias.xml `StatUpdate`). HEALTH=7, FOCUS=8.
- Query `witness_id = N AND msg_name IN ('onStatUpdate','onStateFieldUpdate','onSequence')`
  over the whole session, sort by timestamp (stable), and track per-NPC HP vs
  `0x41` state. On 2026-09-26 this showed **zero** HP-0-without-death NPCs across
  every Cellblock session — "NPCs go to zero health and need another shot" was a
  low residual (6/250 = 2.4% bar) left by the direct-damage + RangedPhysicalDamage
  bleed pair (two onStatUpdates per hit: direct, then script bleed), not a zombie.
- Note `onEffectResults` Delta carries only the NVP direct damage; the script
  bleed (43 HP for ability 579) never appears as a combat-text number.
- "Still shooting in death animation" for Energy Shock (ability 221, seq 1866 =
  `KIS-SA_Beam_Source`, event 1001 End): the NPC's last AI fire landed 9 ms
  before the killing blow; server sent nothing from the NPC after death. The
  lingering beam is client Kismet. Event set 802 also has an Interrupt (1002,
  seq 2894) the server never sends — only a lead, unverified that it stops the beam.
- The NPC `createOnClient` cascade used to hardcode `onStateFieldUpdate(0)` +
  HEALTH 100/100, so a corpse re-entering AoI (walk back / relog / reanchor)
  came back standing and alive → "guard not hostile". Fixed via
  `NpcAoIData::from_entity` (live state_field/HEALTH/FOCUS). If a future report
  says a dead mob "stands back up", check that path first.
- Seeded-NEUTRAL chain-armed spawns (spawnlist `aggression_override=3`) are only
  armed by an edge trigger; a relog makes a fresh instance and the persisted
  logout position skips the edge. Needs a `player_loaded` re-arm chain gated on
  mission progress (chain 1009 for the Region8 guard). See [[pvp-duel-readiness]]
  for other death-tail traps.
