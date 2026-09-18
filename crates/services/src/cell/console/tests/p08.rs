//! P08 — `.spawn` / `.despawn` regression suite
//! (`docs/analysis/legacy-command-parity/handoffs/p08.md`).
//!
//! Legacy source: `deprecated/python/cell/commands/Resource.py`
//! (`spawnEntity` / `despawnEntity`).
//!
//! The guards here are deliberately split by bug shape rather than by command:
//!
//! - **Request shape** — `.spawn` must carry the caller's *exact* position and
//!   facing, and must emit no optimistic success line (legacy printed
//!   `'Spawning entity of type <%s>'` before `createEntity` ever ran).
//! - **Creation result** — feeding the emitted request through the same
//!   `SpawnRecord` → `spawn_npc_from_record_in_space` path the base round-trip
//!   uses must produce exactly one entity, at that position and facing.
//! - **Rejection** — a template id that cannot name a template is refused
//!   cell-side with nothing enqueued.
//! - **Witness fan-out** — `.despawn` must make every observing player receive
//!   `LeftAoI`, not merely drop the entity from the server's maps.
//! - **Player safety** — the NPC-only boundary is asserted twice: once through
//!   the registry target gate, and once directly against
//!   `SpaceManager::despawn_npc` so removing the registry gate alone cannot
//!   open it.

use cimmeria_common::Vector3;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};
use crate::cell::spawner::SpawnRecord;

/// Drain everything the handler pushed, splitting feedback text out from the
/// raw messages so a test can assert on both.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> (Vec<CellToBaseMsg>, Vec<String>) {
    let mut msgs = Vec::new();
    let mut feedback = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            feedback.push(text);
        }
        msgs.push(msg);
    }
    (msgs, feedback)
}

/// The single `GmSpawnNpc` in `msgs`, or `None`. Panics on more than one —
/// "`.spawn` creates exactly one entity" starts with "exactly one request".
fn only_spawn_request(msgs: &[CellToBaseMsg]) -> Option<(u32, i32, u32, String, [f32; 3], f32)> {
    let mut found = None;
    for msg in msgs {
        if let CellToBaseMsg::GmSpawnNpc {
            entity_id,
            template_id,
            space_id,
            world_name,
            position,
            heading,
        } = msg
        {
            assert!(
                found.is_none(),
                "one .spawn must emit exactly one GmSpawnNpc, got a second: {msg:?}"
            );
            found = Some((
                *entity_id,
                *template_id,
                *space_id,
                world_name.clone(),
                *position,
                *heading,
            ));
        }
    }
    found
}

/// The `SpawnRecord` the base builds from a `GmSpawnNpc`: template-derived
/// fields from `resources.entity_templates`, spawn-instance fields (position,
/// heading, world, `spawn_id = -1`) from the command. Mirrors
/// `base::gm_spawn::load_spawn_record_for_template`'s non-DB half so this
/// suite can drive the creation result without a live DB — the DB half's own
/// heading threading is guarded by the live-DB
/// `gm_spawn_resolves_real_template_and_replies` assertion.
fn record_from_request(
    template_id: i32,
    world_name: &str,
    position: [f32; 3],
    heading: f32,
) -> SpawnRecord {
    SpawnRecord {
        spawn_id: -1,
        world_name: world_name.to_string(),
        x: position[0],
        y: position[1],
        z: position[2],
        heading,
        tag: None,
        template_id,
        template_name: "P08 Test Mob".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(3),
        alignment: Some(0),
        faction: Some(1),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: false,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![],
        respawn_secs: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
    }
}

/// Give the GM a distinctive position + facing so "the caller's exact
/// placement" is falsifiable rather than coincidentally zero.
///
/// `direction` is `[pitch, yaw, roll]` (the wire packs
/// `pack_angle(direction[1]) // yaw`; NPC movement writes the same
/// convention directly as `Vector3::new(0.0, yaw, 0.0)`) — yaw lives in
/// `.y`, never derived via `atan2` on the other two components. Pitch/roll
/// are set to distinctive non-zero values here specifically so a
/// regression that reads the wrong component (e.g. `atan2(x, z)`) fails
/// instead of coincidentally matching.
fn place_caller(mgr: &mut SpaceManager, gm: u32) -> ([f32; 3], f32) {
    let pos = [31.5, 4.25, 62.75];
    let dir = Vector3::new(0.37, 1.9106, -0.21);
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.position = Vector3::new(pos[0], pos[1], pos[2]);
        e.direction = dir;
    }
    (pos, dir.y)
}

