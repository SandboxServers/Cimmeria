---
name: cover-peek-los-na23
description: NA23/D-NA12 cover peek-point LoS - why an NPC at a slot needs it, the measured Cellblock peek distances, the mess-hall cost, and the LogCapture ordering trap hit while testing it
metadata:
  type: project
---

An NPC standing at its cover slot looks from the slot's peek point (cover::find_peek, SpaceManager::npc_line_of_sight), for Idle aggro, assist, attack (`los_policy=cover_peek`) and the pick shot check. Clear = peek ray OR own ray clear.

**Why:** a cover prop is a navmesh hole; the ray from behind it stops within 0.42 u, so guards spawned in cover never aggroed, and NA22's `in_cover_slot` (fire regardless) let Hallway02_Guard kill a player through two walls (UAT-1, 2026-09-25).

**How to apply:**
- Cover markers come in back-to-back pairs (/0 and /1 or /2 and /3 facing opposite ways); the spawn hold takes the nearest, and the guard usually stands ~1 u BEHIND its node, so peeks are measured from the node, not the NPC.
- Over-the-prop peeks must NOT be walk-checked: Hallway01's Mid counter is 5 u wide, walk round 9.4 u. Only side peeks get the 6 u walk check.
- Mess-hall tables are 2.3-3.4 u deep holes; from a slot a mess-hall guard sees little, holds fire 3 s, then leaves cover. Real fix is the occluder (#784), not a looser rule.
- Flank band: release at dot < -0.342, pick at dot >= 0; UAT-1 flanks were dot -0.17..-0.20.
- Test trap: under plain `cargo test` a LogCapture installed AFTER the test already ran an ai_tick made an unrelated LogCapture test in the same process see zero rows. Install LogCapture first; nextest (one process per test) hides it.

Related: [[cover-behaviour-na22]], [[faction-derived-aggro-na13]].
