//! Cell-side guard for `BaseToCellMsg::GmSpawnNpcReady` — the second half of the
//! `gmSpawnByCmd` round-trip. The base resolved a template into a `SpawnRecord`;
//! the cell must allocate an NPC id and drop it into the target space.

use super::*;

/// Build a minimal GM-spawn `SpawnRecord` (spawn_id -1, position from "command").
fn gm_record(template_id: i32, pos: [f32; 3]) -> spawner::SpawnRecord {
    spawner::SpawnRecord {
        spawn_id: -1,
        world_name: "Castle".to_string(),
        x: pos[0],
        y: pos[1],
        z: pos[2],
        heading: 0.0,
        tag: None,
        template_id,
        template_name: "GM Test Mob".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(5),
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
    }
}

/// `GmSpawnNpcReady` must spawn the record's NPC into the given space with the
/// template id + position carried by the record. Reverting the handler's
/// `allocate_npc_id` + `spawn_npc_from_record_in_space` call trips this.
#[tokio::test]
async fn gm_spawn_npc_ready_spawns_record_into_space() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // A caller entity gives us the target space id (same path the GM uses).
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let space_id = mgr.get_entity(1).unwrap().space_id.0 as u32;
    assert!(mgr.all_npc_entity_ids().is_empty(), "no NPCs to start");

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let record = gm_record(4242, [12.0, 0.0, -7.0]);

    handle_base_message(
        BaseToCellMsg::GmSpawnNpcReady {
            record: Box::new(record),
            space_id,
            requester_entity_id: 1,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let npcs = mgr.all_npc_entity_ids();
    assert_eq!(npcs.len(), 1, "exactly one NPC must be spawned");
    let e = mgr.get_entity(npcs[0]).unwrap();
    assert_eq!(
        e.template_id,
        Some(4242),
        "spawned NPC carries the record's template id"
    );
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [12.0, 0.0, -7.0],
        "spawned at the record's position"
    );
    assert!(!e.is_player, "GM spawn is an NPC");

    // Definitive feedback: the cell sends the "spawned npc <id>" line to the
    // requesting GM (entity 1) once the spawn actually takes. This is the
    // cell-side completion of the gmSpawnByCmd round-trip — the optimistic
    // cell-side "requested" line was removed.
    let fb = spawned_feedback_text(&mut rx, 1).expect("GmSpawnNpcReady must feed back to the GM");
    assert!(
        fb.contains("spawned npc") && fb.contains("template 4242"),
        "definitive spawn feedback must name the new npc and template, got: {fb}"
    );
}

/// Pull the decoded text of the first `onPlayerCommunication` (method 28)
/// feedback line addressed to `entity_id`, draining the channel.
fn spawned_feedback_text(rx: &mut mpsc::Receiver<CellToBaseMsg>, entity_id: u32) -> Option<String> {
    std::iter::from_fn(|| rx.try_recv().ok()).find_map(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: e,
            method_index: 28,
            args,
        } if e == entity_id => {
            let spk = u32::from_le_bytes(args[0..4].try_into().ok()?) as usize;
            let off = 4 + spk * 2 + 2; // + flags + channel
            let tlen = u32::from_le_bytes(args[off..off + 4].try_into().ok()?) as usize;
            let units: Vec<u16> = (0..tlen)
                .map(|i| u16::from_le_bytes([args[off + 4 + i * 2], args[off + 5 + i * 2]]))
                .collect();
            Some(String::from_utf16_lossy(&units))
        }
        _ => None,
    })
}