// ── .spawn ────────────────────────────────────────────────────────────────

/// `.spawn <templateId>` emits exactly one `GmSpawnNpc` carrying the caller's
/// exact position and facing — and **no** feedback line, because at enqueue
/// time nothing is known about whether the template exists or the entity was
/// created. Legacy printed its `'Spawning entity of type <%s>'` line first and
/// so reported success for spawns that never happened.
#[tokio::test]
async fn legacy_p08_spawn_emits_exact_request_and_no_optimistic_feedback() {
    let (mut mgr, gm, _npc) = setup();
    let (pos, heading) = place_caller(&mut mgr, gm);
    let space_id = mgr.get_entity(gm).unwrap().space_id.0 as u32;
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(32);

    handle_console_command(gm, ".spawn 4321", &tx, &mut mgr, &engine).await;

    let (msgs, feedback) = drain(&mut rx);
    let req = only_spawn_request(&msgs).expect(".spawn must emit a GmSpawnNpc");
    assert_eq!(req.0, gm, "request is attributed to the calling GM");
    assert_eq!(req.1, 4321, "template id from the argument");
    assert_eq!(req.2, space_id, "caller's own space");
    assert_eq!(req.3, "Agnos", "caller's own world");
    assert_eq!(req.4, pos, "spawn at the caller's exact position");
    assert_eq!(req.5, heading, "spawn facing the caller's exact heading");
    assert!(
        feedback.is_empty(),
        "enqueueing a spawn must not claim anything happened yet; got {feedback:?}"
    );
}

/// The emitted request, run through the same record → spawn path the base
/// round-trip uses, creates **exactly one** entity at that exact position and
/// facing. `spawn_npc_from_record_into` turns `record.heading` into
/// `direction = (0, heading, 0)`, so the heading is observable on the entity.
#[tokio::test]
async fn legacy_p08_spawn_creates_exactly_one_entity_at_caller_placement() {
    let (mut mgr, gm, _npc) = setup();
    let (pos, heading) = place_caller(&mut mgr, gm);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(32);
    let before = mgr.all_npc_entity_ids().len();

    handle_console_command(gm, ".spawn 4321", &tx, &mut mgr, &engine).await;
    let (msgs, _) = drain(&mut rx);
    let (_, template_id, space_id, world_name, position, req_heading) =
        only_spawn_request(&msgs).expect(".spawn must emit a GmSpawnNpc");

    let record = record_from_request(template_id, &world_name, position, req_heading);
    let new_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record_in_space(new_id, &record, space_id)
        .expect("record spawn must succeed");

    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        before + 1,
        "exactly one entity created"
    );
    let spawned = mgr.get_entity(new_id).expect("new entity is in the space");
    assert_eq!(
        [spawned.position.x, spawned.position.y, spawned.position.z],
        pos,
        "placed at the caller's exact position"
    );
    assert_eq!(
        [
            spawned.direction.x,
            spawned.direction.y,
            spawned.direction.z
        ],
        [0.0, heading, 0.0],
        "facing the caller's exact heading (yaw in direction.y)"
    );
    assert!(!spawned.is_player, "a .spawn'd entity is never a player");
    assert!(
        spawned.spawn_id.is_none(),
        "a command spawn has no spawnlist row — .savespawn is what creates one"
    );
}

/// A non-positive template id can never name a template, so it is refused
/// cell-side: nothing is enqueued and the GM is told why. Reverting the
/// `template_id <= 0` gate lets a doomed round-trip through.
#[tokio::test]
async fn legacy_p08_spawn_rejects_non_positive_template() {
    for arg in ["0", "-5"] {
        let (mut mgr, gm, _npc) = setup();
        place_caller(&mut mgr, gm);
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(32);
        let before = mgr.all_npc_entity_ids().len();

        handle_console_command(gm, &format!(".spawn {arg}"), &tx, &mut mgr, &engine).await;

        let (msgs, feedback) = drain(&mut rx);
        assert!(
            only_spawn_request(&msgs).is_none(),
            ".spawn {arg} must not enqueue a spawn request"
        );
        assert!(
            feedback.iter().any(|f| f.contains("positive template id")),
            ".spawn {arg} must tell the GM why it was refused; got {feedback:?}"
        );
        assert_eq!(
            mgr.all_npc_entity_ids().len(),
            before,
            ".spawn {arg} must not create an entity"
        );
    }
}

