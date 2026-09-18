//! DB-row → `Action` conversions for the entity-lifecycle verbs
//! (`spawn_entity` / `despawn_entity`, Harset H03).
//!
//! Every test here goes through [`convert_action`], not
//! `convert_spawn_action` directly: the delegation from `action.rs`'s
//! fallthrough is part of what has to keep working. Deleting the
//! `_ => super::action_spawn::convert_spawn_action(row)` arm makes all of
//! the positive tests below fail with `None`.

use super::super::action::convert_action;
use super::super::*;

/// The full spawn descriptor must survive the row → `Action` conversion
/// with every optional override preserved as `Some`. A regression that
/// defaults `respawn_secs` / `is_stationary` / `aggression` / `allow_shared`
/// instead of carrying `Option` would make a seeded hostile spawn passive
/// and a seeded shared-world spawn silently refused.
#[test]
fn convert_spawn_entity_full_descriptor() {
    let row = DbActionRow {
        chain_id: 6301,
        action_type: "spawn_entity".to_string(),
        target_id: Some(207),
        target_key: Some("Rinla_Malac".to_string()),
        params: serde_json::json!({
            "x": -123.625, "y": 1.311, "z": -246.858,
            "heading": 2.71875,
            "respawn_secs": 45,
            "is_stationary": true,
            "aggression": 1,
            "allow_shared": true
        }),
        delay_ms: 0,
        sort_order: 0,
    };
    match convert_action(&row).expect("spawn_entity must convert") {
        Action::SpawnEntity {
            template_id,
            position,
            heading,
            tag,
            respawn_secs,
            is_stationary,
            aggression,
            allow_shared,
        } => {
            assert_eq!(template_id, 207, "template_id comes from target_id");
            assert_eq!(tag, "Rinla_Malac", "tag comes from target_key");
            assert_eq!(position, [-123.625, 1.311, -246.858]);
            // Deliberately not near a `f32::consts` value — clippy's
            // `approx_constant` fires on literals like 3.14159, and a
            // heading that happened to equal PI would also be the one
            // value a "default to PI" regression could reproduce.
            assert!(
                (heading - 2.71875).abs() < 1e-5,
                "heading must survive the f64 → f32 narrowing, got {heading}"
            );
            assert_eq!(respawn_secs, Some(45));
            assert_eq!(is_stationary, Some(true));
            assert_eq!(aggression, Some(1));
            assert_eq!(allow_shared, Some(true));
        }
        other => panic!("Expected SpawnEntity, got {other:?}"),
    }
}

/// With only the mandatory fields, every override must be `None` — that is
/// the signal the executor uses to mean "inherit the template row". A
/// regression that substituted `Some(0)` / `Some(false)` here would force a
/// passive, non-respawning, non-stationary spawn onto templates that
/// specify otherwise.
#[test]
fn convert_spawn_entity_minimal_leaves_overrides_none() {
    let row = DbActionRow {
        chain_id: 6302,
        action_type: "spawn_entity".to_string(),
        target_id: Some(210),
        target_key: Some("Storage_Petbe".to_string()),
        params: serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0}),
        delay_ms: 0,
        sort_order: 0,
    };
    match convert_action(&row).expect("spawn_entity must convert") {
        Action::SpawnEntity {
            heading,
            respawn_secs,
            is_stationary,
            aggression,
            allow_shared,
            ..
        } => {
            assert_eq!(heading, 0.0, "absent heading defaults to 0, not NaN");
            assert_eq!(respawn_secs, None);
            assert_eq!(is_stationary, None);
            assert_eq!(aggression, None);
            assert_eq!(
                allow_shared, None,
                "absent allow_shared must stay None so the executor refuses \
                 a shared-world spawn by default"
            );
        }
        other => panic!("Expected SpawnEntity, got {other:?}"),
    }
}

