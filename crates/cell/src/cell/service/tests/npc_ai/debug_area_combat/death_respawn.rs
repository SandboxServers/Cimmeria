//! Z9, the death and respawn test around respawner 131, and the navmesh
//! placement of every DA-04 spawn.

use super::*;

const OPERATIVE: &str = "DebugArea_Death_Operative";
const RESPAWN_TARGET: &str = "DebugArea_Respawn_";
/// Respawners 130 and 131 (`respawners.sql`, DA-01).
const RESPAWNERS: [[f32; 3]; 2] = [[251.0, 8.0, -962.0], [438.0, 10.4, -916.0]];

/// Every DA-04 spawn grounds onto the navmesh with a start polygon under it,
/// so it can path (no `spawn_off_mesh` warning) and stands within 2 u of the
/// height it was authored at. Fails if a row is moved off the mesh, onto a
/// prop, or to a floor the mesh does not cover.
#[tokio::test]
async fn every_da04_spawn_stands_on_the_navmesh() {
    let records = world_records();
    let Some(mgr) = scene(&records) else {
        return;
    };
    let navmesh = navmesh().unwrap();
    let rows = da04(&records, "DebugArea_");
    assert!(rows.len() >= 19, "fixture: DA-04's rows ({})", rows.len());
    for r in rows {
        let pos = mgr.get_entity(npc_id(r)).unwrap().position;
        assert!(
            navmesh.start_poly_snap(&pos).is_some(),
            "{:?} at {pos:?} has no start polygon",
            r.tag
        );
        assert!(
            (pos.y - r.y).abs() <= 2.0,
            "{:?} grounded {:.2} u from its authored y {}",
            r.tag,
            pos.y - r.y,
            r.y
        );
    }
}

/// Where a world-1300 NPC can stand while Idle: its spawn, every point of its
/// patrol path, and eight points on its wander circle (snapped to the mesh).
fn idle_positions(r: &SpawnRecord, navmesh: &NavMesh) -> Vec<Vector3> {
    let spawn = Vector3::new(r.x, r.y, r.z);
    let mut out = vec![spawn];
    out.extend(r.patrol_path.iter().copied());
    if r.wander_radius > 0.0 {
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            out.push(navmesh.get_nearest_point(&Vector3::new(
                r.x + r.wander_radius * a.cos(),
                r.y,
                r.z + r.wander_radius * a.sin(),
            )));
        }
    }
    out
}

/// **Content guard.** A player standing on either Debug Area respawner is
/// never acquired by any world-1300 NPC, from its spawn or from anywhere on
/// its patrol path or wander circle, so a respawn is never a re-death.
/// Fails if the lethal squad (or any later hostile, patroller or wanderer) is
/// moved or given a radius or route that reaches a respawner.
#[tokio::test]
async fn a_player_on_a_respawner_is_never_pulled() {
    let records = world_records();
    let navmesh = navmesh();
    for spot in RESPAWNERS {
        let Some(mut mgr) = scene(&records) else {
            return;
        };
        let navmesh = navmesh.as_ref().unwrap();
        add_player(&mut mgr, PLAYER, spot);
        for r in &records {
            let mut positions = idle_positions(r, navmesh);
            // The grounded spawn, not the authored height.
            positions[0] = mgr.get_entity(npc_id(r)).unwrap().spawn_position.unwrap();
            for pos in positions {
                mgr.update_entity_position(npc_id(r), [pos.x, pos.y, pos.z], [0, 0, 0], [0.0; 3]);
                reset_idle(&mut mgr, [npc_id(r)]);
                let (_, targets) = scan(&mut mgr, npc_id(r)).await;
                assert!(
                    !targets.contains(&PLAYER),
                    "{:?} at {pos:?} pulled a player standing on the respawner at {spot:?}",
                    r.tag
                );
            }
            let home = mgr.get_entity(npc_id(r)).unwrap().spawn_position.unwrap();
            mgr.update_entity_position(npc_id(r), [home.x, home.y, home.z], [0, 0, 0], [0.0; 3]);
            reset_idle(&mut mgr, [npc_id(r)]);
        }
    }
}

