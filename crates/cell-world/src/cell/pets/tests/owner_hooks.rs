//! PT-02 owner lifecycle (A-31): the two choke points every owner path
//! calls, and the pet corpse timer (D-PT08).

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::cell::pets::{
    on_owner_left, on_owner_teleported, pet_owner_sweep_at, OwnerPath, PetDespawnReason,
    PET_CORPSE_DESPAWN, PET_SPAWN_OFFSET,
};
use crate::test_fixtures::{
    assert_pet_fully_gone, drain_entity_moved_for, drain_left_aoi_for, watched_pet_world,
    PET_FIXTURE_OTHER as OTHER, PET_FIXTURE_OWNER as OWNER,
};
use cimmeria_wire::state_field::BSF_DEAD;

// ---- despawn --------------------------------------------------------------

/// Travel (gate, transfer, ring, cross-world respawn): the pet is despawned
/// before the owner goes, the other player sees it leave, and the traveller
/// does NOT get a `LeftAoI` that would land behind its `RESET_ENTITIES`.
#[tokio::test]
async fn owner_travel_despawns_the_pet_and_spares_the_owner_the_left_aoi() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(64);

    let n = on_owner_left(
        OWNER,
        PetDespawnReason::OwnerLeftSpace,
        OwnerPath::GateTravel,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(n, 1);
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
    assert!(
        mgr.get_entity(OWNER).is_some(),
        "the owner itself is untouched"
    );
}

/// Owner death (D-PT08): the owner's client stays, so it sees the pet go too.
#[tokio::test]
async fn owner_death_despawns_the_pet_and_the_owner_sees_it_go() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(64);

    on_owner_left(
        OWNER,
        PetDespawnReason::OwnerDead,
        OwnerPath::GateTravel,
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OWNER, OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}

/// Negative: a player with no pet is a no-op, and another owner's pet is
/// untouched.
#[tokio::test]
async fn owner_left_touches_only_that_owners_pets() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, mut rx) = mpsc::channel(64);

    assert_eq!(
        on_owner_left(
            OTHER,
            PetDespawnReason::OwnerLeftSpace,
            OwnerPath::GateTravel,
            &tx,
            &mut mgr
        )
        .await,
        0
    );
    assert!(drain_left_aoi_for(&mut rx, pet).is_empty());
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);
}

// ---- teleport with owner --------------------------------------------------

/// A same-space owner teleport: the pet lands `PET_SPAWN_OFFSET` behind the
/// owner's new spot, stopped, and every witness that could see it gets an
/// `EntityMoved` to that spot now. No `TeleportPlayer` is ever sent for a
/// pet.
#[tokio::test]
async fn owner_teleport_moves_the_pet_beside_the_owner() {
    let (mut mgr, pet) = watched_pet_world();
    // Owner faces +x (yaw pi/2), so "behind" is -x.
    let yaw = std::f32::consts::FRAC_PI_2;
    mgr.get_entity_mut(OWNER).unwrap().direction = cimmeria_common::Vector3::new(0.0, yaw, 0.0);
    mgr.update_position_preserving_facing(OWNER, [300.0, 5.0, -40.0], [0.0; 3]);
    let (tx, mut rx) = mpsc::channel(64);

    let moved = on_owner_teleported(OWNER, OwnerPath::ContentTeleport, &tx, &mut mgr).await;

    assert_eq!(moved, 1);
    let expect = [300.0 - PET_SPAWN_OFFSET, 5.0, -40.0];
    let p = mgr.get_entity(pet).unwrap();
    for (got, want) in [p.position.x, p.position.y, p.position.z]
        .iter()
        .zip(expect)
    {
        assert!((got - want).abs() < 1e-4, "pet at {:?}", p.position);
    }
    assert!(p.nav_path.is_empty(), "the pet is stopped");
    assert_eq!(p.velocity, [0.0; 3]);
    assert!((p.direction.y - yaw).abs() < 1e-6, "faces the owner's way");
    assert!(p.pet.as_ref().unwrap().last_teleport_at.is_some());
    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet], "still owned");

    let moves = drain_entity_moved_for(&mut rx, pet);
    let witnesses: Vec<u32> = moves.iter().map(|m| m.0).collect();
    assert_eq!(witnesses, vec![OWNER, OTHER]);
    for (_, pos, vel) in moves {
        for (got, want) in pos.iter().zip(expect) {
            assert!((got - want).abs() < 1e-4, "relayed {pos:?}");
        }
        assert_eq!(vel, [0.0; 3]);
    }
}

