//! PT-02 telemetry (negative-log / LogCapture, TESTING.md type 12): the
//! `pets.lifecycle` rows the owner hooks and the corpse timer emit, and the
//! miss seams (`teleport_skipped`, `grounding_missed`). Every row about a
//! player's pet names the owner's account and character.

use std::time::Instant;

use tokio::sync::mpsc;

use crate::cell::pets::{
    on_owner_left, on_owner_teleported, pet_owner_sweep_at, OwnerPath, PetDespawnReason,
    PET_CORPSE_DESPAWN,
};
use crate::test_fixtures::{watched_pet_world, PET_FIXTURE_OWNER as OWNER};
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
use cimmeria_wire::state_field::BSF_DEAD;

/// The owner's character id in these tests (`add_pet_owner` sets the
/// account id to the entity id, 7, and the character to 1007). The pet's
/// summoner capture is taken at summon, so the fixture's identity must not
/// be changed afterwards or the owner no longer matches it.
const OWNER_PLAYER_ID: i32 = OWNER as i32 + 1000;

fn world() -> (crate::cell::space_manager::SpaceManager, u32) {
    watched_pet_world()
}

/// The single `pets.lifecycle` row with `event = <event>`.
fn row(logs: &LogCaptureGuard, event: &str) -> Captured {
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "pets.lifecycle" && c.has_field("event", event))
        .collect();
    assert_eq!(rows.len(), 1, "one `{event}` row expected: {rows:#?}");
    rows.into_iter().next().unwrap()
}

fn assert_fields(row: &Captured, fields: &[(&str, &str)]) {
    for (k, v) in fields {
        assert!(row.has_field(k, v), "field {k}={v} missing: {row:#?}");
    }
}

