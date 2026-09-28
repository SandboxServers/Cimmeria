---
name: ammo-system-am01
description: Ammo campaign AM-01 RE findings (2026-09-28) — no reserve in client schema, live container-cache source for getAmmoTypes, toggle abilities independent of requestAmmoChange
metadata:
  type: project
---

Full write-up: `docs/reverse-engineering/findings/ammo-system.md`. Key facts worth remembering
beyond that doc:

- **`SGWPlayer+0x8c → *+0x24` is a recurring shared live map.** Four independently-decompiled
  functions hit this exact offset chain: `FUN_00ad8ad0` (active-slot cache, documented in
  `client-wire-emit-suppression.md`), `FUN_00add4a0` (`getAmmoTypes`), `FUN_00ad8ee0`
  (`requestAmmoChange`), `FUN_00ad8f10` (`getCurrentAmmoType`). Whenever a new native function is
  found dereferencing this same chain, it's almost certainly touching the bandolier/container
  cache, not cooked data. Worth checking against this offset before re-deriving from scratch.
- **`knownAmmoTypes`** (`SGWInventoryManager.def`, `ARRAY<INT32>`, `CELL_PRIVATE`) is a
  discovery/unlock flag list, NOT a quantity reserve — already correctly characterized in
  `docs/gameplay/inventory-system.md:65` ("Discovered ammo types"). Do not mistake this for the
  reserve D-AM01 asks about; the client schema has no reserve field at all.
- **`CONDITION_FEEDBACK_AmmoCount{Not}Equal/GreaterThan/LessThan`** (`enumerations.xml:1266-1271`)
  are ability effect-condition comparators against the CLIP count (comment cites ability ids
  1020, 1022), not a bag/reserve count. Don't conflate with a reserve mechanic.
- **Ability 715 (Hollow Point) has an empty `effect_ids='{}'`** in `db/resources/Abilities/Seed/abilities.sql:4730`
  — its damage/penetration modifier needs reconstruction from the description text alone. Ability
  719 (Armor Piercing, `abilities.sql:4745`) has `effect_ids='{747}'` and can serve as the template.
- **Toggle abilities 715/719 are architecturally independent of `requestAmmoChange`.** No
  decompiled path shows the client auto-invoking `UseAbility(715/719)` on an ammo-type pick, and
  `GENERICPROPERTY_AmmoTypeId` (propId 3, per `weapon-ammo-pipeline.md`) only drives the UI icon.
  This is a genuine design decision for the ammo campaign, not a recovered behavior.
- **`GiveAmmo` has zero `.def` entries anywhere in `entities/defs/`** despite being a registered
  CME NetOut event (`Event_NetOut_GiveAmmo`, string `019b3794`) and a working slash command
  (`Event_SlashCmd_GiveAmmo`, id `0x69`). The wire byte layout (ammoId/quantity arg types) was
  never recovered this session — `FUN_00593490` (the vfunc_2 body) is generic CME-emit boilerplate
  that doesn't itself read the args; the field-binding call site needs a further trace.
- **Cooked-data element-key push has no static id-range check** (per `cooked-data-pipeline.md` +
  `cooked-dialog-override-crash.md`: element key is a `long` end-to-end, ZIP entry name built by
  plain decimal-digit streaming). PR #405 proved this in production for MODIFIED existing item ids
  (2893, 4735). Brand-new ids are architecturally supported by the same mechanism but not yet
  empirically proven — flag this distinction if anyone cites #405 as full precedent for new ids.
- **Headless Ghidra workflow confirmed working again 2026-09-28** with the `ND:<name>` token
  (decompile by exact function name, no address needed) — very convenient for chasing `FUN_xxxxx`
  call sites found in the existing decompiled dumps under `docs/reverse-engineering/decompiled/`.
  One batch of 4 `ND:` lookups (`FUN_00593490`, `FUN_00add4a0`, `FUN_00ad8ee0`, `FUN_00ad8f10`)
  completed in a single ~1-2 min headless run.
- **Line-ending correction**: `docs/reverse-engineering/findings/*.md` files are stored **LF**, not
  CRLF, despite the general `reference_docs_crlf_line_endings.md` memory claiming `docs/**/*.md` is
  CRLF. Verified against `weapon-ammo-pipeline.md` in git HEAD (zero `\r` bytes). That CRLF claim
  does not hold repo-wide — check the specific sibling file in the same directory before assuming.
