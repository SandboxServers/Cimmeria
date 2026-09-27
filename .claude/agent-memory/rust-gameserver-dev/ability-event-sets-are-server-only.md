---
name: ability-event-sets-are-server-only
description: Setting abilities.event_set_id needs no cooked-data push; the client resolves onSequence ids from its own KismetSeqEvent PAK. Also, most NPC mob kits deal zero damage.
metadata:
  type: reference
---

- `COOKED_ABILITY` entries in `data/cache/CookedDataAbilities.pak` carry no event-set attribute (0 of 1,886). The server maps `(event_set_id, event_id) -> sequence_id` (`cell-catalog spawner/abilities.rs`) and sends only the sequence id. So wiring an existing event set onto an ability is a seed-only change, as long as the sequence ids are already in the shipped `CookedDataKismetSeqEvent.pak`. Check with Python `zipfile` on the `_<id>` entries. A brand-new sequence id needs a `sequence_overrides.rs` entry (#755).
- Kismet event ids: 1000 is Ability_Begin, 1001 Ability_End, 1002 Ability_Interrupt, 2000 Effect_Init (`enumerations.xml:774`). A "... target" event set usually holds only Effect_Init, which is effect-level, so it cannot go on an ability row.
- Damage comes only from effect NVPs (`HealthDamage`/`FocusDamage`) or an effect script. Many mob "special" abilities have neither: the Straegis kit 1156/2847/1240 deals 0. Check `effect_nvps` before choosing an NPC or pet kit. Also check the abilities' `is_ranged` flag and cooldown, because the lowest-id off-cooldown fallback fires on every AI tick. 1240 Explode has cooldown 0 and death VFX.
- Related: [[npc-range-gate-and-weapon-range-columns]].