/// On a navmesh the pet stands on the floor, not at the owner's height: an
/// owner snapped 2 u above the Cellblock floor (a GM `gmGotoXYZ`, a ring pad
/// origin) must not leave its pet hanging in the air.
#[tokio::test]
async fn owner_teleport_grounds_the_pet_on_the_navmesh() {
    use crate::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
    let Some(mesh) = test_fixture_mesh() else {
        return;
    };
    let mut mgr = crate::test_fixtures::make_pet_world();
    test_insert_navmesh_space(&mut mgr, "CellblockNav", mesh);
    // The NA11 guard spawn: on the fixture mesh, floor Y ~68.5.
    let floor = [-289.465, 68.542, -154.276];
    crate::test_fixtures::add_pet_owner(&mut mgr, OWNER, "CellblockNav", floor, 12);
    let pet = mgr
        .spawn_pet_from_template(OWNER, crate::test_fixtures::PET_FIXTURE_TEMPLATE_ID, 0)
        .unwrap();
    mgr.update_position_preserving_facing(OWNER, [floor[0], floor[1] + 2.0, floor[2]], [0.0; 3]);
    let (tx, _rx) = mpsc::channel(64);

    assert_eq!(
        on_owner_teleported(OWNER, OwnerPath::ContentTeleport, &tx, &mut mgr).await,
        1
    );

    let y = mgr.get_entity(pet).unwrap().position.y;
    assert!(
        (y - floor[1]).abs() < 0.75,
        "pet Y {y} should be the floor (~{}), not the owner's {}",
        floor[1],
        floor[1] + 2.0
    );
}

/// Negative: a dead pet stays where it fell; only its corpse timer moves it
/// (out of the world).
#[tokio::test]
async fn owner_teleport_leaves_a_dead_pet_where_it_fell() {
    let (mut mgr, pet) = watched_pet_world();
    mgr.get_entity_mut(pet).unwrap().set_state_flag(BSF_DEAD);
    let before = mgr.get_entity(pet).unwrap().position;
    mgr.update_position_preserving_facing(OWNER, [300.0, 5.0, -40.0], [0.0; 3]);
    let (tx, mut rx) = mpsc::channel(64);

    assert_eq!(
        on_owner_teleported(OWNER, OwnerPath::ContentTeleport, &tx, &mut mgr).await,
        0
    );
    assert_eq!(mgr.get_entity(pet).unwrap().position, before);
    assert!(drain_entity_moved_for(&mut rx, pet).is_empty());
}

/// Entity ids are reused: the owner is destroyed, a different player gets
/// its id and summons a pet of its own, then is teleported before the sweep
/// runs. Only the new holder's pet follows it; the old pet belongs to a gone
/// summoner and is despawned, never pulled after the id's new holder.
#[tokio::test]
async fn owner_teleport_never_pulls_a_pet_the_id_holder_did_not_summon() {
    let (mut mgr, old_pet) = watched_pet_world();
    let new_pet = super::reuse_owner_id_then_resummon(&mut mgr);
    let old_spot = mgr.get_entity(old_pet).unwrap().position;
    mgr.update_position_preserving_facing(OWNER, [300.0, 5.0, -40.0], [0.0; 3]);
    let (tx, mut rx) = mpsc::channel(64);

    let moved = on_owner_teleported(OWNER, OwnerPath::ContentTeleport, &tx, &mut mgr).await;

    assert_eq!(moved, 1, "only the new holder's own pet moves");
    assert!(
        drain_entity_moved_for(&mut rx, old_pet).is_empty(),
        "the old pet was not relayed to the new holder's spot (it sat at {old_spot:?})"
    );
    assert!(
        mgr.get_entity(old_pet).is_none(),
        "the old pet is despawned"
    );
    assert!(mgr.pets.owner_of(old_pet).is_none());
    assert_eq!(mgr.pets.pets_of(OWNER), vec![new_pet]);
    let p = mgr.get_entity(new_pet).unwrap().position;
    assert!(
        (p.x - 300.0).abs() <= PET_SPAWN_OFFSET + 1e-3,
        "new pet at {p:?}"
    );
}

// ---- pet corpse -----------------------------------------------------------

