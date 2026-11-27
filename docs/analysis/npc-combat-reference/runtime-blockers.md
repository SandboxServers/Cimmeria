# NPC combat runtime blockers, re-verified (NC-07)

Static read of the code at this worktree's HEAD; nothing was run through the
lane and nothing was fixed. Line numbers are as of this commit. Each proposed
packet is a sketch for the architect, not a spec.

| # | Suspect | Verdict |
|---|---|---|
| 1 | NPC mitigation 0/0 against player damage | confirmed |
| 2 | Damage type forced to `DT_PHYSICAL` | confirmed (direct hits only) |
| 3 | A DoT kill fires no `entity_death` | fixed (NPC-vs-NPC DoT skips it by design) |
| 4 | NPC LOS and target validation before firing | not a bug (launch is not friend-gated for NPCs; one edge unverified) |
| 5 | NPC cooldown use against the 2 s cadence | not a bug; the cadence cap is confirmed |
| 6 | `effect_inert` for 736 and 1717 | confirmed |

## 1. NPC mitigation 0/0: confirmed

- The armour term is `af * max(MITIGATION - PENETRATION, 0) / 100 /
  penetration_mult` (`crates/cell-combat/src/cell/combat/damage/pipeline.rs:149-158`),
  with `af` = `PHYSICAL_AF + PHYSICAL_DENSITY` for physical
  (`pipeline.rs:229-237`).
- `PHYSICAL_AF` defaults to 0 (`crates/entity/src/stats/stat_list.rs:55`) and
  `PHYSICAL_DENSITY` to 0 (`stat_list.rs:84`).
  `MITIGATION` and `PHYSICAL_DENSITY` have max 0 (`stat_list.rs:84`, `:118`), so
  `MITIGATION` stays 0 for any NPC until its cap is raised.
- Spawn only writes `HEALTH` and `FOCUS` on an NPC
  (`crates/cell-world/src/cell/space_manager/spawn.rs:327-335`). No other
  non-test code sets AF, density or `MITIGATION` on an NPC.
- Result: `af_mitigation` is 0 for every NPC defender, so no NPC has armour
  against any player damage; only `stat_resistance` (FORTITUDE and similar, also
  0 unless seeded) and absorb pools apply.
- Smallest fix (proposed packet): derive AF and `MITIGATION` for an NPC from its
  level, or from template columns if a seed gives them, raising the
  `MITIGATION` max first (AF alone does nothing: `af * (MITIGATION - PEN)` is 0
  while `MITIGATION` is 0), at spawn
  (`spawn.rs` beside :327), then a unit test that a level-N NPC takes less
  damage from the same hit than a level-1 one. Needs a maintainer decision on
  the formula; the original data does not carry one. Files: `spawn.rs`, a new
  `npc_stats` helper, `damage_apply` tests; `damage_apply/ammo_tests.rs:103-108`
  is the pattern for the test setup.

## 2. Damage type forced to physical: confirmed for direct hits

- `damage_apply/mod.rs:221-227`: with no special ammo (`shot` is `None`,
  which is every NPC shot and every player shot with no special ammo), the
  hit's `damage_type` is `DT_PHYSICAL`. It is
  passed to the NVP damage (`mod.rs:290-300`) and the damage scripts
  (`mod.rs:305-311`, `effect_scripts.rs:211-328`), and written to the wire as
  `damage_code` (`pipeline.rs:198`, `effect_scripts.rs:258`).
- The script-aware helper `script_damage_type` (`combat/damage/absorb.rs:87-92`,
  `RangedEnergyDamage` gives `DT_ENERGY`) is used only for DoT pulses
  (`effects/pulsing/pulse.rs:251`), never on a direct hit.
- So 221 Energy Shock (effect 264, script `RangedEnergyDamage`,
  `db/resources/Effects/Seed/effects.sql:7813`) lands as physical: the physical absorb pools drain instead of the
  energy ones, and the client gets a physical `damage_code`. Script damage takes no
  armour factor (`effect_scripts.rs:206-266` only runs the shield absorb), so AF
  is not part of this bug; it is for NVP effects (item 1).
