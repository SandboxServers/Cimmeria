//! Live-DB guards for `entity_templates.vault_scope` (bank-vault BV-02,
//! D-BV09).
//!
//! The column is `text NOT NULL DEFAULT 'personal'` with a CHECK for the
//! three scopes. A banker template that sets `team` must reach the spawned
//! NPC as `Banker { scope: Team }`; one that omits the column must default
//! to `personal`; and the DB must refuse anything else.

use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope};
use cimmeria_entity::interaction_flags::INT_BANKER;

use crate::cell::spawner::{load_spawn_templates, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

/// Sentinel template id, unique to this file. Fits in `i32`.
const SENTINEL_TEMPLATE_ID: i32 = 0x7000_B402;

async fn delete_sentinel(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM resources.entity_templates WHERE template_id = $1")
        .bind(SENTINEL_TEMPLATE_ID)
        .execute(pool)
        .await
        .expect("sentinel template cleanup must succeed");
}

/// Insert the sentinel banker template. `None` omits the column, so the
/// DB default applies.
async fn insert_sentinel(pool: &sqlx::PgPool, vault_scope: Option<&str>) -> sqlx::Result<()> {
    let q = match vault_scope {
        None => sqlx::query(
            "INSERT INTO resources.entity_templates \
                 (template_id, template_name, class, body_set, interaction_type, faction) \
             VALUES ($1, 'BV02 Banker Probe', 'mob', 'GLB_Components.WorldObject_Small', $2, 1)",
        ),
        Some(_) => sqlx::query(
            "INSERT INTO resources.entity_templates \
                 (template_id, template_name, class, body_set, interaction_type, faction, \
                  vault_scope) \
             VALUES ($1, 'BV02 Banker Probe', 'mob', 'GLB_Components.WorldObject_Small', $2, 1, $3)",
        ),
    };
    let q = q.bind(SENTINEL_TEMPLATE_ID).bind(INT_BANKER);
    let q = match vault_scope {
        Some(scope) => q.bind(scope),
        None => q,
    };
    q.execute(pool).await.map(|_| ())
}

/// Load the sentinel through the template cache and spawn it.
async fn spawned_interaction(pool: &sqlx::PgPool) -> Option<NpcInteractionType> {
    let loaded = load_spawn_templates(pool).await;
    delete_sentinel(pool).await;
    let mut record = loaded
        .expect("load_spawn_templates must succeed")
        .remove(&SENTINEL_TEMPLATE_ID)
        .expect("the sentinel template must surface from the loader");
    record.world_name = "Agnos".to_string();
    let mut mgr = crate::test_support::make_space_manager();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc_id, &record)
        .expect("sentinel NPC must spawn");
    mgr.get_entity(npc_id).unwrap().interaction_type.clone()
}

/// A template that omits `vault_scope` defaults to `personal`. Fails if the
/// column's DEFAULT is removed (the insert then violates NOT NULL).
#[tokio::test]
async fn vault_scope_defaults_to_personal() {
    let pool = require_db_or_skip!();
    delete_sentinel(&pool).await;
    insert_sentinel(&pool, None)
        .await
        .expect("a template without vault_scope must insert");
    assert_eq!(
        spawned_interaction(&pool).await,
        Some(NpcInteractionType::Banker {
            scope: VaultScope::Personal
        })
    );
}

/// `team` loads and reaches the spawned Banker. Fails if the column is
/// dropped from the shared template SELECT (the load errors) or not mapped.
#[tokio::test]
async fn vault_scope_team_reaches_the_spawned_banker() {
    let pool = require_db_or_skip!();
    delete_sentinel(&pool).await;
    insert_sentinel(&pool, Some("team"))
        .await
        .expect("vault_scope = 'team' must insert");
    assert_eq!(
        spawned_interaction(&pool).await,
        Some(NpcInteractionType::Banker {
            scope: VaultScope::Team
        })
    );
}

/// The DB refuses an unknown scope. Fails if the CHECK is removed.
#[tokio::test]
async fn vault_scope_check_rejects_an_unknown_value() {
    let pool = require_db_or_skip!();
    delete_sentinel(&pool).await;
    let bogus = insert_sentinel(&pool, Some("guild")).await;
    delete_sentinel(&pool).await;
    let err = bogus.expect_err("vault_scope = 'guild' must violate the CHECK");
    assert!(
        err.to_string()
            .contains("entity_templates_vault_scope_known"),
        "wrong error: {err}"
    );
}

/// The seeded-spawn loader reads the column too, and no seeded template is
/// a Banker yet (BV-04 adds template 370): every seeded spawn loads
/// `Personal`. Fails if `load_spawns_from_db` stops selecting the column.
#[tokio::test]
async fn seeded_spawns_load_vault_scope() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db must select vault_scope");
    assert!(!spawns.is_empty(), "the seed has spawns");
    assert!(
        spawns.iter().all(|s| s.vault_scope == VaultScope::Personal),
        "no seeded template sets a non-personal vault_scope yet"
    );
}