fn owner_fields() -> [(&'static str, String); 3] {
    [
        ("owner_id", OWNER.to_string()),
        ("account_id", OWNER.to_string()),
        ("player_id", OWNER_PLAYER_ID.to_string()),
    ]
}

fn assert_owner(row: &Captured) {
    for (k, v) in owner_fields() {
        assert!(row.has_field(k, &v), "field {k}={v} missing: {row:#?}");
    }
}

/// `owner_left` (DEBUG) then one `despawned` (INFO) per pet, both with the
/// owner's identity (resolved before the despawn), the reason and the path.
#[tokio::test]
async fn owner_left_logs_the_owner_path_reason_and_identity() {
    let (mut mgr, pet) = world();
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    on_owner_left(
        OWNER,
        PetDespawnReason::OwnerLeftSpace,
        OwnerPath::GateTravel,
        &tx,
        &mut mgr,
    )
    .await;

    let left = row(&logs, "owner_left");
    assert_eq!(left.level, tracing::Level::DEBUG);
    assert_owner(&left);
    assert_fields(
        &left,
        &[
            ("reason", "owner_left_space"),
            ("path", "gate_travel"),
            ("pet_count", "1"),
        ],
    );
    let gone = row(&logs, "despawned");
    assert_owner(&gone);
    assert_fields(
        &gone,
        &[
            ("pet_id", &pet.to_string()),
            ("reason", "owner_left_space"),
            ("path", "gate_travel"),
        ],
    );
}

/// `owner_teleported` carries from / to / distance and how the spot was
/// grounded; with no navmesh a `grounding_missed` row says why.
#[tokio::test]
async fn owner_teleported_logs_from_to_distance_and_the_grounding_miss() {
    let (mut mgr, pet) = world();
    let from = mgr.get_entity(pet).unwrap().position;
    mgr.update_position_preserving_facing(OWNER, [300.0, 5.0, -40.0], [0.0; 3]);
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    on_owner_teleported(OWNER, OwnerPath::ConsoleTravel, &tx, &mut mgr).await;

    let moved = row(&logs, "owner_teleported");
    assert_eq!(moved.level, tracing::Level::DEBUG);
    assert_owner(&moved);
    assert_fields(
        &moved,
        &[
            ("pet_id", &pet.to_string()),
            ("path", "console_travel"),
            ("grounding", "no_navmesh"),
            ("witnesses_notified", "2"),
        ],
    );
    let num = |k: &str| -> f32 {
        moved.fields[k]
            .parse()
            .unwrap_or_else(|_| panic!("{k} not a number: {moved:#?}"))
    };
    for (k, want) in [
        ("from_x", from.x),
        ("from_z", from.z),
        ("to_x", 300.0),
        ("to_y", 5.0),
        ("to_z", -42.0),
    ] {
        assert!(
            (num(k) - want).abs() < 1e-3,
            "{k} = {}, want {want}",
            num(k)
        );
    }
    let d = num("distance");
    let want =
        ((300.0 - from.x).powi(2) + (5.0 - from.y).powi(2) + (-42.0 - from.z).powi(2)).sqrt();
    assert!((d - want).abs() < 1e-2, "distance {d}, want {want}");

    let miss = row(&logs, "grounding_missed");
    assert_owner(&miss);
    assert_fields(
        &miss,
        &[("reason", "no_navmesh"), ("path", "console_travel")],
    );
}

/// A navmesh space with the owner off the mesh: `grounding_missed` says
/// `owner_off_mesh`, not `no_navmesh`.
#[tokio::test]
async fn grounding_miss_names_an_owner_off_the_mesh() {
    use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
    let Some(mesh) = test_fixture_mesh() else {
        return;
    };
    let mut mgr = crate::test_fixtures::make_pet_world();
    test_insert_navmesh_space(&mut mgr, "CellblockNav", mesh);
    crate::test_fixtures::add_pet_owner(&mut mgr, OWNER, "CellblockNav", [0.0, 900.0, 0.0], 12);
    mgr.spawn_pet_from_template(OWNER, crate::test_fixtures::PET_FIXTURE_TEMPLATE_ID, 0)
        .unwrap();
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    on_owner_teleported(OWNER, OwnerPath::GmTravel, &tx, &mut mgr).await;

    assert_fields(
        &row(&logs, "grounding_missed"),
        &[("reason", "owner_off_mesh"), ("path", "gm_travel")],
    );
    assert_fields(
        &row(&logs, "owner_teleported"),
        &[("grounding", "owner_off_mesh")],
    );
}

/// Miss: a dead pet is not moved, and says so.
#[tokio::test]
async fn dead_pet_teleport_skip_is_logged() {
    let (mut mgr, pet) = world();
    mgr.get_entity_mut(pet).unwrap().set_state_flag(BSF_DEAD);
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    on_owner_teleported(OWNER, OwnerPath::Respawn, &tx, &mut mgr).await;

    let skip = row(&logs, "teleport_skipped");
    assert_eq!(skip.level, tracing::Level::DEBUG);
    assert_owner(&skip);
    assert_fields(
        &skip,
        &[
            ("pet_id", &pet.to_string()),
            ("reason", "pet_dead"),
            ("path", "respawn"),
        ],
    );
}

/// Miss: the teleported player holds the owner's id but did not summon the
/// pet (id reuse). The row names the pet's summoner beside the id's new
/// holder, so SigNoz shows whose pet was dropped and why.
#[tokio::test]
async fn reused_owner_id_teleport_skip_names_both_players() {
    let (mut mgr, old_pet) = world();
    let _new_pet = super::reuse_owner_id_then_resummon(&mut mgr);
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    on_owner_teleported(OWNER, OwnerPath::GmTravel, &tx, &mut mgr).await;

    let skip = row(&logs, "teleport_skipped");
    assert_eq!(skip.level, tracing::Level::DEBUG);
    assert_owner(&skip);
    assert_fields(
        &skip,
        &[
            ("pet_id", &old_pet.to_string()),
            ("reason", "owner_identity_mismatch"),
            ("path", "gm_travel"),
            ("holder_account_id", "4242"),
            ("holder_player_id", "4243"),
        ],
    );
    assert_fields(
        &row(&logs, "despawned"),
        &[("pet_id", &old_pet.to_string()), ("reason", "owner_gone")],
    );
}

/// Miss: a teleport hook for an owner that is in no space (a caller bug)
/// is a WARN with `reason = owner_not_found`, and nothing moves.
#[tokio::test]
async fn owner_not_found_teleport_skip_is_a_warn() {
    let (mut mgr, _pet) = world();
    mgr.destroy_entity(OWNER);
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    assert_eq!(
        on_owner_teleported(OWNER, OwnerPath::Ring, &tx, &mut mgr).await,
        0
    );

    let skip = row(&logs, "teleport_skipped");
    assert_eq!(skip.level, tracing::Level::WARN);
    assert_fields(
        &skip,
        &[
            ("owner_id", &OWNER.to_string()),
            ("reason", "owner_not_found"),
            ("path", "ring"),
            ("pet_count", "1"),
        ],
    );
    // The owner entity is gone, so the row names the pet's summoner
    // capture instead: still the right player, never a zero.
    assert_owner(&skip);
}

/// Corpse timer: `corpse_timer_started` then, 10 s later,
/// `corpse_expired` and a `despawned` row with `reason = corpse_expired`,
/// `path = sweep`, all with the owner's identity.
#[tokio::test]
async fn corpse_timer_start_and_expiry_are_logged() {
    let (mut mgr, pet) = world();
    mgr.get_entity_mut(pet).unwrap().set_state_flag(BSF_DEAD);
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();
    let t0 = Instant::now();

    pet_owner_sweep_at(t0, &tx, &mut mgr).await;
    let started = row(&logs, "corpse_timer_started");
    assert_owner(&started);
    assert_fields(
        &started,
        &[("pet_id", &pet.to_string()), ("corpse_secs", "10")],
    );

    pet_owner_sweep_at(t0 + PET_CORPSE_DESPAWN, &tx, &mut mgr).await;
    assert_owner(&row(&logs, "corpse_expired"));
    let gone = row(&logs, "despawned");
    assert_owner(&gone);
    assert_fields(&gone, &[("reason", "corpse_expired"), ("path", "sweep")]);
}
