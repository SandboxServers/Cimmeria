# Effect scripts run AFTER damage_apply's death check

**Read before touching `cell/abilities/damage_apply`, `cell/effects/scripts.rs`, or
any new kill path.**

## The trap

`apply_damage_to_target` resolves in this order:

1. `combat::calculate_damage` — direct NVP damage
2. lethality probe on the resulting HEALTH
3. wire packets (`onEffectResults`, `onStatUpdate`)
4. **effect-script dispatch** — `RangedPhysicalDamage`, `MeleePhysicalDamage`,
   `RangedEnergyDamage`, `Suppression` all write `HEALTH` directly with
   `stat.update(min, (cur - dmg).max(0), max)` and no death check

So a script can zero HEALTH *after* step 2 has already decided the target lived.
Pre-fix that produced a 0-HP NPC that kept `ai_state == Fighting` and kept
attacking; it only died on the attacker's next shot. Playtest 2026-09-19,
`MessHall_Guard1`, pistol auto attack 579 / effect 641.

Scripts cannot fix this themselves: `EffectContext` holds `&mut SpaceManager`
through a **synchronous** `on_apply`, so a script cannot await the wire burst.
The sweep has to live in the async caller.

## What it looks like in the logs

The giveaway is a `fire_entity_death` / `entity_dead_tag` content event with **no
matching `Target killed!`** nearby, then `Target killed!` 1–2 s later on the next
shot. Kill credit and the death transition get credited to different shots
because `handle_use_ability_with_kill_credit` and `fan_out_cone_effects` detect
deaths by comparing the HEALTH stat across the whole resolution — they *do* see
effect-driven zeroes. Only the transition was missing.

## The shape after the fix

`abilities::death::resolve_death` (in `death/mod.rs`) is the single kill path —
state mutations + `apply_death_transition` + threat drain + death anim + kill XP +
Defeat Window. Idempotent on `BSF_DEAD`, which is what allows `damage_apply` to
call it twice per hit. `kill_npc_out_of_band` is a thin NPC-only wrapper (GM
`.kill`, DoT pulse) and takes an explicit `grant_xp` so an admin kill can't mint
levels.

If you add a new HEALTH-mutating path, call `resolve_death` — do not re-derive
the mutations. `combat::mark_npc_dead` alone is NOT a death: it skips loot,
threat fanout, XP and every wire packet.

## Test-fixture recipe for a "direct damage survives, bleed kills" ability

An effect with a `FocusDamage` NVP and **no** `HealthDamage` NVP gives
`health_base_damage = 0` in the legacy path (`calculate_damage` returns early on
base 0), so all health loss comes from the script. With `FocusDamage = 80` against
an **empty** Focus pool the two-step truncation `(80*100/80) * 80 / 300` = 26
health damage. NPC at 20 HP → survives direct, dies to the bleed. See
`damage_apply/tests.rs::make_bleed_fixture`.

## Adjacent

`npc_ai::dispatch::npc_is_incapacitated` is the backstop: `npc_ai_tick` and
`npc_ai_retry_sweep` drop any NPC at HEALTH <= 0 before the `ai_state` filter, and
warn when such an NPC has no `BSF_DEAD`. Proven non-vacuous — with the fix
reverted the 0-HP NPC really does start its ability cooldown.
