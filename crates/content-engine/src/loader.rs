//! Chain loading from JSON and database sources.
//!
//! Provides deserialization of chain definitions from JSON (for testing,
//! import/export, and the ServerEd chain editor) and a placeholder for
//! database loading (which will use sqlx once the services crate provides
//! a connection pool).

use crate::chain::Chain;

/// Deserialize a list of chains from a JSON string.
///
/// The JSON should be an array of [`Chain`] objects. This is the primary
/// entry point for loading chains from JSON files exported by the ServerEd
/// chain editor, or for test fixtures.
///
/// # Errors
///
/// Returns a `serde_json::Error` if the JSON is malformed or does not match
/// the expected `Chain` schema.
///
/// # Example
///
/// ```
/// use cimmeria_content_engine::loader::load_chains_from_json;
///
/// let json = r#"[{
///     "id": 1,
///     "name": "Test",
///     "enabled": true,
///     "trigger": { "OnCustomEvent": { "event_name": "test" } },
///     "conditions": [],
///     "actions": [],
///     "priority": 0
/// }]"#;
///
/// let chains = load_chains_from_json(json).unwrap();
/// assert_eq!(chains.len(), 1);
/// assert_eq!(chains[0].name, "Test");
/// ```
pub fn load_chains_from_json(json: &str) -> Result<Vec<Chain>, serde_json::Error> {
    serde_json::from_str(json)
}