/// A non-numeric template id is refused by the shared arg parser before the
/// handler's own gate — still nothing enqueued, still a reason.
#[tokio::test]
async fn legacy_p08_spawn_rejects_malformed_template() {
    let (mut mgr, gm, _npc) = setup();
    place_caller(&mut mgr, gm);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(32);
    let before = mgr.all_npc_entity_ids().len();

    handle_console_command(gm, ".spawn notanumber", &tx, &mut mgr, &engine).await;

    let (msgs, feedback) = drain(&mut rx);
    assert!(
        only_spawn_request(&msgs).is_none(),
        "a malformed templateId must not enqueue a spawn request"
    );
    assert!(
        !feedback.is_empty(),
        "a malformed templateId must produce a rejection line"
    );
    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        before,
        "a malformed templateId must not create an entity"
    );
}

/// A closed cell→base channel must surface as a truthful failure line, not a
/// panic and not silence. The feedback send fails too (same channel), so the
/// assertion is on the handler surviving and enqueuing nothing.
#[tokio::test]
async fn legacy_p08_spawn_survives_closed_channel() {
    let (mut mgr, gm, _npc) = setup();
    place_caller(&mut mgr, gm);
    let engine = ChainEngine::new();
    let (tx, rx) = mpsc::channel(32);
    drop(rx);
    let before = mgr.all_npc_entity_ids().len();

    handle_console_command(gm, ".spawn 4321", &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        before,
        "a dropped request must never create an entity cell-side"
    );
}

// ── .despawn ──────────────────────────────────────────────────────────────

/// Add a second player witness next to the NPC and run one AoI tick so both
/// players are actually witnessing it. Returns the witness's id.
async fn add_witness_and_settle(
    mgr: &mut SpaceManager,
    witness_id: u32,
    npc: u32,
) -> Vec<CellToBaseMsg> {
    let npc_pos = mgr.get_entity(npc).unwrap().position;
    mgr.create_entity(
        witness_id,
        "Agnos",
        [npc_pos.x + 1.0, npc_pos.y, npc_pos.z + 1.0],
        [0.0; 3],
    )
    .unwrap();
    mgr.connect_entity(witness_id);
    mgr.compute_aoi_changes()
}

/// `.despawn` removes the NPC from the space **and** every player currently
/// witnessing it receives `LeftAoI` for it. Server-side removal alone would
/// leave both clients rendering a ghost.
#[tokio::test]
async fn legacy_p08_despawn_notifies_every_witness_and_removes_the_entity() {
    let (mut mgr, gm, npc) = setup();
    let witness = 2u32;
    let enter = add_witness_and_settle(&mut mgr, witness, npc).await;
    // Both players must actually be witnessing the NPC, or the fan-out
    // assertion below would pass vacuously.
    let entered: Vec<u32> = enter
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EnteredAoI {
                witness_id,
                entity_id,
                ..
            } if *entity_id == npc => Some(*witness_id),
            _ => None,
        })
        .collect();
    assert!(
        entered.contains(&gm) && entered.contains(&witness),
        "fixture precondition: both players must witness the NPC, got {entered:?}"
    );

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, ".despawn", &tx, &mut mgr, &engine).await;

    let (msgs, feedback) = drain(&mut rx);
    let mut left: Vec<u32> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::LeftAoI {
                witness_id,
                entity_id,
            } if *entity_id == npc => Some(*witness_id),
            _ => None,
        })
        .collect();
    left.sort_unstable();
    assert_eq!(
        left,
        vec![gm, witness],
        "every witnessing player must receive LeftAoI for the despawned entity"
    );

    assert!(
        mgr.get_entity(npc).is_none(),
        "the despawned entity must be gone from the space"
    );
    assert!(
        !mgr.all_npc_entity_ids().contains(&npc),
        "the despawned entity must be gone from the NPC roster"
    );
    assert!(
        feedback
            .iter()
            .any(|f| f.contains("2 observer(s) notified")),
        "feedback must report the real notified-observer count; got {feedback:?}"
    );
}

