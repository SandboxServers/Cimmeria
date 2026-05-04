# Tests — `tools/ContentEditor`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 12  
> **CI-gated**: no  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Content editor desktop tooling (non-CI).

## All tests (12)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [event_loaded_to_accept_mission](../../../tools/ContentEditor/src/commands/convert.rs#L872) | unit | Commands / Convert | 2026-03-11 | Event loaded to accept mission |  |
| [region_enter_to_system_message](../../../tools/ContentEditor/src/commands/convert.rs#L907) | unit | Commands / Convert | 2026-03-11 | Region enter to system message |  |
| [interact_event_with_get_entity](../../../tools/ContentEditor/src/commands/convert.rs#L940) | unit | Commands / Convert | 2026-03-11 | Interact event with get entity |  |
| [disabled_nodes_skipped](../../../tools/ContentEditor/src/commands/convert.rs#L977) | unit | Commands / Convert | 2026-03-11 | Disabled nodes skipped |  |
| [launch_ability_action](../../../tools/ContentEditor/src/commands/convert.rs#L995) | unit | Commands / Convert | 2026-03-11 | Launch ability action |  |
| [teleport_event_trigger](../../../tools/ContentEditor/src/commands/convert.rs#L1015) | unit | Commands / Convert | 2026-03-11 | Teleport event trigger |  |
| [empty_script_produces_no_chains](../../../tools/ContentEditor/src/commands/convert.rs#L1047) | unit | Commands / Convert | 2026-03-11 | Asserts on `result.chains.is_empty()` |  |
| [add_dialog_action](../../../tools/ContentEditor/src/commands/convert.rs#L1055) | unit | Commands / Convert | 2026-03-15 | Add dialog action |  |
| [generate_threat_action](../../../tools/ContentEditor/src/commands/convert.rs#L1082) | unit | Commands / Convert | 2026-03-15 | Generate threat action |  |
| [cmp_int_archetype_branch](../../../tools/ContentEditor/src/commands/convert.rs#L1102) | unit | Commands / Convert | 2026-03-15 | Cmp int archetype branch |  |
| [var_nodes_dont_forward](../../../tools/ContentEditor/src/commands/convert.rs#L1156) | unit | Commands / Convert | 2026-03-15 | Var nodes dont forward |  |
| [chain_ids_increment](../../../tools/ContentEditor/src/commands/convert.rs#L1178) | unit | Commands / Convert | 2026-03-11 | Chain ids increment |  |