/// Missing coordinates drop the row rather than spawning at the world
/// origin. Same failure class `cross_world_teleport` guards against: (0,0,0)
/// is inside the floor or outside the map on every SGW world.
#[test]
fn convert_spawn_entity_without_coords_is_dropped() {
    let row = DbActionRow {
        chain_id: 6303,
        action_type: "spawn_entity".to_string(),
        target_id: Some(210),
        target_key: Some("NoCoords".to_string()),
        params: serde_json::json!({"heading": 1.0}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(
        convert_action(&row).is_none(),
        "a spawn row with no x/y/z must be dropped, not spawned at (0,0,0)"
    );
}

/// A non-finite coordinate is dropped for the same reason a missing one is
/// — `NaN` propagates into the spatial grid and the AoI distance test.
#[test]
fn convert_spawn_entity_with_non_finite_coord_is_dropped() {
    let row = DbActionRow {
        chain_id: 6304,
        action_type: "spawn_entity".to_string(),
        target_id: Some(210),
        target_key: Some("BadCoords".to_string()),
        params: serde_json::json!({"x": 1.0, "y": f64::INFINITY, "z": 3.0}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(
        convert_action(&row).is_none(),
        "a non-finite coordinate must drop the spawn row"
    );
}

/// The tag is mandatory. An untagged spawn can never be despawned, killed
/// by `entity_dead_tag`, or interacted with — it is a permanent orphan in
/// the player's instance.
#[test]
fn convert_spawn_entity_without_tag_is_dropped() {
    let row = DbActionRow {
        chain_id: 6305,
        action_type: "spawn_entity".to_string(),
        target_id: Some(210),
        target_key: None,
        params: serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(
        convert_action(&row).is_none(),
        "a spawn row with no target_key (tag) must be dropped"
    );
}

/// An empty-string tag is the same orphan case as a missing one, but
/// `target_key` is `Some("")` so the `?` short-circuit doesn't catch it.
#[test]
fn convert_spawn_entity_with_empty_tag_is_dropped() {
    let row = DbActionRow {
        chain_id: 6306,
        action_type: "spawn_entity".to_string(),
        target_id: Some(210),
        target_key: Some(String::new()),
        params: serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(
        convert_action(&row).is_none(),
        "an empty tag must be dropped — `?` on target_key does not catch Some(\"\")"
    );
}

/// A spawn row with no `target_id` has no template to instantiate.
#[test]
fn convert_spawn_entity_without_template_is_dropped() {
    let row = DbActionRow {
        chain_id: 6307,
        action_type: "spawn_entity".to_string(),
        target_id: None,
        target_key: Some("NoTemplate".to_string()),
        params: serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(
        convert_action(&row).is_none(),
        "a spawn row with no target_id (template) must be dropped"
    );
}

/// `despawn_entity` carries only the tag.
#[test]
fn convert_despawn_entity_action() {
    let row = DbActionRow {
        chain_id: 6308,
        action_type: "despawn_entity".to_string(),
        target_id: None,
        target_key: Some("Crogan_Corpse".to_string()),
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    };
    match convert_action(&row).expect("despawn_entity must convert") {
        Action::DespawnEntity { entity_tag } => assert_eq!(entity_tag, "Crogan_Corpse"),
        other => panic!("Expected DespawnEntity, got {other:?}"),
    }
}

/// Without a tag there is nothing to despawn.
#[test]
fn convert_despawn_entity_without_tag_is_dropped() {
    let row = DbActionRow {
        chain_id: 6309,
        action_type: "despawn_entity".to_string(),
        target_id: None,
        target_key: None,
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(
        convert_action(&row).is_none(),
        "a despawn row with no target_key must be dropped"
    );
}

/// The delegation must not swallow genuinely unknown verbs — those still
/// have to reach the caller as `None` so the loader logs its
/// "Unknown action_type" warn.
#[test]
fn unknown_action_type_still_returns_none() {
    let row = DbActionRow {
        chain_id: 6310,
        action_type: "summon_replicator".to_string(),
        target_id: Some(1),
        target_key: Some("x".to_string()),
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(convert_action(&row).is_none());
}
