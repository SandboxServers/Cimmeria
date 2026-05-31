# Items Systems Advisor — Memory Index

- [user_profile.md](user_profile.md) — Steve is emulator lead; deep Rust + RE background; works on Windows with Ghidra+x64dbg MCP
- [feedback_terse_responses.md](feedback_terse_responses.md) — Skip trailing summaries; user reads the diff/output directly
- [trainer_implementation_status.md](trainer_implementation_status.md) — Ability trainer: onTrainerOpen + trainAbility fully implemented in services, dead stub in crates/game
- [wire_format_trainer.md](wire_format_trainer.md) — onTrainerOpen wire layout, TrainerAbility FIXED_DICT encoding, method indices
- [training_points_currency.md](training_points_currency.md) — Trainer uses training_points (integer), NOT Naquadah; TrainerResult::NotEnoughMoney is a misnomer
- [db_schema_trainer.md](db_schema_trainer.md) — trainer_abilities, trainer_ability_lists, archetype_ability_tree schema and joins
- [ghidra_trainer_addresses.md](ghidra_trainer_addresses.md) — Ghidra RE addresses for trainer-related functions in SGW.exe
- [project_crafting_system.md](project_crafting_system.md) — Crafting system deep-dive: wire formats, DB schema, item flags, expertise formulas, implementation phases (issue #53)
