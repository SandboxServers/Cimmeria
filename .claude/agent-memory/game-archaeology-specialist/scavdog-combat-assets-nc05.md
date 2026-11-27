---
name: scavdog-combat-assets-nc05
description: ScavDog client assets (NC-05, 2026-11-27) - one attack anim, no AnimNotifies, mob_scav.fev unreferenced in cooked data, cooked abilities have no event-set field, MeleeDistance 0
metadata:
  type: project
---

Full finding: `docs/reverse-engineering/findings/scavdog-combat-assets.md`.

- `MOB_ScavDog.upk`: one attack sequence `sAttack1` (export 16, 1.133 s); no sequence has `Notifies`, no AnimNotify/SoundCue import, `BodySounds` empty. Kismet hooks (KD_MOBs) are death + 5 hit kinds only.
- `mob_scav.fev` = 5 events (`attack` with 5 wave variants, breathe, death, hit, shake). Only the FEV/FSB and `GameplayEngine.ini` name `mob_scav`; every `.upk` hit is the substring of `MOB_ScavDog`. Who plays mob FEV events is open. audio-voice-inventory §7 "notifies trigger these" is wrong for ScavDog.
- Cooked `COOKED_ABILITY` XML has **no event-set attribute**, so any ability → event_set link (e.g. 1433/983 → 300) is seed-only. The seed gives both 1433 and 983 set 300 (the 300 on 1433/983 is a Cimmeria Tier-3 reconstruction from PR #822, not legacy data).
- `BodyStances[0].MeleeDistance = 0`; abilities 1433/983 have Min/MaxRange 0. No client reach value.

**Why:** the npc-combat-reference campaign gates NC-13 (ScavDog gets 1433) on this.

**How to apply:**
- Cooked `.pak` files are ZIP-of-XML: read them with Python `zipfile` (entries are `_<id>`).
- Id collisions across cooked categories (KismetSetEvent `_983`) are unrelated namespaces.
- `upk_info --properties N` leaves struct arrays as raw bytes; a small tagged-property decoder is needed. In this licensee build a `Matrix` StructProperty has 4 extra bytes before its 64 floats.
- Related: [[being-eye-heights]], [[client-engine-sinks-seams]].
