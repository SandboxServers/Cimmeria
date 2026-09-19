---
name: handoff-pack-v1.2-weapons-audit
description: Read-only audit (2026-09-18) of docs/analysis/sgw-handoff-pack-v1.2/pack weapon/ammo/auto-attack data against our own seed and runtime — what's already done, what's a real gap, and the starter-world/loadout authority conflict
metadata:
  type: project
---

Full report delivered to team-lead 2026-09-18 as a teammate audit task. Key durable takeaways:

**The pack's weapon/ammo numbers are 100% provenance-matched to our own seed, not independent evidence.** `items_weapons_ammo.json` / `weapons_full_recovered_variants.json` claims (6,059 items, 1,073 weapon variants, 214 unique names, 15 families, clip sizes, ammo types, Event6/Event7 bindings) were all cross-checked by direct parse of `db/resources/Items/Seed/items.sql` and `items_event_sets.sql` — exact match on every count, every Item ID present, 100% of non-empty event bindings identical. This means "SOURCE-BACKED" in the pack for these columns = "matches our seed," which the pack itself is honest about being unvalidated legacy data (see its own `known_gaps` P0 items). Don't treat pack/seed agreement as proof of retail correctness.

**Phase 4 checklist reality check** (pack's `docs/IMPLEMENTATION_CHECKLIST.md`):
- Weapon-family/auto-attack mapping: **already live**, not a gap — see [[items_event_sets_dual_purpose]].
- 4-slot bandolier/active weapon: **already live** — `crates/entity/src/cell_entity/bandolier.rs`.
- Reload: **already live** — two-phase draw-window state machine, `crates/services/src/cell/cell_methods/player/world/reload.rs`.
- Ammo-*type* selection (which physical ammo is loaded): **already live** — `requestAmmoChange` handler, `crates/services/src/cell/cell_methods/inventory/bandolier/ammo_change.rs`.
- Ammo-*mode* toggle **abilities** (Hollow Point 715, Armor Piercing 719, Incendiary 723, 14 dart-type toggles): **real gap.** Ability/effect rows exist in the seed but no effect script implements the damage/penetration modifier — `crates/services/src/cell/effects/registry.rs` has no arm for them, and effect 747's own `script_name` column is NULL even in the raw seed (the original 2009 client never wired a script here either). This is combat-systems-advisor territory once someone builds it (it's a damage-modifier effect, not an inventory op).
- TechComp→damage scaling: **not started anywhere.** `tech_comp` only appears in vendor recharge pricing and the Livewire minigame; zero hits in `cell/combat/` or `cell/effects/`. Also combat-systems-advisor territory; item data already carries the `tech_comp` column so no schema work is needed, just the damage-formula hookup.

**Starter-loadout/starting-world conflict (needs a human decision, not an agent fix):**
1. Pack defers starter items/abilities as `null`/"unresolved" in `starter_loadouts.json`, calling `char_creation_abilities` "not authoritative enough" — but doesn't account for `resources.char_creation_choices` + `char_creation_visgroups`, which our server **already uses live** to grant starter items (including weapons/armor) via `crates/services/src/base/character_create.rs:196-489`. This mechanism is shipping today; the pack's caution is about a narrower table than what's actually live.
2. Pack states as "USER-CONFIRMED / PROJECT TARGET": SGU Human→Earth SGC, Free Jaffa→Dakara, Asgard→Pertho, explicitly rejecting any CellBlock/SGC_W1 placeholder start. **Every row in our current `db/resources/Archetypes/Seed/char_creation.sql` has `starting_world = 'Castle_CellBlock'`** for every archetype — i.e. the live seed routes all new characters through the CellBlock tutorial. Given the active Castle rebuild campaign (Castle 701-708 merged per other session memory), this is a genuine unresolved conflict between the pack's target state and current project direction. Flagged, not resolved — needs the user's call before Phase 5 work starts.

**No propId/wire-format errors found in the pack** — it never touches propIds, AccessLevel, or AmmoTypeId, so no collision with [[db_schema_trainer]]-adjacent wire-format hazards this agent normally watches for.

Related: [[items_event_sets_dual_purpose]] (the corrected liveness finding this audit produced), [[project_pr520_bandolier_ammo_fix]] (the ammo-persistence code this audit re-verified is still correct), [[vendor_trainer_seed_gap]] (same "plumbing done, content/formula gap remains" shape as the ammo-toggle/TechComp findings here).
