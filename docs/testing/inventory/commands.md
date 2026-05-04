# Tests — `commands`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-04  
> **Total tests**: 29  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Server command dispatch framework — parser, registry, and execution context for player and GM commands.

## All tests (29)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [parse_command_with_slash](../../../crates/commands/src/parser.rs#L98) | unit | Parser | 2026-03-03 | Asserts equality on `name` |  |
| [parse_command_without_slash](../../../crates/commands/src/parser.rs#L105) | unit | Parser | 2026-03-03 | Asserts equality on `name` |  |
| [parse_command_no_args](../../../crates/commands/src/parser.rs#L112) | unit | Parser | 2026-03-03 | Asserts equality on `name` |  |
| [parse_command_empty_input](../../../crates/commands/src/parser.rs#L119) | unit | Parser | 2026-03-03 | Asserts on `parse_command("").is_none()` |  |
| [parse_command_only_slash](../../../crates/commands/src/parser.rs#L125) | unit | Parser | 2026-03-03 | Asserts on `parse_command("/").is_none()` |  |
| [parse_command_extra_whitespace](../../../crates/commands/src/parser.rs#L130) | unit | Parser | 2026-03-03 | Asserts equality on `name` |  |
| [parse_entity_id_plain](../../../crates/commands/src/parser.rs#L137) | unit | Parser | 2026-03-03 | Asserts equality on `id` |  |
| [parse_entity_id_hash_prefix](../../../crates/commands/src/parser.rs#L143) | unit | Parser | 2026-03-03 | Asserts equality on `id` |  |
| [parse_entity_id_negative](../../../crates/commands/src/parser.rs#L149) | unit | Parser | 2026-03-03 | Asserts equality on `id` |  |
| [parse_entity_id_invalid](../../../crates/commands/src/parser.rs#L156) | unit | Parser | 2026-03-03 | Asserts on `parse_entity_id("abc").is_none()` |  |
| [parse_vector3_valid](../../../crates/commands/src/parser.rs#L163) | unit | Parser | 2026-03-03 | Asserts equality on `v.x` |  |
| [parse_vector3_integers](../../../crates/commands/src/parser.rs#L171) | unit | Parser | 2026-03-03 | Asserts equality on `v.x` |  |
| [parse_vector3_too_few_args](../../../crates/commands/src/parser.rs#L179) | unit | Parser | 2026-03-03 | Asserts on `parse_vector3(&["1.0", "2.0"]).is_none()` |  |
| [parse_vector3_non_numeric](../../../crates/commands/src/parser.rs#L186) | unit | Parser | 2026-03-03 | Asserts on `parse_vector3(&["1.0", "abc", "3.0"]).is_none()` |  |
| [parse_vector3_extra_args_ignored](../../../crates/commands/src/parser.rs#L191) | unit | Parser | 2026-03-03 | Asserts equality on `v.x` |  |
| [access_level_ordering](../../../crates/commands/src/permissions.rs#L58) | unit | Permissions | 2026-03-03 | Asserts on `AccessLevel::Developer > AccessLevel::Admin` |  |
| [can_execute_same_level](../../../crates/commands/src/permissions.rs#L66) | unit | Permissions | 2026-03-03 | Asserts on `AccessLevel::GameMaster.can_execute(AccessLevel::GameMaster)` |  |
| [can_execute_higher_level](../../../crates/commands/src/permissions.rs#L71) | unit | Permissions | 2026-03-03 | Asserts on `AccessLevel::Developer.can_execute(AccessLevel::Player)` |  |
| [cannot_execute_insufficient_level](../../../crates/commands/src/permissions.rs#L77) | unit | Permissions | 2026-03-03 | Asserts on `!AccessLevel::Player.can_execute(AccessLevel::Moderator)` |  |
| [register_and_execute](../../../crates/commands/src/registry.rs#L213) | unit | Registry | 2026-03-03 | Asserts equality on `result` |  |
| [execute_passes_args](../../../crates/commands/src/registry.rs#L229) | unit | Registry | 2026-03-03 | Asserts equality on `result` |  |
| [execute_unknown_command](../../../crates/commands/src/registry.rs#L245) | unit | Registry | 2026-03-03 | Asserts on `msg.contains("Unknown command")` |  |
| [execute_empty_input](../../../crates/commands/src/registry.rs#L256) | unit | Registry | 2026-03-03 | Asserts on `msg.contains("Empty")` |  |
| [execute_permission_denied](../../../crates/commands/src/registry.rs#L267) | unit | Registry | 2026-03-03 | Asserts on `msg.contains("Permission denied")` |  |
| [execute_permission_granted_higher_level](../../../crates/commands/src/registry.rs#L286) | unit | Registry | 2026-03-03 | Asserts equality on `result` |  |
| [command_name_case_insensitive](../../../crates/commands/src/registry.rs#L302) | unit | Registry | 2026-03-03 | Asserts equality on `result` |  |
| [list_commands_filters_by_access](../../../crates/commands/src/registry.rs#L318) | unit | Registry | 2026-03-03 | List commands filters by access |  |
| [list_commands_sorted_alphabetically](../../../crates/commands/src/registry.rs#L354) | unit | Registry | 2026-03-03 | Asserts equality on `names` |  |
| [console_context_no_entity_id](../../../crates/commands/src/registry.rs#L366) | unit | Registry | 2026-03-03 | Console context no entity id |  |