/// A tester who walks up to the lethal squad is engaged by the closest
/// operative and the other three join through same-room assist (NA14), so the
/// whole squad fires. The respawn-timer targets (seeded NEUTRAL) never engage
/// a player standing beside them.
#[tokio::test]
async fn the_lethal_squad_engages_together_and_the_respawn_targets_wait() {
    let records = world_records();
    let squad = da04(&records, OPERATIVE);
    assert_eq!(squad.len(), 4, "fixture: four operatives");
    let navmesh = navmesh();
    let Some(mut mgr) = scene(&records) else {
        return;
    };
    let first = squad
        .iter()
        .find(|r| r.tag.as_deref() == Some("DebugArea_Death_Operative2"))
        .unwrap();
    let x = first.x - 8.0;
    let y = navmesh
        .as_ref()
        .and_then(|n| n.get_height_near(x, first.y, first.z))
        .expect("the approach west of the squad is on the mesh");
    add_player(&mut mgr, PLAYER, [x, y, first.z]);
    let (engaged, targets) = scan(&mut mgr, npc_id(first)).await;
    assert!(engaged && targets == vec![PLAYER], "{targets:?}");
    for r in &squad {
        let e = mgr.get_entity(npc_id(r)).unwrap();
        assert!(
            e.threat_list.contains_key(&PLAYER),
            "{:?} must join the fight (assist)",
            r.tag
        );
    }

    for r in da04(&records, RESPAWN_TARGET) {
        let Some(mut mgr) = scene(&records) else {
            return;
        };
        add_player(&mut mgr, PLAYER, [r.x + 2.0, r.y, r.z]);
        let (_, targets) = scan(&mut mgr, npc_id(r)).await;
        assert!(
            !targets.contains(&PLAYER),
            "{:?} is NEUTRAL and must wait to be shot",
            r.tag
        );
    }
}

fn set_health(mgr: &mut SpaceManager, id: u32, cur: i32) {
    mgr.get_entity_mut(id)
        .unwrap()
        .stats
        .get_mut(cimmeria_entity::stats::HEALTH)
        .unwrap()
        .update(0, cur, 760);
}

/// **Content guard: no death loop at respawner 131.** The worst case: the
/// lethal squad chased a fleeing tester right onto the respawner (5 u from it,
/// inside its 50 u leash) and killed him there, and he clicked Release at once,
/// reappearing at 131 with no AI pass in between. The dead player leaves every
/// threat list at death and the squad walks home evading (a Leashing NPC does
/// not scan). Home again, it scans only its 10 u radius, 28-31 u from 131,
/// behind a rise. Over 40 s of AI and movement ticks no operative may take the
/// respawned player back, and every one must end Idle at its spawn.
///
/// Fails if a dead player stops being purged from threat lists, if a squad
/// walking home starts scanning, or if the squad's home scan can reach 131.
/// That reach is radius AND line of sight: with the radius raised to 35 u the
/// test still passes, because the rise between the squad and 131 blocks the
/// occluder ray (measured 2026-10-04); `a_player_on_a_respawner_is_never_pulled`
/// is the static check of the same reach.
#[tokio::test]
async fn a_tester_killed_at_respawner_131_is_not_killed_again_on_respawn() {
    let records = world_records();
    let squad = da04(&records, OPERATIVE);
    let Some(mut mgr) = scene(&records) else {
        return;
    };
    let navmesh = navmesh().unwrap();
    let resp = RESPAWNERS[1];
    add_player(&mut mgr, PLAYER, resp);
    set_health(&mut mgr, PLAYER, 760);
    // The chase ended 5 u east of the respawner, the squad side by side.
    for (i, r) in squad.iter().enumerate() {
        let p = navmesh.get_nearest_point(&Vector3::new(
            resp[0] + 5.0,
            resp[1],
            resp[2] - 3.0 + 2.0 * i as f32,
        ));
        mgr.update_entity_position(npc_id(r), [p.x, p.y, p.z], [0, 0, 0], [0.0; 3]);
        let e = mgr.get_entity_mut(npc_id(r)).unwrap();
        crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
        e.threat_list.insert(PLAYER, 10.0);
    }
    let (tx, _rx) = mpsc::channel(65_536);
    // The kill, then the instant Release: flags cleared, pools full, at 131.
    set_health(&mut mgr, PLAYER, 0);
    assert!(
        crate::cell::abilities::resolve_death_for_test(PLAYER, npc_id(squad[0]), &tx, &mut mgr)
            .await
    );
    mgr.get_entity_mut(PLAYER).unwrap().clear_all_state_flags();
    set_health(&mut mgr, PLAYER, 760);
    mgr.update_entity_position(PLAYER, resp, [0, 0, 0], [0.0; 3]);

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let events = crate::cell::content::EngineEvents(&engine);
    for tick in 0..400 {
        if tick % 10 == 0 {
            crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr, &events).await;
        }
        crate::cell::service::npc_ai::npc_ai_retry_sweep(&tx, &mut mgr, &events).await;
        crate::cell::service::ticks::npc_movement_tick(&mut mgr);
        for r in &squad {
            let e = mgr.get_entity(npc_id(r)).unwrap();
            assert!(
                !e.threat_list.contains_key(&PLAYER),
                "tick {tick}: {:?} took the respawned player back ({:?})",
                r.tag,
                e.ai_state()
            );
        }
    }
    for r in &squad {
        let e = mgr.get_entity(npc_id(r)).unwrap();
        let home = e.spawn_position.unwrap();
        assert_eq!(e.ai_state(), AiState::Idle, "{:?} is home and Idle", r.tag);
        assert!(
            (e.position.x - home.x).hypot(e.position.z - home.z) <= 1.5,
            "{:?} ended at {:?}, home {home:?}",
            r.tag,
            e.position
        );
    }
}
