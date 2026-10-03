# AM-11d Worknotes

> Type: reference. Audience: the ammo coordinator and reviewers.
> Companions: [README.md](../README.md), [AM-11c worknotes](AM-11c.md), [ADR § 31](../../../architecture/abilities-and-effects-decisions-23-33.md#31-special-ammo-modifies-the-shot-directly-from-resourcesammo_modifiers-ammo-campaign-am-04-d-am07).

## Contract

- **Packet:** AM-11d, ally targeting for the support darts (Stim, Antidote, Coagulant, Adrenaline). Follows AM-11c, which shipped them while they could only land on a hostile target, so a Stim dart healed the enemy you shot.
- **Decision in force:** @Cadacious, 2026-09-28: support darts must be usable on allies.
- **Base:** the Wave 2 integration branch `ammo/wave2-integration` (#1063), which carries AM-11c unchanged.

## Step 0: can the client aim a shot at a friend?

**Verdict: yes as far as the static evidence goes, so the path is built and the first UAT step checks it live.**

1. **Client Lua.** `Content/UI/Core/Ability/Ability.lua` calls `useAbility(trainableInfo.id, Unit.Target)` with no hostility check. The action bar (`ActionButtons.lua`) calls the native `useAction(actionId, false)`. `AutoAttack.lua` only toggles `setAutoAttack`. No Lua in `Content/UI/Core` gates an ability press on `unitHostilityToPlayer`; that function only colours unit frames (`UnitFrames.lua`).
2. **Native emit chain** (headless Ghidra, 2026-09-28). The Lua thunk `0x00aa2910` checks only that its arguments are numbers, then calls `FUN_00ad78e0`. That resolves the local player and the target through the entity map (`FUN_00dd0de0`) and RTTI-casts it to `GameBeing`. Both branches of that cast call the same `FUN_00d2afc0`, which calls `FUN_00d2ae40`. `FUN_00d2ae40` branches only on target type 3 (ground reticle), and otherwise builds `Event_NetOut_UseAbility` with `AbilityID` and `TargetID` and emits it. Nothing on the path reads faction, aggression level or hostility. This matches [ability-resolution-pipeline.md](../../../reverse-engineering/findings/ability-resolution-pipeline.md) item 6: "the server's `useAbility` check is the only gate".
3. **Right-click** sends `interact` (`FUN_00e84b20`), not a shot. It refuses the player's own entity, and its pick filter `FUN_00e85e80` only checks that the actor is a BigWorld entity, not its team ([right-click-routing-on-corpse.md](../../../reverse-engineering/findings/right-click-routing-on-corpse.md)). The server's `interact` handler turns a click into an attack only for an alive hostile NPC, so a right-click on an ally never reaches this path. The ally path is the action-bar press of the weapon ability.
4. **Not proven:** whether the client lets you *select* yourself (`Unit.Target` = your own id) and what id it sends with no target. [client-wire-emit-suppression.md](../../../reverse-engineering/findings/client-wire-emit-suppression.md) is about the in-flight queue and the arg-count gate, not hostility.

## What shipped

| Piece | Where |
|---|---|
| `beneficial boolean NOT NULL DEFAULT false` | `db/resources/Abilities/Tables/ammo_modifiers.sql` |
| `true` on the four support rows | `db/resources/Abilities/Seed/ammo_modifiers_dart_support.sql` |
| `AmmoModifier::beneficial` and its loader column | `crates/cell-catalog/src/cell/spawner/ammo_catalog.rs` |
| The support shot: `beneficial_shot`, `classify`, `is_support_ally`, `refuse`, `fire_support` | `crates/cell-combat/src/cell/abilities/use_ability/support_shot.rs` (new) |
| Launch gate: admit an ally or self, refuse a hostile with feedback, clear auto-cycle on that refusal, never arm it for an ally shot | `use_ability/handle.rs` |
| Fire: a support shot goes to `fire_support` instead of the damage pipeline, channel cancel and cone fan-out | `use_ability/fire.rs` |
| Warmup re-check admits a support shot's ally | `use_ability/warmup/tick.rs` |
| Belt: `apply_damage_to_target` never runs a beneficial row's on-hit effect | `damage_apply/mod.rs` |
| Docs | ADR § 31 (the `beneficial` column row and the AM-11d decision replace AM-11c's "follow-up" paragraph), `docs/gameplay/combat-system.md`, the CAT-C-03 audit finding |

## Rules

A support shot is a player's weapon shot (`required_ammo > 0`) with `ammo.finite_special` on and a `beneficial` row loaded in the active slot. Its target is:

| Target | Launch | Fire |
|---|---|---|
| The shooter, or another player in the same space whom `player_may_attack` does not admit | admitted; range, line of sight, ammo and cooldown as usual | the on-hit effect runs on the target; `onStatUpdate` to the target and its witnesses; nothing else |
| A hostile NPC or an engaged duel opponent | refused before the cooldown and the dart are charged; feedback line; an armed auto-cycle loop is cleared | refused again (the warmup or an ammo change let it through), with the feedback line; the dart was already spent |
| A vendor, a friendly NPC, an ally's pet | the #444 refusal, unchanged | `not_an_ally` refusal (only reachable through a warmup) |

A support shot at an ally has no QR roll, no damage, no `onEffectResults`, no threat on anyone, no `BSF_InCombat`, no duel or PvP state, and never arms auto-cycle. Non-beneficial ammo is untouched.

## Changes from the brief

1. **An ally is a player.** The brief says "anyone for whom `player_may_attack` is false and who isn't hostile". Read literally that admits vendors and quest NPCs. This packet admits only players (and the shooter). A friendly NPC or an ally's pet stays refused by #444. Healing a friendly NPC or a pet is a separate owner decision.
2. **No `onEffectResults` on an ally shot.** The heal shows as the ally's bars moving (`onStatUpdate` to the ally and their witnesses). Sending an effect result from one player to another outside a duel has not been tested against the client, and the damage-number UI would show a zero hit.
3. **Self means the shooter's own entity id.** A `useAbility` with target 0 keeps its old meaning: the cooldown and the dart are charged and nothing resolves. What the client sends with no target selected is not verified (Step 0, point 4).
4. **The hostile refusal clears auto-cycle.** Otherwise an armed loop would repeat the refusal line every cooldown.
5. **Telemetry names are module constants, not `ammo_telemetry` entries.** The brief asks for new rows to go through the coordinator. Rows to add to the work-packets catalog and `cimmeria_entity::ammo_telemetry`:

| Event | Level | Packet | Fields beyond the correlators |
|---|---|---|---|
| `ammo_support_applied` | debug | AM-11d | `decision_outcome = applied`, `target_entity_id`, `target_player_id`, `self_target`, `ability_id`, `on_hit_effect_id`, `target_health_before` / `after`, `target_focus_before` / `after` |
| `ammo_support_refused` | debug | AM-11d | `decision_outcome = refused`, `target_entity_id`, `target_player_id`, `ability_id`, `stage` (`launch`, `fire`), `reason` (`hostile_target`, `target_gone`, `not_an_ally`) |
| `ammo_support_feedback_send_failed` | warn | AM-11d | `reason = cell_to_base_closed` |

The refusal is DEBUG because any player can aim at a hostile at will (negative-logging convention).

## Tests

| Test | Type | Guards | Failed under |
|---|---|---|---|
| `use_ability::tests::support_shot::a_stim_shot_on_an_ally_heals_them` | unit (whole handler) | the ally heal, no damage, dart and cooldown charged | M1, M3 |
| `...::a_stim_shot_on_self_heals_the_shooter` | unit | a self shot | M1, M3 |
| `...::a_stim_shot_on_a_hostile_does_nothing_and_sends_feedback` | unit + byte-exact feedback | no heal, no damage, no dart, no cooldown, no threat, the exact `CHAN_FEEDBACK` payload | M2 |
| `...::a_stim_shot_on_a_duel_opponent_is_refused` | unit | a duel opponent is hostile | M2 |
| `...::a_default_dart_on_an_ally_is_still_refused` | unit (regression) | non-beneficial ammo keeps the #444 rule at an ally and at self, with no support feedback | regression pin: no code of this packet to revert |
| `...::a_healing_row_not_marked_beneficial_cannot_target_an_ally` | unit | the column, not the script name, drives targeting | regression pin |
| `...::a_support_shot_starts_no_threat_combat_or_pvp` | unit | no threat on a mob fighting the ally, no `BSF_InCombat`, no duel, no `onEffectResults` / `onEntityProperty` / `onStateFieldUpdate` | M1, M3 |
| `...::a_warmed_up_support_shot_lands_on_the_ally` | unit | the warmup re-check admits the ally | M1, M3, M4 |
| `...::a_support_shot_that_turns_hostile_at_fire_is_refused` | unit | ammo swapped to Stim mid-warmup at a hostile: no heal, feedback | M3 |
| `...::support_shots_log_applied_and_refused_rows` | negative-log / telemetry (type 12) | both rows at DEBUG with `account_id`, `player_id`, target, `ammo_type`, `item_id`, `decision_outcome`, before/after Focus, `reason`, `stage` | pin |
| `damage_apply::ammo_support_tests::a_beneficial_row_never_heals_through_the_damage_pipeline` | unit | the belt: a beneficial row in `apply_damage_to_target` heals no hostile; control with the flag off does | M5 |
| `cimmeria-cell-combat` integration `ammo_dart_support` (both tests rewritten for AM-11d) | smoke (public API only) | an ally is healed and default darts at an ally are refused; a hostile NPC is never healed and no dart is spent | not run under the mutations (the lib suite failed first) |
| `ammo_dart_support::tests::live_db_dart_support_seed_rows` (extended) | live-DB seed guard | the four rows load with `beneficial = true`, and they are the only beneficial rows | M6 |

**Revert proof** (each mutation applied alone, confirmed applied, then restored with `git checkout HEAD --`):

- M1, launch admission forced off (`support_ally = false`): 4 failed (ally, self, no-threat, warmup).
- M2, launch hostile refusal removed: 2 failed (hostile, duel opponent).
- M3, the fire intercept disabled: 5 failed (ally, self, no-threat, warmup, turns-hostile).
- M4, warmup re-check reverted to the #444 rule: 1 failed (warmup).
- M5, the `damage_apply` belt removed: 1 failed (belt).
- M6, Stim seeded `beneficial = false`, run through `tools/build-lane/live-db-test.sh` on the worktree's database: the seed guard failed.

All pass with everything restored: `cimmeria-cell-combat`, `cimmeria-cell-world` and `cimmeria-cell-catalog` suites, clippy `--all-targets -D warnings` on the three crates, and the live-DB guard.

## Known limits

- A support shot needs `ammo.finite_special` on, like every ammo modifier. With the flag off, support darts fire as plain darts and the #444 rule applies.
- A heal to a player who is fighting a mob gives the shooter no threat. This is the brief's rule, not a 2009 behaviour anyone has evidence for.
- `ammo_modifiers_mults_positive_chk` still forces `damage_mult = 0.0001`. The support path no longer reads `damage_mult`, so the value only matters on the belt path, where it still rounds to 0.

## UAT

Needs `ammo.finite_special` on, two characters in one space, and a dart weapon whose `ammo_types` include the support types. Load Stim darts.

1. **Client check (Step 0).** Target the other character and press the dart weapon's ability on the action bar. In SigNoz, look for `combat.use_ability` with the ally's `target_id`, then `ammo_support_applied` on target `ammo`. If no `useAbility` arrives at all, the client does not emit at a friend: stop and report it, because the rest of this list cannot pass.
2. Shoot the ally. Their Focus rises by 10% on both clients, no damage number appears, one dart is spent, and neither player enters combat.
3. Target yourself (click your own portrait) and shoot. Your Focus rises by 10%. If the client will not target yourself, note it: the server path is tested, the client reachability is not.
4. Shoot a hostile NPC. Nothing happens to it, no dart is spent, and the chat shows "Support rounds only affect allies." on the first press. SigNoz: `ammo_support_refused reason=hostile_target stage=launch`.
5. Load default darts and shoot the ally. The shot is refused as before (#444) with no feedback line.
6. Duel the other character, load Stim, shoot them. Refused with the feedback line.
7. With a mob fighting the ally, heal the ally. The mob does not turn on the shooter.