/// The witness sets are scrubbed in the same pass as the fan-out, so the next
/// AoI tick has nothing left to diff — no duplicate `LeftAoI`, and no stale
/// entry pointing at a destroyed entity.
#[tokio::test]
async fn legacy_p08_despawn_scrubs_witness_sets_so_the_next_tick_is_quiet() {
    let (mut mgr, gm, npc) = setup();
    let witness = 2u32;
    add_witness_and_settle(&mut mgr, witness, npc).await;

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, ".despawn", &tx, &mut mgr, &engine).await;
    let _ = drain(&mut rx);

    for player in [gm, witness] {
        let w = &mgr.get_entity(player).unwrap().witnesses;
        assert!(
            !w.contains(&cimmeria_common::EntityId(npc as i32)),
            "player {player} still witnesses the destroyed entity"
        );
    }

    let next_tick = mgr.compute_aoi_changes();
    assert!(
        !next_tick.iter().any(|m| matches!(
            m,
            CellToBaseMsg::LeftAoI { entity_id, .. } if *entity_id == npc
        )),
        "the tick after a despawn must not re-emit LeftAoI for the same entity"
    );
}

/// `.despawn` is runtime-only: it never writes a spawnlist row, even for an
/// entity that has one. Deleting the persistent row is `.delspawn`.
#[tokio::test]
async fn legacy_p08_despawn_never_mutates_the_spawnlist() {
    let (mut mgr, gm, npc) = setup();
    if let Some(e) = mgr.get_entity_mut(npc) {
        e.spawn_id = Some(4242);
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    handle_console_command(gm, ".despawn", &tx, &mut mgr, &engine).await;

    let (msgs, _) = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::ExecuteAuthoringSql { .. })),
        "despawn must not emit authoring SQL — the spawnlist row is .delspawn's job"
    );
    assert!(
        mgr.authoring_changes.get(&gm).is_none_or(|v| v.is_empty()),
        "despawn must not buffer any pending seed SQL"
    );
}

/// The registry gate: `.despawn` is `Target::Mob`, so a selected *player*
/// never reaches the handler. Reverting the spec to the legacy
/// `SGWSpawnableEntity` equivalent (`Target::Spawnable`, which matches
/// everything) lets a player through and trips this.
#[tokio::test]
async fn legacy_p08_despawn_registry_gate_rejects_a_player_target() {
    let (mut mgr, gm, _npc) = setup();
    let victim = 2u32;
    mgr.create_entity(victim, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(victim);
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.current_target_id = Some(victim as i32);
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    handle_console_command(gm, ".despawn", &tx, &mut mgr, &engine).await;

    let (msgs, feedback) = drain(&mut rx);
    assert!(
        mgr.get_entity(victim).is_some(),
        ".despawn must never destroy a player's cell entity"
    );
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::LeftAoI { entity_id, .. } if *entity_id == victim
        )),
        "a refused despawn must not fan out a removal for the player"
    );
    assert!(
        feedback.iter().any(|f| f.contains("expected an NPC")),
        "the GM must be told the target type is wrong; got {feedback:?}"
    );
}

/// Defence in depth: the primitive refuses a player on its own, so removing or
/// loosening the registry gate above cannot by itself open the hole. Reverting
/// the `is_player` / `space.players` check in
/// `SpaceManager::despawn_npc` trips this even with the registry untouched.
#[tokio::test]
async fn legacy_p08_despawn_primitive_refuses_a_player_without_the_registry_gate() {
    let (mut mgr, gm, _npc) = setup();
    let (tx, mut rx) = mpsc::channel(64);

    let outcome = mgr.despawn_npc(gm, &tx).await;

    assert_eq!(
        outcome,
        DespawnOutcome::RefusedPlayer,
        "despawn_npc is NPC-only and must refuse a player"
    );
    assert!(
        mgr.get_entity(gm).is_some(),
        "a refused despawn must leave the player entity intact"
    );
    let (msgs, _) = drain(&mut rx);
    assert!(
        msgs.is_empty(),
        "a refused despawn must not emit anything; got {msgs:?}"
    );
}

/// An id that is not in any space reports `NotFound` rather than silently
/// succeeding — the console turns this into a truthful "no longer in a space"
/// line instead of claiming a removal.
#[tokio::test]
async fn legacy_p08_despawn_primitive_reports_not_found() {
    let (mut mgr, _gm, npc) = setup();
    let (tx, mut rx) = mpsc::channel(64);

    assert_eq!(
        mgr.despawn_npc(npc, &tx).await,
        DespawnOutcome::Despawned {
            witnesses_notified: 0
        },
        "no observers yet (no AoI tick has run), so nothing is notified"
    );
    assert_eq!(
        mgr.despawn_npc(npc, &tx).await,
        DespawnOutcome::NotFound,
        "a second despawn of the same id must report NotFound"
    );
    let (msgs, _) = drain(&mut rx);
    assert!(
        msgs.is_empty(),
        "an unwitnessed despawn emits no LeftAoI; got {msgs:?}"
    );
}
