//! `BaseToCellMsg::CreateEntity` destination-instance handling.
//!
//! `destination_space_id` is the GM cross-instance transfer's way of saying
//! "join *this* instance" (see `crate::cell::space_transfer`). The default
//! resolution — `find_or_create_space` — cannot express that: for an
//! instanced world it allocates a brand new space on every create.
//!
//! The id is re-validated here rather than trusted, because the cell loop
//! keeps running between the transfer's validation and this message arriving,
//! and an instanced space is destroyed the moment its last player leaves.

use super::*;

const INSTANCED: &str = "Castle_CellBlock";
const FLAT: &str = "Agnos";

fn make_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
            <Space WorldName="Agnos" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

async fn create(
    mgr: &mut SpaceManager,
    entity_id: u32,
    world: &str,
    destination_space_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<u32> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    handle_base_message(
        BaseToCellMsg::CreateEntity {
            entity_id,
            world_name: world.to_string(),
            position: [1.0, 2.0, 3.0],
            rotation: [0.0; 3],
            destination_space_id,
            reply_tx,
        },
        tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    reply_rx.await.ok()
}

/// The point of the packet: a create that names an existing instance joins
/// THAT instance instead of allocating a fresh one.
///
/// Regression shape: ignore `destination_space_id` and the create falls
/// through to `find_or_create_space`, which allocates a third space — the
/// arriving GM ends up alone in an empty copy of the map.
#[tokio::test]
async fn create_entity_with_destination_space_id_joins_that_exact_instance() {
    let mut mgr = make_manager();
    let (tx, mut rx) = mpsc::channel(32);

    // Two live instances of the same instanced world.
    let space_a = create(&mut mgr, 1, INSTANCED, None, &tx).await.unwrap();
    mgr.connect_entity(1);
    let space_b = create(&mut mgr, 2, INSTANCED, None, &tx).await.unwrap();
    mgr.connect_entity(2);
    assert_ne!(
        space_a, space_b,
        "instanced creates must allocate distinctly"
    );
    let space_count_before = mgr.space_count();
    while rx.try_recv().is_ok() {}

    // Third entity explicitly joins instance B.
    let joined = create(&mut mgr, 3, INSTANCED, Some(space_b), &tx)
        .await
        .expect("create must reply with a space id");

    assert_eq!(joined, space_b, "must join instance B exactly");
    assert_eq!(
        mgr.get_entity_space_id(3),
        Some(space_b),
        "the cell entity must actually live in instance B"
    );
    assert_eq!(
        mgr.space_count(),
        space_count_before,
        "joining an existing instance must not allocate another space"
    );

    // Joining must not re-announce the space: a second SpaceData for an
    // already-registered instance would re-register it base-side as if it
    // were new.
    let mut space_data_count = 0;
    let mut entity_created = None;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::SpaceData { .. } => space_data_count += 1,
            CellToBaseMsg::EntityCreated {
                entity_id: 3,
                space_id,
                ..
            } => entity_created = Some(space_id),
            _ => {}
        }
    }
    assert_eq!(
        space_data_count, 0,
        "joining an existing instance must not emit SpaceData"
    );
    assert_eq!(
        entity_created,
        Some(space_b),
        "EntityCreated must report the joined instance"
    );
}

/// A create with no `destination_space_id` keeps the historical behavior —
/// this is the control that proves the test above isn't passing by accident.
#[tokio::test]
async fn create_entity_without_destination_space_id_allocates_a_fresh_instance() {
    let mut mgr = make_manager();
    let (tx, mut rx) = mpsc::channel(32);

    let space_a = create(&mut mgr, 1, INSTANCED, None, &tx).await.unwrap();
    mgr.connect_entity(1);
    while rx.try_recv().is_ok() {}

    let space_b = create(&mut mgr, 2, INSTANCED, None, &tx).await.unwrap();
    assert_ne!(
        space_b, space_a,
        "an instanced world without an explicit destination must allocate a NEW space"
    );

    let mut space_data = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::SpaceData { space_id, .. } = msg {
            space_data.push(space_id);
        }
    }
    assert_eq!(
        space_data,
        vec![space_b],
        "a freshly allocated instance must be announced exactly once"
    );
}

/// The instance died between the transfer's validation and this message
/// (its last player left, so `destroy_entity` reaped it). Degrading to
/// by-world-name resolution keeps the player *in a space*; failing the create
/// would leave them in none at all, which is the worse outcome.
#[tokio::test]
async fn stale_destination_space_id_degrades_to_world_name_resolution() {
    let mut mgr = make_manager();
    let (tx, mut rx) = mpsc::channel(32);

    let space_a = create(&mut mgr, 1, INSTANCED, None, &tx).await.unwrap();
    mgr.connect_entity(1);
    // Last player leaves → the instanced space is reaped.
    mgr.destroy_entity(1);
    assert!(
        mgr.world_name_for_space(space_a).is_none(),
        "fixture must actually reap the instance"
    );
    while rx.try_recv().is_ok() {}

    let joined = create(&mut mgr, 2, INSTANCED, Some(space_a), &tx)
        .await
        .expect("a stale destination must still produce a usable space");

    assert_ne!(joined, space_a, "the stale instance is gone");
    assert_eq!(
        mgr.get_entity_space_id(2),
        Some(joined),
        "the entity must end up in the freshly allocated instance, not in no space"
    );
}

/// A `destination_space_id` pointing at a live space of a *different* world
/// must not be honoured — that would drop the entity into the wrong map
/// entirely, with the base still building world-entry packets for the world
/// it was told about.
#[tokio::test]
async fn destination_space_id_from_another_world_is_ignored() {
    let mut mgr = make_manager();
    let (tx, _rx) = mpsc::channel(32);

    let agnos = mgr.space_id_for_world(FLAT).expect("startup space");
    let joined = create(&mut mgr, 1, INSTANCED, Some(agnos), &tx)
        .await
        .expect("create must still succeed");

    assert_ne!(
        joined, agnos,
        "a cross-world instance id must be refused, not followed"
    );
    assert_eq!(
        mgr.world_name_for_space(joined),
        Some(INSTANCED),
        "the entity must land in the world the message named"
    );
}
