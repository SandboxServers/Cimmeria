# Ammo: Session Resume

> Type: how-to. Audience: the next coordinator session and the owner running the UAT.
> Updated: 2026-09-28 (AM-12 close-out). Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md), [AM-12 worknote](../worknotes/AM-12.md), [unified UAT guide § Special ammo](../../../guides/unified-uat.md#special-ammo).

## State: campaign complete, awaiting the owner's UAT

Every packet is merged on `main`, and AM-12 turned `ammo.finite_special` on by default (D-AM11). The per-packet table with PR numbers is [README.md § Packet status](../README.md#packet-status).

**Rollback lever.** Set `CIMMERIA_AMMO_FINITE_SPECIAL=0` in the server's environment and restart it. Special reloads go back to free refills, every shot fires unmodified, and support darts fire as plain darts. It does **not** withdraw the pushed ammo item definitions (9000-9014), the loot rows or the GM commands: the items stay in bags with their icons and names, as ordinary inert stacks. To withdraw the definitions too, empty `ITEM_ADDITIONS` in `crates/resources/src/base/item_overrides/mod.rs` and redeploy; the next cooked-data resync evicts them (AM-07 worknote, "Deviation: no toggle").

## Resuming

1. **Run the UAT below**, in order. Steps AMMO-01 to AMMO-03 are the three risks of shipping the flag on; if one fails, pull the lever above first and then triage.
2. Record results in the [unified UAT guide's template](../../../guides/unified-uat.md#recording-results) and tell the coordinator; each failure becomes an issue, and this ledger's [Known issues and follow-ups](../README.md#known-issues-and-follow-ups) gets a row.
3. The follow-ups that need an owner decision or a packet of their own are listed in the README. None blocks the UAT.
4. Retire any campaign worktree still on disk: `bash tools/build-lane/rm-worktree.sh --merged`.

## Cross-campaign notes

- **Bank campaign.** Ammo stacks are ordinary items (container sets 1, 15, 17), so the personal vault, mail and trade already handle them. The vault (17) never counts toward a reload.
- **Loot campaign (#1031).** AM-05's rows sit on the same tables as #1031's. `live_db_castle_loot` and `castle_loot_containers` skip ammo rows by `ammo_item_types`, so a future loot change there does not need to know about ammo.
- **Enemy combat.** Penetration waits on `MITIGATION` being populated, which is the same blocker the enemy-combat handoff tracks (`MITIGATION 0/0`).

## UAT checklist

Run these on the colo (or a local server) as a GM, after a deploy that carries AM-12. At anything odd, type `.bug <what you saw>`. Each step ends with the SigNoz query that verifies it: open Logs Explorer, filter `service.name = 'cimmeria-server'` and add the filter shown, plus `player_id = <your character id>`. Every ammo row is on `scope_name = 'ammo'`, exported at DEBUG (`ammo=debug`); the on-hit effect scripts log on `scope_name = 'abilities'`.

<!-- markdownlint-disable MD029 -->
<!-- The steps keep one numbering across the sub-headings, so the step ids match AMMO-NN. -->

**Setup.** A GM character with room for about 20 items in the main bag. The debug-hub crate in the Castle Cellblock stasis room is the quickest source of everything: right-click it for five 500-round bullet stacks, ten 500-round dart stacks, an SI 3 9mm Pistol (3235), an MPX 77 SMG (3147) and a CO2 Pistol Dartgun (3584). Load a special type by opening the weapon's ammo-type picker on the weapon bar and choosing it, then reload.

### The three risks of shipping the flag on

These come first. Stop and report if any fails.

1. **AMMO-01: the new ammo items render.** Type `.giveammo hp 50`. A stack of 50 appears in your main bag with the Hollow Point ammo icon (the one the weapon bar shows for Hollow Point, not the missing-icon square), the name "Hollow Point Rounds", and a tooltip that starts "Special ammunition." Right-click the debug-hub crate and check two more of the new stacks the same way (one bullet, one dart). Log out and back in: the stacks keep their icon and name, and the client does not resync items a second time. This is the first in-client proof that a wholly new item id (9000-9014) can be pushed (AM-07's spike).
   - SigNoz: `event = 'gm_give_ammo'` INFO with `item_id = 9001`, `returned = 50`, `stack_after`. The items resync is `event = 'cooked_data.version_reply' AND category_id = 4`: `outcome = full_resync` on the first login after the deploy, `up_to_date` after the relog.
2. **AMMO-02: unknown effect ids do not crash a client.** Two clients in the same place: yours and a second character watching the same NPC. Shoot a hostile NPC (a Castle Cellblock guard, or one you `.spawn`) with **Incendiary** rounds from the pistol, then switch to the dartgun and hit it with a **Poison** dart and a **Radioactive** dart. Optional: a **Disease** and a **Tranquilizer** dart. Neither client crashes or freezes. On each client, note what the NPC's target frame and buff bar show for the effect (nothing, an icon, or a blank icon): these effects (9110, 9140-9142, 9151) are not in the client's cooked data, and their duration timer reaches every watcher (the #938 crash class).
   - SigNoz: `event = 'ammo_damage_applied'` with `on_hit_effect_id` 9110, 9140 or 9151; the timers are `scope_name = 'abilities' AND event IN ('active_effect_registered','active_effect_refreshed')` with the same `effect_id`.
   - If a client crashes: pull the rollback lever, capture the client log, and look for `onTimerUpdate` carrying the effect id. The fix is in the pulsing layer (AM-11a worknote, "UAT risk").
3. **AMMO-03: a Stim dart heals an ally and is refused at an enemy.** Two characters in the same space. Load **Stim** darts in the dartgun. The ally lowers their Focus first (take a few shots from a hostile NPC). Target the ally and press the dartgun's ability on the action bar: their Focus rises by 10 % on both clients, no damage number appears, one dart is spent, and neither of you enters combat. Then target a hostile NPC and press it: nothing happens to the NPC, no dart is spent, and chat shows "Support rounds only affect allies." on the first press.
   - SigNoz: `event = 'ammo_support_applied'` with the ally's `target_player_id`, `target_focus_before` and `target_focus_after`; then `event = 'ammo_support_refused' AND reason = 'hostile_target' AND stage = 'launch'`.
   - If no `combat.use_ability` arrives at all when you aim at the ally, the client does not emit a shot at a friend: report it, because the rest of the support-dart steps cannot pass (AM-11d, Step 0).

### Reload and the bag reserve

4. **AMMO-04: a special reload draws from the bag.** Load Hollow Point in the pistol (its default clip empties: default rounds are free). Reload. The clip fills and your Hollow Point stack drops by exactly the clip size.
   - SigNoz: `event = 'reload_draw_requested'` (cell), then `event = 'reload_draw'` (base: `requested`, `drawn`, `clip_before`, `clip_after`, `stack_before`, `stack_after`), then `event = 'reload_drawn_loaded'`. The switch is `event = 'ammo_switch_default_emptied'`.
5. **AMMO-05: a partial reload takes only what is missing.** Fire a few shots, count them, and reload. The stack drops by exactly the shots fired; the rounds already in the clip stay.
   - SigNoz: `event = 'reload_draw'` with `drawn` = the shots fired and `clip_before` = what was left.
6. **AMMO-06: a short stack loads what is there.** Delete your Armor Piercing stacks, then `.giveammo ap 5`. Load Armor Piercing and reload. The clip shows 5 and the stack is gone.
   - SigNoz: `event = 'reload_draw'` with `requested` above 5 and `drawn = 5`, `stack_after = 0`.
7. **AMMO-07: an empty stack is refused with feedback.** Fire the 5 rounds, then reload. Chat shows "You have no Armor Piercing rounds left." on the first press, the clip stays at 0, and no reload animation plays.
   - SigNoz: `event = 'reload_refused' AND reason = 'stack_empty'` (base), then `event = 'reload_refused_feedback' AND reason = 'stack_empty'` (cell).
8. **AMMO-08: switching type returns the unfired rounds.** Load Hollow Point, reload, fire a few shots, then switch to Armor Piercing (`.giveammo ap 50` first). The Hollow Point stack grows by exactly the rounds that were in the clip, the clip is empty, and a reload now draws Armor Piercing.
   - SigNoz: `event = 'ammo_switch_return_requested'`, then `event = 'ammo_switch_return'` (base: `returned`, `remainder = 0`, `stack_before`, `stack_after`).
9. **AMMO-09: a full bag keeps the rounds loaded.** Load and reload Hollow Point. Delete your other Hollow Point stacks, then fill every free slot of your main bag and your crafting bag (`/gmgiveitem 2893 1` repeatedly, for example). Switch to Armor Piercing. The switch does not happen: chat shows "Your bags are full: N Hollow Point rounds stay loaded. Make room and switch again.", and the weapon still shows Hollow Point with its rounds. Free a slot and switch again: it works.
   - SigNoz: `event = 'ammo_switch_refused' AND reason = 'bags_full'` with `remainder`; after the second switch, `event = 'ammo_switch_return'` with `remainder = 0`.
10. **AMMO-10: default ammo is unchanged.** Switch the pistol back to its default ammo, fire and reload. The reload refills for free and no bag stack changes.
    - SigNoz: no `reload_draw` row for the reload; `scope_name = 'ammo'` shows nothing for it.

### The ammo picker

11. **AMMO-11: the picker offers the special types on the widened weapons.** Open the ammo-type picker on the SI 3 9mm Pistol (Standard Pistol), the MPX 77 SMG (Standard SMG) and an SGHC 6 SMG (`/gmgiveitem 3127 1`, High Capacity SMG). Each offers Hollow Point, Armor Piercing, Incendiary, EMP and Explosive besides its default. The CO2 Pistol Dartgun offers the ten dart types. Picking each one is accepted.
    - SigNoz: an accepted pick is `scope_name = 'bandolier' AND event = 'ammo_type_change'`; a refusal would be `event = 'ammo_type_change_rejected'` with its `reason` (there should be none).
    - Note: swapping bandolier slots (F1-F4) right around a pick can look stale for a moment; that is a client-side quirk of the slot swap, not the picker (AM-01 Q5).

### Damage and on-hit effects

12. **AMMO-12: Hollow Point hits harder, Armor Piercing a little softer.** Shoot the same kind of NPC with default rounds, then Hollow Point, then Armor Piercing, and compare the damage numbers over a few shots each. Hollow Point is about 1.25 times default, Armor Piercing about 0.9 times. Penetration does nothing yet (known issue).
    - SigNoz: `event = 'ammo_damage_applied'` with `damage_mult = 1.25` (Hollow Point) or `0.9` (Armor Piercing) and `toggle_ability_id` 715 or 719.
13. **AMMO-13: Incendiary burns.** Hit an NPC once with Incendiary rounds. It takes the shot, then three more small Focus-and-Health ticks about a second apart. A second hit from you refreshes the burn rather than stacking it.
    - SigNoz: `event = 'ammo_damage_applied' AND on_hit_effect_id = 9110`, then `scope_name = 'abilities' AND event = 'ranged_energy_damage'` per tick (15 Focus, 3 Health).
14. **AMMO-14: EMP rounds split on living and mechanical targets.** Shoot a Prisoner Retrieval Unit drone in Castle, then a guard, with EMP rounds. The drone loses 5 extra Health per hit and no Focus; the guard loses 10 extra Focus and no extra Health. Nothing is interrupted (known issue).
    - SigNoz: `event = 'ammo_emp_disrupt'` with `mechanical = true` and `health_damage = 5` for the drone, `mechanical = false` and `focus_drained = 10` for the guard.
15. **AMMO-15: Explosive rounds splash.** Shoot one NPC of a group standing close together. The neighbours within about 5 m take a smaller damage number of their own; one behind a wall, one further away, you, and a friendly NPC take nothing. A splashed NPC does not splash again.
    - SigNoz: `event = 'ammo_splash'` with `splash_count`, `targets`, `los_blocked`; each splashed NPC also gets its own `ammo_damage_applied`.
16. **AMMO-16: the crowd-control darts.** With the dartgun: a **Poison** dart chips 4 Health on the hit and every 2 s for 8 s; a **Disease** dart 2 Health every 2 s for 18 s; a **Tranquilizer** dart slows the NPC to about 60 % of its speed for about 6 s (watch it chase you).
    - SigNoz: `scope_name = 'abilities' AND event = 'suppression_pulse'` (Poison, Disease); `event IN ('movement_slow_applied','movement_slow_expired')` (Tranquilizer).
17. **AMMO-17: the tech darts.** An **EMP** dart drains 50 extra Focus on the hit. A **Radioactive** dart takes 3 Health on the hit and every 2 s for 8 s.
    - SigNoz: `scope_name = 'abilities' AND event = 'ranged_energy_damage'` with `effect_id = 9150`; `event = 'radiation_pulse'` with `effect_id = 9151`, `health_before`, `health_after`.
18. **AMMO-18: the other support darts.** Two characters. **Adrenaline** on a hurt ally raises their Health by 10 %. **Antidote**: duel the other character, hit them with a **Disease** dart, end the duel at once with `.duel_end <name>`, then fire an **Antidote** dart at them within 18 s: the Disease ticks stop. (If the ticks already stopped when the duel ended, note it: nothing in the ammo code clears effects on a duel's end, so that would be a finding.) **Coagulant** on an ally changes nothing visible (nothing seeded is a Wound yet). **Nanites** fires as a plain dart (known issue). Shoot yourself with Stim (target your own portrait) and your Focus rises; if the client will not let you target yourself, note it.
    - SigNoz: `scope_name = 'abilities' AND event IN ('heal_health','heal_focus','effect_removed_by_cleanse')`; the cleanse row has `category = Disease`. `event = 'ammo_support_applied' AND self_target = true` for the self shot.

### Loot

19. **AMMO-19: the debug-hub crate.** Right-click the crate. Loot All puts in your bags: Hollow Point, Armor Piercing, Incendiary, EMP and Explosive rounds (500 each), the ten dart types (500 each), the SI 3 9mm Pistol, the MPX 77 SMG and the CO2 Pistol Dartgun. It needs about 18 free slots; with fewer, the rest stay in the crate with the usual "left in the container" line.
    - SigNoz: one `event = 'ammo_loot_dropped'` per ammo stack, with `quantity = 500` and `loot_table_id = 3`.
20. **AMMO-20: Castle NID guards drop Hollow Point.** Kill Castle NID guards and veterans and loot them. About one guard in 20 (one in 10 in the hall) drops 10-25 Hollow Point, and about one veteran in 16 drops 15-30 Hollow Point or, more rarely, 10-20 Armor Piercing. The stack lands in your bag and merges with an existing one.
    - SigNoz: `event = 'ammo_loot_dropped'` with `loot_table_id` 4, 5 or 7, `corpse_template_id`, `quantity`.
21. **AMMO-21: the Castle pre-Romney chest's Hollow Point fits its SMG.** With mission 703 active, open the Castle pre-Romney chest as a non-Jaffa character. It gives 50-75 Hollow Point and an SGHC 6 SMG (3127). Equip the SMG: its ammo picker offers Hollow Point, and a reload draws from the chest's rounds. (As a Jaffa the chest gives a Serpent Staff, which does not take bullets.)
    - SigNoz: `event = 'ammo_loot_dropped' AND loot_table_id = 8`, then `event = 'reload_draw'` with `item_id = 9001`.

### GM commands

22. **AMMO-22: `.giveammo`.** `.giveammo hollowpoint 700` gives one full stack of 500 and one of 200 (never one stack over the cap). `.giveammo emp 10` is refused as ambiguous (bullet or dart); `.giveammo bullet_emp 10` works. `.giveammo 1 10` is refused as free default ammo. Select another player and `.giveammo stim 20`: the rounds go to them. As a non-GM, `.giveammo` answers "is a GM command".
    - SigNoz: `event = 'gm_give_ammo'`: INFO granted with `returned` and `remainder`, WARN refused with `reason` (`unknown_ammo_type`, `not_special_ammo`, and so on).
23. **AMMO-23: `.infiniteammo`.** `.infiniteammo on`, then load Hollow Point, empty the clip and reload: the clip fills and the bag stack does not change, but the clip still empties as you fire. `.infiniteammo off`: the next reload draws again. The alias `.gmsetinfiniteammo` does the same.
    - SigNoz: `event = 'gm_infinite_ammo_toggled'` with `on`; no `reload_draw` for the reload while it is on.

### Rollback lever (operator, optional)

24. **AMMO-24: `CIMMERIA_AMMO_FINITE_SPECIAL=0`.** On a local server only: restart it with the variable set to `0`. Special reloads refill for free, damage numbers match default ammo, and a Stim dart at an ally is refused like any weapon shot. The ammo stacks keep their icons and names. Unset the variable and restart: everything is back.
    - SigNoz: `event = 'feature_flag'` with `on = false` at startup (`on = true` when unset); no `ammo_damage_applied` while it is off.

<!-- markdownlint-enable MD029 -->

## Known gaps (carried forward)

Everything in [README.md § Known issues and follow-ups](../README.md#known-issues-and-follow-ups), and the guard audit's exemptions in [AM-12's worknote](../worknotes/AM-12.md#telemetry-guard-audit).
