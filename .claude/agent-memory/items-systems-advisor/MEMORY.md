# Items Systems Advisor — Memory Index

- [user_profile.md](user_profile.md) — Steve is emulator lead; deep Rust + RE background; works on Windows with Ghidra+x64dbg MCP
- [feedback_terse_responses.md](feedback_terse_responses.md) — Skip trailing summaries; user reads the diff/output directly
- [trainer_implementation_status.md](trainer_implementation_status.md) — Ability trainer: onTrainerOpen live at cell/interactions/trainer.rs; crates/game stub already RETIRED (not just dead) — supersedes stale 2026-05-27 note
- [wire_format_trainer.md](wire_format_trainer.md) — onTrainerOpen wire layout, TrainerAbility FIXED_DICT encoding, method indices
- [training_points_currency.md](training_points_currency.md) — Trainer uses training_points (integer), NOT Naquadah; TrainerResult::NotEnoughMoney is a misnomer
- [db_schema_trainer.md](db_schema_trainer.md) — trainer_abilities, trainer_ability_lists, archetype_ability_tree schema and joins
- [ghidra_trainer_addresses.md](ghidra_trainer_addresses.md) — Ghidra RE addresses for trainer-related functions in SGW.exe
- [project_crafting_system.md](project_crafting_system.md) — Crafting system deep-dive: wire formats, DB schema, item flags, expertise formulas, implementation phases (issue #53)
- [trade_system_wire_formats.md](trade_system_wire_formats.md) — Trade system: verified wire formats, enum values, state machine, TRAPS (INT32 result, cancel=Completed, QA client skips tradeRequest), issue #54
- [project_pr520_bandolier_ammo_fix.md](project_pr520_bandolier_ammo_fix.md) — PR #520 SHIP-WITH-NITS: instance_id guard correct, doc comment wrongly denies declared PK on sgw_inventory
- [item_use_trigger_mechanism.md](item_use_trigger_mechanism.md) — items_event_sets event_id=5 is LEGACY for UseInventoryItem; real wiring is content_triggers(item_use,<item_id>) -> OnItemUse -> fire_item_use — CORRECTED, see next entry
- [items_event_sets_dual_purpose.md](items_event_sets_dual_purpose.md) — items_event_sets event_id 6/7 (melee/ranged) IS live for weapon auto-attack binding; event 5 live since 2026-09-28 (next entry)
- [native_consumables_and_stat_buffs.md](native_consumables_and_stat_buffs.md) — event 5 native consumables: 597 filler gate, consume-first round trip, pulse_count=1 never registers, stat-keyed stim stacking, unwired stealth/energy/disguise
- [vendor_trainer_seed_gap.md](vendor_trainer_seed_gap.md) — Vendor/trainer Rust plumbing complete; content is ~empty system-wide (only 1 test entity_template wired); no bank/storage service exists at all
- [harset_source_material.md](harset_source_material.md) — Surviving Harset legacy sources: GivingTheWallsEars.script (mission 742), Harset.py/space scripts, monolithic resources.sql dialog text
- [project_handoff_pack_v1.2_weapons_audit.md](project_handoff_pack_v1.2_weapons_audit.md) — 2026-09-18 audit of sgw-handoff-pack-v1.2 weapon/ammo data vs our seed+runtime: 100% provenance match, Phase 4 mostly done, ammo-mode toggles + TechComp scaling are real gaps, starter-world conflict flagged
- [project_craft_verb_named_instance_vs_design_consume.md](project_craft_verb_named_instance_vs_design_consume.md) — craft verb review 2026-09-27: holding a craft to its named instances spuriously failed a queued job whose named stack was drained (fixed: craft passes no named_items); INV_MAIN=1/INV_CRAFTING=15 carry no equipped items
