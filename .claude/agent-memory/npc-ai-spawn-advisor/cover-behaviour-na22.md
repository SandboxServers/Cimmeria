---
name: cover-behaviour-na22
description: NA22 cover traps - startup spawns precede cover/world-id load (sweep in cover_loaded), guards are authored AT markers, navmesh LoS from a slot reads blocked, stance ids are single-shot rows
metadata:
  type: project
---

Facts that shaped NA22 (docs/architecture/cover-system.md) and are easy to trip on again:

- **Startup order:** `start()` spawns the population BEFORE `stamp_world_rows`, ability/effect defs and the cover index load. Anything a spawn needs from those (world id, cover, `is_ranged`) is empty for startup spawns. NA22 sweeps the spawn hold in `cover_loaded()` and does the melee-only check at fight time, not spawn time.
- **Designers placed guards in cover:** 9 of Cellblock's 12 NID Guard spawns are within 1.5 u of an NA21 marker (MessHall_Guard1 0.63 u from 1200046/0, Hallway02_Guard 0.25 u from 1200034/0). Castle guards sit 1.4-3 u off markers.
- **LoS from a slot:** a cover prop is usually a navmesh hole, so `has_line_of_sight` from behind it reads blocked by construction. NA22 exempts it as `AttackLosPolicy::InCoverSlot` inside NA16's `attack_los_policy` (keyed on holding Cover Stance), so `has_los` must be computed AFTER the cover step in fight.rs or the arrival tick misses the stance.
- **Cover Stance rows 4565/1742 are single-shot (`pulse_count = 1`) with no NVPs:** `register_active_effect` never keeps an instance, so the buff lives only as the script's stat change (+100 COVER_DEFENSE) and the stance set on `CoverReservations`. COVER_DEFENSE is not read by hit resolution.
- **Scorer must skip flanked candidates** or a released NPC re-picks the same slot and oscillates.

**Why:** these were the non-obvious failure modes found while wiring hold/seek/stance.
**How to apply:** check startup ordering before relying on spawn-time lookups; keep the in-slot LoS exemption when touching fight.rs attack gates. Related: [[leash-and-fight-exit-traps]], [[harset-zone-evidence]].
