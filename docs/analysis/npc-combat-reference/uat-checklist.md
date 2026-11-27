# NPC combat reference: Z12 live UAT checklist (NC-33)

Status: written, not run. Nothing here is verified in the client. Every
pass/fail cell is blank until a lab session (NC-40 to NC-44, D-NC7) fills it.

- Lab spec: `docs/guides/uat-specs/debug-area.toml`, rows `NC-U1` to `NC-U5`
  (the four reference blocks). The telemetry block is server-log and SigNoz
  only and has no toml row.
- Range: Debug Area zone Z12, world 1300, spawns 14300 to 14304, tags
  `DebugArea_Ref_NID`, `_PRU`, `_ScavDog`, `_JaffaGuard`, `_JaffaLieutenant`.
  Reference: [Combat reference range](../../content/debug-area.md#combat-reference-range).
- Evidence class target: a row that is seen in the client is marked
  `ORIGINAL CLIENT VERIFIED`. A row that rests on a reconstructed value stays
  `RECONSTRUCTED` even when it passes. Nothing here upgrades a class by itself.

## Lab gotchas (read before every block)

- Clear dialogs and any open window (Escape twice) before the first click or
  target. A dialog left open swallows the Z12 targeting press.
- Stand 4 to 6 m off the target. The interact range is 5 u, so a closer
  stand is still in reach of every click, and a farther one may leave the
  NPC's aggro radius.
- `.gotolocation DebugArea <x> <y> <z>` always moves the tester, never the
  selection. Target after the move, not before.
- Setup: the Z1 arrival (respawner 130's point, 251 8 -962), then the Z2
  granter (spawn 13020) for the ability set, the Basic Equipment Quartermaster
  (spawn 13001) for the weapon and the munitions vendor (spawn 13006) for
  ammo. A fresh character already holds the starter kit (SI 3 9mm Pistol and
  the five starter abilities), so the vendors matter only once that ammo runs
  out. Then walk to Z12 (about 280 to 370 m west of the Z1 arrival).
- Teardown returns to the Z1 arrival.

## Block 1: NID Guard (template 24, spawn 14300, set 3 = 559)

Expected server evidence for the NID, from the NC-11 reference tests: the
SMG ability is effect set 15 (ability 559, `Generic burst weapon use`) with
effect 621 (`RangedPhysicalDamage`) and `WP-Human.WP_SMG_1A`. Loot is table 2,
respawn 20 s. The 559 tooltip gives -10 Health and -100 Force per burst
(D-NC8, RECONSTRUCTED). Effect 621 executes 20 Health and 200 Force today, so
the damage cell records the observed number and the mismatch is the
UNRESOLVED item O1 in the registry. A mismatch is not a presentation fault.

| Row | Step | Expected client presentation | Expected server evidence | Pass / fail | Class target |
|---|---|---|---|---|---|
| N1 (NC-U1) | Stand 5 m off it at 95 5.8 -860 and draw its fire with your Pistol Shot | It acquires you and fires short SMG bursts from the muzzle, with a flash or tracer and the SMG sound | `npc_ai.aggro` acquired with tag `DebugArea_Ref_NID`; the ability 559 use | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| N2 (NC-U1) | Hit it so it takes cover | It steps behind the cover node and keeps firing from there | `use_cover` in the npc log; the cover node chosen | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| N3 | Read your Health after one burst | The health bar drops by the burst's damage | The damage number in the combat log: 10 H per the 559 tooltip (D-NC8, the expected value); effect 621 executes 20 H today, so record the observed value as O1 | [ ] pass [ ] fail | RECONSTRUCTED (D-NC8) |
| N4 (NC-U1) | Kill it | It falls, leaves loot from table 2 you can pick up | Loot roll for table 2 in the server log; no loot from the wrong table | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| N5 (NC-U1) | Wait about 20 s after the kill | It stands again at spawn 14300 | `spawner.npc_respawn` with `respawn_secs` 20 for template 24 | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |

## Block 2: Praxis Jaffa Guard (template 1380, spawn 14303) and Lieutenant (1381, spawn 14304)

The guard and lieutenant share the set: 584 (Staff Auto Attack, the ranged bolt) and 710
(Staff Melee AA, the staff swing), faction 10, a 20 s respawn. Templates 1380 and 1381
copy 1371 and are byte-identical to it. The staff in hand is the existing look;
NC-22 adds the staff to 159 and 160 only (D-NC3), not to these pens. The
melee damage rides effect 736 (NC-12, hand effects 470 to 473).

| Row | Step | Expected client presentation | Expected server evidence | Pass / fail | Class target |
|---|---|---|---|---|---|
| J1 (NC-U2) | Stand 15 m off the guard at 55 5.8 -890 and target it | The serpent staff is visible in its hand, it raises it and a ranged bolt flies at you with its sound | Ability 584 used by `DebugArea_Ref_JaffaGuard` | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| J2 (NC-U3) | Walk to 2 m (42 5.8 -890) and wait | It swings the staff (ability 710): staff arc and sound, your Health drops | Ability 710 use; melee damage from effect 736 (NC-12) | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED (swing and sound); RECONSTRUCTED (D-NC2) for the damage number |
| J3 (NC-U3) | Walk back out to about 8 m | It stops swinging and returns to the bolt | The ability choice flips back to 584 once you are out of reach (NC-20) | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| J4 | Repeat J1 on the lieutenant at spawn 14304 (40 5.25 -900) | The lieutenant shows the same staff and bolt, beside the guard | Ability 584 from `DebugArea_Ref_JaffaLieutenant` | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |

## Block 3: ScavDog (template 76, spawn 14302, set 6 = 1433)

Set 6 is RECONSTRUCTED (NC-13). The ScavDog has no Pistol Shot and no gun.
Its only ability is Mob Strike (1433), in reach, so every presentation check
here is a presentation check on a reconstructed ability.

| Row | Step | Expected client presentation | Expected server evidence | Pass / fail | Class target |
|---|---|---|---|---|---|
| S1 (NC-U4) | Stand 5 m off at 95 5.8 -900 for 5 s | No gun, no muzzle flash, no shot, no projectile at any point | No ability use from set 6 while you are out of reach | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| S2 (NC-U4) | Walk into reach (91.5 5.8 -900) | It bites: a creature attack animation and a creature attack sound | Ability 1433 used once you are in reach | [ ] pass [ ] fail | RECONSTRUCTED (NC-13) |
| S3 (NC-U4) | Read your Health after the bite | The health bar drops | Melee damage from the reconstructed set | [ ] pass [ ] fail | RECONSTRUCTED (NC-13) |
| S4 (NC-U4) | Hit it, then kill it | It reacts to the hit and falls when killed | Death and respawn in 20 s | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |

## Block 4: Prisoner Retrieval Unit (template 4, spawn 14301, drone ability 221)

| Row | Step | Expected client presentation | Expected server evidence | Pass / fail | Class target |
|---|---|---|---|---|---|
| P1 (NC-U5) | Stand 5 m off at 65 5.8 -850 and target it | An energy beam or projectile leaves its emitter toward you, with its sound | Ability 221 used by `DebugArea_Ref_PRU` | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| P2 (NC-U5) | Read your Health after the shock | The health bar drops | The shock damage (NC-14 pins the defs) | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |
| P3 (NC-U5) | Watch the drone | The drone moves to its firing position and back | The drone's ability state in the log | [ ] pass [ ] fail | ORIGINAL CLIENT VERIFIED |

## Block 5: Telemetry (no client driving)

Run after the four blocks, from the server log or SigNoz. No client row drives
these checks.

| Row | Check | Expected | Pass / fail |
|---|---|---|---|
| T1 | No `combat_profile_placeholder` for templates 24, 4, 76, 1380 or 1381 | Zero rows | [ ] pass [ ] fail |
| T2 | No `effect_inert` for effects 736 or 1717 | Zero rows (NC-12 closed both) | [ ] pass [ ] fail |
| T3 | Every `combat_profile_*` line carries its template name next to the id (R8) | The name and the id appear together | [ ] pass [ ] fail |

## Owner decisions that touch this checklist

- D-NC3 (staff on 159 and 160, RECONSTRUCTED): no row here; the pens keep
  their look. Lineup rows follow `docs/content/debug-area.md`.
- D-NC6 (`weapon_item_id` informational): the NID's `weapon_item_id` 21 has no
  server consumer, so no row checks it.
- D-NC7 (lab approved in principle): this checklist is written, not run. The
  lab sessions are NC-40 to NC-44 and each needs a fresh go-ahead.
- D-NC8 (follow the 559 tooltip, RECONSTRUCTED): N3 expects the tooltip's
  10 Health per burst and records the observed value. The effect row is not
  changed here; the mismatch stays UNRESOLVED (O1).
