//! Seed lookups that turn "template N" or "the item the Guard hands over"
//! into facts the wire shows (README D-WC1). The mirror never sees a
//! template id, so a test asks the seed what the client is told about one.

use sqlx::PgPool;

use cimmeria_wireclient::mirror::EntityQuery;

/// The mirror query for `template_id`: its `static_mesh` and `name_id` from
/// `resources.entity_templates`, the two facts the AoI cascade carries
/// (`onStaticMeshNameUpdate`, `onBeingNameIDUpdate`). Either column may be
/// NULL in the seed; a NULL is left out of the query, and a template with
/// both NULL panics, since nothing on the wire could then identify it.
pub async fn template_query(pool: &PgPool, template_id: i32) -> EntityQuery {
    let (static_mesh, name_id): (Option<String>, Option<i32>) = sqlx::query_as(
        "SELECT static_mesh, name_id FROM resources.entity_templates WHERE template_id = $1",
    )
    .bind(template_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("read entity template {template_id}: {e}"));
    let mut query = EntityQuery {
        static_mesh,
        ..EntityQuery::default()
    };
    if let Some(id) = name_id {
        query = query.name_id(id);
    }
    assert!(
        query.static_mesh.is_some() || query.name_id.is_some(),
        "template {template_id} has neither a static mesh nor a name id"
    );
    query
}

/// The bandolier-equippable item design that opening `dialog_id` grants:
/// the `add_item` actions of every enabled chain triggered by `dialog_open`
/// of that dialog, kept to designs whose `container_sets` include the
/// bandolier (3). Several chains may grant it (archetype-gated twins, such
/// as chains 1005 and 1010 on dialog 3996); they must agree on one design.
pub async fn item_granted_by_dialog(pool: &PgPool, dialog_id: i32) -> i32 {
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT a.target_id FROM resources.content_actions a \
         JOIN resources.content_triggers t ON t.chain_id = a.chain_id \
         JOIN resources.content_chains c ON c.chain_id = a.chain_id \
         JOIN resources.items i ON i.item_id = a.target_id \
         WHERE c.enabled AND t.event_type = 'dialog_open' AND t.event_key = $1 \
           AND a.action_type = 'add_item' AND 3 = ANY(i.container_sets)",
    )
    .bind(dialog_id.to_string())
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| panic!("read the items dialog {dialog_id} grants: {e}"));
    match ids.as_slice() {
        [only] => *only,
        other => panic!(
            "dialog {dialog_id} grants bandolier designs {other:?}, expected exactly one \
             (for 3996: chains 1005 and 1010's pistol). A second bandolier add_item on \
             that trigger (ammo for the pistol, OD-CS13, if its design fits the \
             bandolier; or a weapon per archetype) needs this query to pick one"
        ),
    }
}