/// D-PT08: a dead pet with a live owner is a corpse for `PET_CORPSE_DESPAWN`
/// (10 s), then despawns with `LeftAoI` to everyone who saw it, the owner
/// included.
#[tokio::test]
async fn dead_pet_corpse_despawns_after_ten_seconds() {
    assert_eq!(PET_CORPSE_DESPAWN, Duration::from_secs(10));
    let (mut mgr, pet) = watched_pet_world();
    mgr.get_entity_mut(pet).unwrap().set_state_flag(BSF_DEAD);
    let (tx, mut rx) = mpsc::channel(64);
    let t0 = Instant::now();

    assert_eq!(pet_owner_sweep_at(t0, &tx, &mut mgr).await, 0);
    assert_eq!(
        mgr.get_entity(pet)
            .unwrap()
            .pet
            .as_ref()
            .unwrap()
            .despawn_at,
        Some(t0 + PET_CORPSE_DESPAWN),
        "the first sweep after the death starts the timer"
    );
    let almost = t0 + PET_CORPSE_DESPAWN - Duration::from_millis(1);
    assert_eq!(pet_owner_sweep_at(almost, &tx, &mut mgr).await, 0);
    assert!(
        mgr.get_entity(pet).is_some(),
        "still a corpse just before 10 s"
    );
    assert!(drain_left_aoi_for(&mut rx, pet).is_empty());

    assert_eq!(
        pet_owner_sweep_at(t0 + PET_CORPSE_DESPAWN, &tx, &mut mgr).await,
        1
    );
    assert_eq!(drain_left_aoi_for(&mut rx, pet), vec![OWNER, OTHER]);
    assert_pet_fully_gone(&mgr, OWNER, pet);
}

/// Negative: a living pet gets no corpse timer however long the sweep runs.
#[tokio::test]
async fn living_pet_gets_no_corpse_timer() {
    let (mut mgr, pet) = watched_pet_world();
    let (tx, _rx) = mpsc::channel(64);
    let t0 = Instant::now();
    pet_owner_sweep_at(t0, &tx, &mut mgr).await;
    pet_owner_sweep_at(t0 + Duration::from_secs(60), &tx, &mut mgr).await;
    let p = mgr.get_entity(pet).expect("still here");
    assert_eq!(p.pet.as_ref().unwrap().despawn_at, None);
}

// ---- wiring ---------------------------------------------------------------

/// Every non-test cell source file that sends a player out of its space
/// (`CellToBaseMsg::GateTravel`) must call `pets::on_owner_left`, and every
/// one that snaps a player within it (`CellToBaseMsg::TeleportPlayer`) must
/// call `pets::on_owner_teleported`. A new travel path that forgets its pets
/// fails here instead of leaving them to the one-tick sweep (or, for a
/// same-space move, stranded where the owner was).
#[test]
fn every_owner_travel_site_calls_the_pet_hooks() {
    // The movement validator's snap-back puts a player back where it
    // already was; the pet never left it, so there is nothing to move.
    const TELEPORT_EXEMPT: &[&str] = &["cell/src/cell/service/base_messages/movement.rs"];
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut missing = Vec::new();
    let mut seen = (0usize, 0usize);
    for krate in [
        "cell",
        "cell-combat",
        "cell-content",
        "cell-interactions",
        "cell-console",
        "cell-methods",
        "cell-world",
    ] {
        let mut stack = vec![crates_dir.join(krate).join("src")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    // `tests/`, `chain_replay_tests/`, `test_fixtures/`, ...
                    if path
                        .file_name()
                        .is_some_and(|n| !n.to_string_lossy().contains("test"))
                    {
                        stack.push(path);
                    }
                    continue;
                }
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if !name.ends_with(".rs") || name.contains("test") {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .unwrap()
                    .replace("\r\n", "\n");
                // Production code only: stop at an inline test module (a
                // `mod tests;` pointing at a file is skipped with the file).
                let code = text
                    .find("#[cfg(test)]\nmod tests {")
                    .map_or(text.as_str(), |i| &text[..i]);
                let rel = path
                    .strip_prefix(&crates_dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if code.contains("CellToBaseMsg::GateTravel {") {
                    seen.0 += 1;
                    if !code.contains("pets::on_owner_left(") {
                        missing.push(format!("{rel}: GateTravel without on_owner_left"));
                    }
                }
                if code.contains("CellToBaseMsg::TeleportPlayer {") {
                    seen.1 += 1;
                    if !TELEPORT_EXEMPT.contains(&rel.as_str())
                        && !code.contains("pets::on_owner_teleported(")
                    {
                        missing.push(format!("{rel}: TeleportPlayer without on_owner_teleported"));
                    }
                }
            }
        }
    }
    assert!(
        seen.0 >= 6 && seen.1 >= 5,
        "the scan found too few travel sites ({seen:?}); it is not looking where they are"
    );
    assert!(missing.is_empty(), "{missing:#?}");
}