/// Load chains from the PostgreSQL database.
///
/// This function will query the `content_chains` table (or equivalent) and
/// deserialize each row into a [`Chain`]. The implementation requires a
/// database connection pool from the `cimmeria-services` crate, which is not
/// yet available.
///
/// # Future Implementation
///
/// When the services crate provides a `DbPool` type, this function's
/// signature will become:
///
/// ```ignore
/// pub async fn load_chains_from_db(pool: &DbPool) -> Result<Vec<Chain>, CimmeriaError> {
///     // SELECT id, name, enabled, trigger_json, conditions_json, actions_json, priority
///     // FROM content_chains
///     // WHERE enabled = true
///     // ORDER BY priority DESC
/// }
/// ```
///
/// The database schema stores trigger, conditions, and actions as JSONB
/// columns, allowing the same serde deserialization used by
/// [`load_chains_from_json`].
pub fn load_chains_from_db() -> cimmeria_common::Result<Vec<Chain>> {
    todo!("load_chains_from_db - requires sqlx pool from cimmeria-services crate")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_empty_array() {
        let chains = load_chains_from_json("[]").unwrap();
        assert!(chains.is_empty());
    }

    #[test]
    fn load_single_chain() {
        let json = r#"[{
            "id": 1,
            "name": "Grant XP on mob kill",
            "enabled": true,
            "trigger": {
                "OnEntityDeath": { "entity_type": "SGWMob" }
            },
            "conditions": [],
            "actions": [
                { "GrantXP": { "amount": 100 } }
            ],
            "priority": 10
        }]"#;

        let chains = load_chains_from_json(json).unwrap();
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].id, 1);
        assert_eq!(chains[0].name, "Grant XP on mob kill");
        assert!(chains[0].enabled);
        assert_eq!(chains[0].priority, 10);
        assert!(chains[0].conditions.is_empty());
        assert_eq!(chains[0].actions.len(), 1);
    }

    #[test]
    fn load_chain_with_conditions_and_multiple_actions() {
        let json = r#"[{
            "id": 2,
            "name": "Boss loot with key check",
            "enabled": true,
            "trigger": {
                "OnEntityDeath": { "entity_type": "SGWBoss" }
            },
            "conditions": [
                { "HasItem": { "item_id": 99, "min_count": 1 } },
                { "InRegion": { "region_id": 42 } }
            ],
            "actions": [
                { "RollLootTable": { "table_id": 50 } },
                { "GrantXP": { "amount": 500 } },
                { "SendMessage": { "channel": "system", "message": "Boss defeated!" } }
            ],
            "priority": 100
        }]"#;

        let chains = load_chains_from_json(json).unwrap();
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].conditions.len(), 2);
        assert_eq!(chains[0].actions.len(), 3);
    }

    #[test]
    fn load_multiple_chains() {
        let json = r#"[
            {
                "id": 1,
                "name": "Chain A",
                "enabled": true,
                "trigger": { "OnTimer": { "timer_name": "respawn" } },
                "conditions": [],
                "actions": [
                    { "SpawnEntity": { "template_id": 10, "position": [1.0, 2.0, 3.0] } }
                ],
                "priority": 0
            },
            {
                "id": 2,
                "name": "Chain B",
                "enabled": false,
                "trigger": { "OnCustomEvent": { "event_name": "wave_start" } },
                "conditions": [],
                "actions": [],
                "priority": 5
            }
        ]"#;

        let chains = load_chains_from_json(json).unwrap();
        assert_eq!(chains.len(), 2);
        assert_eq!(chains[0].name, "Chain A");
        assert!(chains[0].enabled);
        assert_eq!(chains[1].name, "Chain B");
        assert!(!chains[1].enabled);
    }

    #[test]
    fn load_invalid_json_returns_error() {
        let result = load_chains_from_json("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn load_chain_with_all_action_types_serialized() {
        // Verify that all 21 action types can be deserialized.
        let json = r#"[{
            "id": 99,
            "name": "All actions test",
            "enabled": true,
            "trigger": { "OnCustomEvent": { "event_name": "test" } },
            "conditions": [],
            "actions": [
                { "GrantXP": { "amount": 100 } },
                { "GrantItem": { "item_id": 1, "count": 5 } },
                { "RemoveItem": { "item_id": 1, "count": 1 } },
                { "ApplyEffect": { "effect_id": 10, "duration_secs": 30.0 } },
                { "RemoveEffect": { "effect_id": 10 } },
                { "Teleport": { "space_id": 1, "position": [0.0, 0.0, 0.0] } },
                { "SpawnEntity": { "template_id": 5, "position": [10.0, 0.0, 10.0] } },
                "DespawnEntity",
                { "StartDialog": { "dialog_set_id": 3 } },
                { "AdvanceMission": { "mission_id": 7 } },
                { "CompleteMission": { "mission_id": 7 } },
                { "PlayAnimation": { "animation": "cheer" } },
                { "PlaySound": { "sound": "victory_fanfare" } },
                { "SendMessage": { "channel": "system", "message": "Hello" } },
                { "ModifyProperty": { "property": "health", "operation": "Add", "value": 50 } },
                { "RollLootTable": { "table_id": 20 } },
                { "SpawnLootBag": { "position": [5.0, 0.0, 5.0] } },
                { "StartTimer": { "name": "respawn", "duration_secs": 60.0, "repeat": true } },
                { "CancelTimer": { "name": "respawn" } },
                { "TriggerChain": { "chain_id": 42 } },
                { "ExecuteCustom": { "handler": "my_handler", "params": {"key": "value"} } }
            ],
            "priority": 0
        }]"#;

        let chains = load_chains_from_json(json).unwrap();
        assert_eq!(chains[0].actions.len(), 21);
    }

    #[test]
    fn load_chain_with_all_condition_types() {
        let json = r#"[{
            "id": 98,
            "name": "All conditions test",
            "enabled": true,
            "trigger": { "OnCustomEvent": { "event_name": "test" } },
            "conditions": [
                { "PropertyEquals": { "property": "level", "value": 50 } },
                { "PropertyInRange": { "property": "health", "min": 0.0, "max": 100.0 } },
                { "HasItem": { "item_id": 42, "min_count": 1 } },
                { "HasAbility": { "ability_id": 7 } },
                { "InRegion": { "region_id": 3 } },
                { "FactionCheck": { "faction": "SGC", "relation": "Friendly" } },
                { "CustomExpression": { "expression": "source.level > 10" } }
            ],
            "actions": [],
            "priority": 0
        }]"#;

        let chains = load_chains_from_json(json).unwrap();
        assert_eq!(chains[0].conditions.len(), 7);
    }

    #[test]
    fn load_chain_with_all_trigger_types() {
        // Each trigger type in its own chain to verify deserialization.
        let json = r#"[
            { "id": 1, "name": "t1", "enabled": true, "priority": 0,
              "trigger": { "OnEntityCreated": { "entity_type": null } },
              "conditions": [], "actions": [] },
            { "id": 2, "name": "t2", "enabled": true, "priority": 0,
              "trigger": { "OnEntityDestroyed": { "entity_type": "SGWMob" } },
              "conditions": [], "actions": [] },
            { "id": 3, "name": "t3", "enabled": true, "priority": 0,
              "trigger": { "OnEntityDeath": { "entity_type": null } },
              "conditions": [], "actions": [] },
            { "id": 4, "name": "t4", "enabled": true, "priority": 0,
              "trigger": { "OnAbilityUsed": { "ability_id": 5 } },
              "conditions": [], "actions": [] },
            { "id": 5, "name": "t5", "enabled": true, "priority": 0,
              "trigger": { "OnInteraction": { "interaction_type": "dialog" } },
              "conditions": [], "actions": [] },
            { "id": 6, "name": "t6", "enabled": true, "priority": 0,
              "trigger": { "OnRegionEnter": { "region_id": 1 } },
              "conditions": [], "actions": [] },
            { "id": 7, "name": "t7", "enabled": true, "priority": 0,
              "trigger": { "OnRegionExit": { "region_id": 1 } },
              "conditions": [], "actions": [] },
            { "id": 8, "name": "t8", "enabled": true, "priority": 0,
              "trigger": { "OnMissionStep": { "mission_id": 10, "step": 3 } },
              "conditions": [], "actions": [] },
            { "id": 9, "name": "t9", "enabled": true, "priority": 0,
              "trigger": { "OnItemAcquired": { "item_id": null } },
              "conditions": [], "actions": [] },
            { "id": 10, "name": "t10", "enabled": true, "priority": 0,
              "trigger": { "OnTimer": { "timer_name": "wave" } },
              "conditions": [], "actions": [] },
            { "id": 11, "name": "t11", "enabled": true, "priority": 0,
              "trigger": { "OnCustomEvent": { "event_name": "custom" } },
              "conditions": [], "actions": [] }
        ]"#;

        let chains = load_chains_from_json(json).unwrap();
        assert_eq!(chains.len(), 11);
    }
}