- Smallest fix (proposed packet): in `damage_apply/mod.rs`, when `shot` is
  `None`, take the type from the effect's script per effect
  (`script_damage_type(effect.script_name)`), not one type per hit; the
  `apply_damage_scripts` call at `mod.rs:305` is the place. Test: a hit by 221
  at a target with `ABSORB_ENERGY` set and `ABSORB_PHYSICAL` 0 reports
  `damage_code == DT_ENERGY` and the energy pool drains; with the fix reverted
  the pool is untouched and `damage_code == DT_PHYSICAL`, so the test
  fails. Files: `damage_apply/mod.rs`,
  `effect_scripts.rs`, `damage_apply/*_tests.rs`. NVP-only effects have no
  script, so they stay physical.

## 3. DoT kill fires no `entity_death`: fixed

- `effects/pulsing/pulse.rs:58-65` runs `dot_kill_credit` before the
  health-below check; it calls `kill_npc_out_of_band`
  (`pulse.rs:128`, `abilities/death/mod.rs:623`), which runs the full death
  resolver, then fires `entity_death` (`pulse.rs:181`).
- The exception is by design: when the DoT's invoker credits no player (an NPC
  or an NPC pet's DoT) the event is skipped (`pulse.rs:176`), or the target has
  no tag (`pulse.rs:147-149`): the corpse and loot still happen, but no
  event. A player is refused (`death/mod.rs:633`): a DoT never kills a player through this path.
- Not re-run; the death-event test is `a_lethal_pulse_raises_the_death_then_drains_health_below`
  (`pulsing/tests.rs:712-739`); the surrender and finish cases are at `:561-640`.

## 4. NPC LOS and target validation: not a bug

- LOS: the fight tick asks `attack_line_of_sight` before firing
  (`service/npc_ai/fight.rs:252`); out of LOS it paths, or holds in cover
  (`fight.rs:255-273`).
- Dead, gone and out-of-perception targets drop the engagement
  (`fight_target.rs:112-144`, checks `BSF_DEAD` and `HEALTH <= 0`). The launch
  path refuses a dead target (`use_ability/handle.rs:284`), runs
  `beneficial::target_gate` (`handle.rs:290-306`), which is player-only: NPC attackers are deliberately not
  gated (`beneficial.rs:420-422`, `:481`); it then checks range
  (`handle.rs:311`).
- `handle_use_ability` itself has no LOS test, so a caller that skips the fight
  tick has none. The 500 ms retry does not skip it: `npc_ai_retry_sweep` re-runs
  the whole `npc_ai_fight` (`dispatch.rs:401`), LOS included.
- Friendly targets: an NPC launch has no friendliness check (see above); the
  only guard is that the threat table holds hostiles. Not verified: whether
  anything but the idle aggro scan and assist (`idle_aggro.rs`, `assist.rs`) can
  put a friendly entity on an NPC's threat table; one test over `fight_target`
  would settle it.

## 5. Cooldowns against the 2 s cadence: not a bug; the cap is real

- Selection skips cooling abilities (`npc_ai/ability_select.rs:75,150`); the
  launch refuses a cooling one (`handle.rs:257`); a refused launch retries after
  500 ms (`fight.rs:522`, `fight.rs:495-505`).
- The natural fight tick runs every 20th AoI tick, about 2 s
  (`npc_ai/mod.rs:8`). When every ability is cooling, the tick returns with no
  retry scheduled (`fight.rs:419-435`).
- So an NPC fires at most once per ~2 s tick, however short the cooldown: a
  1.5 s ability waits 0.5 s, and a 3 s one fires every 4 s. A DPS cap, not a
  correctness bug.
- Smallest fix (proposed packet, optional): on `no_ability`, set `ai_retry_at`
  to the soonest cooldown expiry. Files: `fight.rs`, `ability_select.rs`.
  Test: an NPC with a 1.0 s cooldown ability fires twice inside 2.5 s.

## 6. `effect_inert` for 736 and 1717: confirmed (static)

- Effect 736 (ability 710, `effects.sql:7522`) and effect 1717 (ability 1433,
  `effects.sql:3012`) have `script_name` NULL, and `effect_nvps.sql` has no
  row for either id.
- With no script and no NVP damage the plan path is `PATH_SKIPPED` /
  `REASON_NO_SCRIPT` (`abilities/effect_plan.rs:252`), which calls `warn_inert`
  (`effect_plan.rs:306-327`): one WARN per `(ability_id, effect_id)` per throttle
  window (`effect_plan.rs:50-63`). Jaffa 710 and ScavDog 1433 hits are inert
  and the log says so.
- Fix is NC-12 (hand NVPs 470-473, D-NC2), already planned. A live run was not
  done.
