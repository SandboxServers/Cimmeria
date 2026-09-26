//! NA43, handoff §26 test 20: an NPC's attack animation reaches every player
//! watching it.
//!
//! The fight tick fires through `handle_use_ability`, which sends the
//! Ability_End `onSequence` (client method 1). An NPC has no client, so the
//! animation exists on the wire only as one `WitnessEntityMethod` per AoI
//! witness. Nothing drove the real fight tick and read that packet before;
//! the only neighbouring guard (`stop_hygiene`) asserts the *absence* of a
//! one-byte method 1.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::EVENT_ABILITY_END;
use crate::mercury::method_idx::ON_SEQUENCE;
use crate::test_support::LogCapture;
use tokio::sync::mpsc;

const NPC: u32 = 200;
const TARGET: u32 = 101;
const BYSTANDER: u32 = 102;
/// Pistol Shot's seeded event set and its Ability_End sequence
/// (`KIS-SA_Sing_Source`).
const EVENT_SET: i32 = 3;
const END_SEQ: i32 = 3;

fn add_player(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.create_entity(id, "Castle", pos, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(id).unwrap();
    p.is_player = true;
    p.player_id = Some(id as i32);
    if let Some(h) = p.stats.get_mut(HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    mgr.connect_entity(id);
}

/// A Fighting NPC at the origin with Pistol Shot (event set 3 → Ability_End
/// 3), its target 10 u away and a bystander 12 u away, both in AoI.
fn fixture() -> SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, 0, 30);
    mgr.ability_defs
        .get_mut(&crate::cell::combat::NPC_DEFAULT_ABILITY)
        .unwrap()
        .event_set_id = Some(EVENT_SET);
    mgr.sequence_map
        .insert((EVENT_SET, EVENT_ABILITY_END), END_SEQ);
    add_player(&mut mgr, TARGET, [10.0, 0.0, 0.0]);
    add_player(&mut mgr, BYSTANDER, [0.0, 0.0, 12.0]);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.abilities
        .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    npc.threat_list.insert(TARGET, 10.0);
    let _ = mgr.compute_aoi_changes();
    let mut witnesses = mgr.get_witnesses_of(NPC);
    witnesses.sort_unstable();
    assert_eq!(
        witnesses,
        vec![TARGET, BYSTANDER],
        "fixture: both players see the NPC"
    );
    mgr
}

/// §26 test 20. One fight tick: each of the two witnesses gets exactly one
/// method-1 `WitnessEntityMethod` for the NPC, 26 bytes, whose first `u32` is
/// the Ability_End sequence id and whose SourceID is the NPC, flagged as an
/// NPC ghost. The `abilities.sequence` `ability_end` row reports
/// `witness_count = 2`.
///
/// Removing the Ability_End send, or routing it to anything that skips the
/// witnesses, fails the per-witness count; dropping the field from the row
/// fails the log assertion.
#[tokio::test]
async fn an_npc_fight_tick_sends_each_witness_the_ability_end_sequence() {
    let mut mgr = fixture();
    let (tx, mut rx) = mpsc::channel(512);
    let logs = LogCapture::install();

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let mut msgs = Vec::new();
    while let Ok(m) = rx.try_recv() {
        msgs.push(m);
    }
    for witness in [TARGET, BYSTANDER] {
        let got: Vec<(&Vec<u8>, bool)> = msgs
            .iter()
            .filter_map(|m| match m {
                CellToBaseMsg::WitnessEntityMethod {
                    witness_id,
                    entity_id: NPC,
                    method_index: ON_SEQUENCE,
                    args,
                    entity_is_player,
                } if *witness_id == witness => Some((args, *entity_is_player)),
                _ => None,
            })
            .collect();
        assert_eq!(
            got.len(),
            1,
            "witness {witness} must get the NPC's attack animation exactly once: {msgs:#?}"
        );
        let (args, entity_is_player) = got[0];
        assert_eq!(args.len(), 26, "onSequence args are 26 bytes");
        assert_eq!(
            i32::from_le_bytes(args[0..4].try_into().unwrap()),
            END_SEQ,
            "first u32 is the Ability_End sequence id, not the event set id"
        );
        assert_eq!(
            i32::from_le_bytes(args[4..8].try_into().unwrap()),
            NPC as i32,
            "SourceID is the NPC"
        );
        assert_eq!(
            i32::from_le_bytes(args[8..12].try_into().unwrap()),
            TARGET as i32,
            "TargetID is the player the NPC shot"
        );
        assert!(!entity_is_player, "an NPC ghost uses the NPC idbase");
    }

    let row = logs
        .all()
        .into_iter()
        .find(|c| c.target == "abilities.sequence" && c.has_field("event", "ability_end"))
        .unwrap_or_else(|| panic!("an ability_end row: {:#?}", logs.all()));
    assert!(row.has_field("witness_count", "2"), "{row:?}");
    assert!(row.has_field("sequence_id", "3"), "{row:?}");
}
