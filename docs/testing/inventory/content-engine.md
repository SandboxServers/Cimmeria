# Tests — `content-engine`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-05-05  
> **Total tests**: 64  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Data-driven content runtime: trigger/condition/action chains for missions, dialogs, sequences, and interactions. Replaces the per-script Python in the legacy server.

## All tests (64)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [trigger_chain_returns_chain_trigger_result](../../../crates/content-engine/src/actions.rs#L275) | unit | Actions | 2026-03-03 | Asserts equality on `id` |  |
| [action_serialization_roundtrip](../../../crates/content-engine/src/actions.rs#L285) | unit | Actions | 2026-03-03 | Action serialization roundtrip | smell: no_assert_or_question_mark |
| [property_op_serialization_roundtrip](../../../crates/content-engine/src/actions.rs#L293) | unit | Actions | 2026-03-03 | Property op serialization roundtrip | smell: no_assert_or_question_mark |
| [complex_action_serialization](../../../crates/content-engine/src/actions.rs#L308) | unit | Actions | 2026-03-03 | Asserts on `json.contains("health")` |  |
| [teleport_action_serialization](../../../crates/content-engine/src/actions.rs#L321) | unit | Actions | 2026-03-03 | Asserts equality on `space_id` |  |
| [grant_item_with_container](../../../crates/content-engine/src/actions.rs#L338) | unit | Actions | 2026-03-07 | Asserts on `json.contains("container_id")` |  |
| [grant_item_without_container_defaults_none](../../../crates/content-engine/src/actions.rs#L362) | unit | Actions | 2026-03-07 | Asserts equality on `container_id` |  |
| [move_waypoint_serialization](../../../crates/content-engine/src/actions.rs#L372) | unit | Actions | 2026-03-11 | Move waypoint serialization |  |
| [set_active_slot_serialization](../../../crates/content-engine/src/actions.rs#L395) | unit | Actions | 2026-03-11 | Asserts equality on `bag_id` |  |
| [launch_ability_serialization](../../../crates/content-engine/src/actions.rs#L409) | unit | Actions | 2026-03-11 | Asserts equality on `ability_id` |  |
| [launch_ability_without_entity_tag](../../../crates/content-engine/src/actions.rs#L429) | unit | Actions | 2026-03-11 | Asserts equality on `ability_id` |  |
| [accept_mission_serialization](../../../crates/content-engine/src/actions.rs#L445) | unit | Actions | 2026-03-07 | Asserts equality on `mission_id` |  |
| [new_engine_has_no_chains](../../../crates/content-engine/src/chain.rs#L295) | unit | Chain | 2026-03-03 | Asserts equality on `engine.chain_count()` |  |
| [register_chain_increases_count](../../../crates/content-engine/src/chain.rs#L301) | unit | Chain | 2026-03-03 | Asserts equality on `engine.chain_count()` |  |
| [chains_sorted_by_priority_descending](../../../crates/content-engine/src/chain.rs#L310) | unit | Chain | 2026-03-03 | Chains sorted by priority descending |  |
| [fire_event_with_no_matching_chains](../../../crates/content-engine/src/chain.rs#L341) | unit | Chain | 2026-03-03 | Asserts on `ctx.results.is_empty()` |  |
| [disabled_chain_is_skipped](../../../crates/content-engine/src/chain.rs#L356) | unit | Chain | 2026-03-03 | Disabled chain is skipped |  |
| [trigger_chain_action_produces_chain_trigger_result](../../../crates/content-engine/src/chain.rs#L380) | unit | Chain | 2026-03-03 | Trigger chain action produces chain trigger result |  |
| [chain_serialization_roundtrip](../../../crates/content-engine/src/chain.rs#L414) | unit | Chain | 2026-03-03 | Chain serialization roundtrip |  |
| [multiple_trigger_types_are_independent](../../../crates/content-engine/src/chain.rs#L450) | unit | Chain | 2026-03-03 | Multiple trigger types are independent |  |
| [faction_relation_serialization_roundtrip](../../../crates/content-engine/src/conditions.rs#L266) | unit | Conditions | 2026-03-03 | Asserts equality on `*rel` |  |
| [condition_serialization_roundtrip](../../../crates/content-engine/src/conditions.rs#L280) | unit | Conditions | 2026-03-03 | Condition serialization roundtrip | smell: no_assert_or_question_mark |
| [has_item_condition_serialization](../../../crates/content-engine/src/conditions.rs#L291) | unit | Conditions | 2026-03-03 | Asserts on `json.contains("42")` |  |
| [mission_status_eq_not_active](../../../crates/content-engine/src/conditions.rs#L302) | unit | Conditions | 2026-03-07 | Asserts on `condition.evaluate(&ctx)` |  |
| [mission_status_neq_active](../../../crates/content-engine/src/conditions.rs#L314) | unit | Conditions | 2026-03-07 | Asserts on `condition.evaluate(&ctx)` |  |
| [step_status_active](../../../crates/content-engine/src/conditions.rs#L329) | unit | Conditions | 2026-03-07 | Asserts on `condition.evaluate(&ctx)` |  |
| [archetype_eq](../../../crates/content-engine/src/conditions.rs#L345) | unit | Conditions | 2026-03-07 | Asserts on `condition.evaluate(&ctx)` |  |
| [archetype_neq](../../../crates/content-engine/src/conditions.rs#L356) | unit | Conditions | 2026-03-07 | Asserts on `condition.evaluate(&ctx)` |  |
| [counter_gte](../../../crates/content-engine/src/conditions.rs#L367) | unit | Conditions | 2026-03-07 | Asserts on `condition.evaluate(&ctx)` |  |
| [new_context_is_empty](../../../crates/content-engine/src/context.rs#L94) | unit | Context | 2026-03-03 | Asserts on `ctx.source_entity_id.is_none()` |  |
| [builder_pattern](../../../crates/content-engine/src/context.rs#L104) | unit | Context | 2026-03-03 | Asserts equality on `ctx.source_entity_id` |  |
| [set_and_get_param](../../../crates/content-engine/src/context.rs#L115) | unit | Context | 2026-03-03 | Asserts equality on `ctx.get_param("damage")` |  |
| [get_missing_param_returns_none](../../../crates/content-engine/src/context.rs#L122) | unit | Context | 2026-03-03 | Asserts on `ctx.get_param("nonexistent").is_none()` |  |
| [set_param_overwrites](../../../crates/content-engine/src/context.rs#L128) | unit | Context | 2026-03-03 | Asserts equality on `ctx.get_param("level")` |  |
| [load_empty_array](../../../crates/content-engine/src/loader.rs#L599) | unit | Loader | 2026-03-03 | Asserts on `chains.is_empty()` |  |
| [load_single_chain](../../../crates/content-engine/src/loader.rs#L605) | unit | Loader | 2026-03-03 | Load single chain |  |
| [load_invalid_json_returns_error](../../../crates/content-engine/src/loader.rs#L627) | unit | Loader | 2026-03-03 | Asserts on `result.is_err()` |  |
| [build_chains_from_db_rows](../../../crates/content-engine/src/loader.rs#L633) | unit | Loader | 2026-03-07 | Build chains from db rows |  |
| [triggerless_chain_gets_custom_event](../../../crates/content-engine/src/loader.rs#L693) | unit | Loader | 2026-03-07 | Triggerless chain gets custom event |  |
| [parse_destination_valid](../../../crates/content-engine/src/loader.rs#L723) | unit | Loader | 2026-03-07 | Asserts equality on `parse_destination("-123.625,1.311,-246.858")` |  |
| [parse_destination_invalid](../../../crates/content-engine/src/loader.rs#L731) | unit | Loader | 2026-03-07 | Asserts equality on `parse_destination("bad")` |  |
| [convert_interact_tag_trigger](../../../crates/content-engine/src/loader.rs#L736) | unit | Loader | 2026-03-07 | Asserts equality on `entity_tag` |  |
| [convert_move_waypoint_action](../../../crates/content-engine/src/loader.rs#L755) | unit | Loader | 2026-03-11 | Convert move waypoint action |  |
| [convert_set_active_slot_action](../../../crates/content-engine/src/loader.rs#L781) | unit | Loader | 2026-03-11 | Asserts equality on `bag_id` |  |
| [convert_set_active_slot_defaults_bandolier](../../../crates/content-engine/src/loader.rs#L802) | unit | Loader | 2026-03-11 | Asserts equality on `bag_id` |  |
| [convert_launch_ability_action](../../../crates/content-engine/src/loader.rs#L823) | unit | Loader | 2026-03-11 | Convert launch ability action |  |
| [convert_counter_condition](../../../crates/content-engine/src/loader.rs#L847) | unit | Loader | 2026-03-07 | Convert counter condition |  |
| [trigger_type_discriminant](../../../crates/content-engine/src/triggers.rs#L327) | unit | Triggers | 2026-03-03 | Asserts equality on `t.trigger_type()` |  |
| [wildcard_trigger_matches_any](../../../crates/content-engine/src/triggers.rs#L346) | unit | Triggers | 2026-03-03 | Asserts on `trigger.matches(&event)` |  |
| [filtered_trigger_matches_correct_type](../../../crates/content-engine/src/triggers.rs#L359) | unit | Triggers | 2026-03-03 | Asserts on `trigger.matches(&event)` |  |
| [filtered_trigger_rejects_wrong_type](../../../crates/content-engine/src/triggers.rs#L374) | unit | Triggers | 2026-03-03 | Asserts on `!trigger.matches(&event)` |  |
| [wrong_trigger_type_never_matches](../../../crates/content-engine/src/triggers.rs#L389) | unit | Triggers | 2026-03-03 | Asserts on `!trigger.matches(&event)` |  |
| [region_enter_matches_correct_key](../../../crates/content-engine/src/triggers.rs#L396) | unit | Triggers | 2026-03-07 | Asserts on `trigger.matches(&event)` |  |
| [region_enter_rejects_wrong_key](../../../crates/content-engine/src/triggers.rs#L408) | unit | Triggers | 2026-03-07 | Asserts on `!trigger.matches(&event)` |  |
| [mission_step_requires_both_fields](../../../crates/content-engine/src/triggers.rs#L420) | unit | Triggers | 2026-03-03 | Asserts on `trigger.matches(&event)` |  |
| [custom_event_matches_name](../../../crates/content-engine/src/triggers.rs#L446) | unit | Triggers | 2026-03-03 | Asserts on `trigger.matches(&event)` |  |
| [dialog_choice_matches](../../../crates/content-engine/src/triggers.rs#L461) | unit | Triggers | 2026-03-07 | Asserts on `trigger.matches(&event)` |  |
| [interact_tag_matches](../../../crates/content-engine/src/triggers.rs#L471) | unit | Triggers | 2026-03-07 | Asserts on `trigger.matches(&event)` |  |
| [entity_death_by_tag](../../../crates/content-engine/src/triggers.rs#L483) | unit | Triggers | 2026-03-07 | Asserts on `trigger.matches(&event)` |  |
| [effect_init_matches](../../../crates/content-engine/src/triggers.rs#L496) | unit | Triggers | 2026-03-07 | Asserts on `trigger.matches(&event)` |  |
| [mission_completed_matches](../../../crates/content-engine/src/triggers.rs#L503) | unit | Triggers | 2026-03-07 | Asserts on `trigger.matches(&event)` |  |
| [every_interact_tag_chain_has_set_interaction_type](../../../crates/content-engine/tests/interact_tag_linter.rs#L159) | integration | Tests / Interact Tag Linter | 2026-05-02 | Asserts on `seed_dir.exists()` |  |
| [scan_chains_picks_up_basic_pattern](../../../crates/content-engine/tests/interact_tag_linter.rs#L212) | integration | Tests / Interact Tag Linter | 2026-05-02 | Asserts on `triggers.contains_key(&9999)` |  |
