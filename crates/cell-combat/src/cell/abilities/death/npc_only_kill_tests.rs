//! NPC-vs-NPC (#1009): what a kill pays when the killer is a plain NPC.
//!
//! An NPC-only kill pays nobody: no loot on the corpse, no `GrantXP`, and
//! (tested at the fight-tick level in `service::tests::npc_ai::npc_vs_npc`)
//! no mission `EntityDeath`. A player's kill of the same mob still pays in
//! full, which is the control that proves the fixture's loot table drops.
//! The dead NPC also leaves every other NPC's threat list at the moment of
//! death, as a dead player always has (NA24).

use super::*;
use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::spawner::LootTableEntry;
use crate::test_support::LogCapture;
use cimmeria_entity::cell_entity::AiState;

use super::super::loot_drop::INT_NORMAL_LOOT;

const PLAYER: u32 = 7;
const LOOT_TABLE: i32 = 0x7000_1009;
/// Praxis: HOSTILE to faction 10 in the reaction table, and 10 to it.
const FRIENDLY_FACTION: u8 = 3;

/// A player, a friendly (faction 3) NPC and a hostile (faction 10) level-5
/// NPC whose loot table always drops. Returns `(mgr, friendly, hostile)`.
fn world() -> (SpaceManager, u32, u32) {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER, "Castle", [0.0, 0.0, -10.0], [0.0; 3])
        .unwrap();
    {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.is_player = true;
        p.player_id = Some(700);
    }
    let friendly = mgr.allocate_npc_id();
    mgr.spawn_npc(friendly, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    {
        let f = mgr.get_entity_mut(friendly).unwrap();
        f.faction = FRIENDLY_FACTION;
        f.tag = Some("Castle_Standoff_Test_Friendly".to_string());
    }
    let hostile = mgr.allocate_npc_id();
    mgr.spawn_npc(hostile, "Castle", [6.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let h = mgr.get_entity_mut(hostile).unwrap();
        h.faction = HOSTILE_FACTION;
        h.level = 5;
        h.tag = Some("Castle_Test_Guard".to_string());
        h.loot_table_id = Some(LOOT_TABLE);
    }
    mgr.loot_tables.insert(
        LOOT_TABLE,
        vec![LootTableEntry {
            design_id: Some(7),
            min_quantity: 1,
            max_quantity: 1,
            probability: 1.0,
        }],
    );
    (mgr, friendly, hostile)
}

/// Kill `target` as `attacker` through the real resolver with XP enabled
/// (what combat and DoT kills pass), and return every `GrantXP` recipient.
async fn kill(mgr: &mut SpaceManager, target: u32, attacker: u32) -> Vec<u32> {
    let (tx, mut rx) = mpsc::channel(512);
    let attacker_is_player = mgr.get_entity(attacker).is_some_and(|a| a.is_player);
    assert!(
        kill_npc_out_of_band(target, attacker, attacker_is_player, true, &tx, mgr).await,
        "fixture: the kill must resolve"
    );
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        if let CellToBaseMsg::GrantXP { entity_id, .. } = m {
            out.push(entity_id);
        }
    }
    out
}

/// **Guard.** A friendly NPC's kill leaves no loot on the corpse and pays no
/// XP. With the loot gate reverted the corpse carries the always-dropping
/// row and the loot cursor.
#[tokio::test]
async fn an_npc_only_kill_rolls_no_loot_and_pays_no_xp() {
    let (mut mgr, friendly, hostile) = world();
    let grants = kill(&mut mgr, hostile, friendly).await;
    assert!(grants.is_empty(), "an NPC kill pays nobody: {grants:?}");
    let corpse = mgr.get_entity(hostile).unwrap();
    assert!(corpse.loot.is_empty(), "no loot for an NPC-only kill");
    assert_eq!(corpse.interaction_type_flags & INT_NORMAL_LOOT, 0);
    assert_eq!(corpse.ai_state(), AiState::Dead, "the death itself happens");
}

/// Control: the same mob killed by the player rolls its loot and pays the
/// player. Pins that the fixture's table really drops, so the guard above
/// cannot pass on an empty table.
#[tokio::test]
async fn a_player_kill_of_the_same_mob_still_pays() {
    let (mut mgr, _friendly, hostile) = world();
    let grants = kill(&mut mgr, hostile, PLAYER).await;
    assert_eq!(grants, vec![PLAYER]);
    let corpse = mgr.get_entity(hostile).unwrap();
    assert_eq!(corpse.loot.len(), 1);
    assert_ne!(corpse.interaction_type_flags & INT_NORMAL_LOOT, 0);
}

/// The skipped roll is not silent: one `loot.drop event=skipped
/// reason=npc_only_kill` row with both entities and both factions.
#[tokio::test]
async fn an_npc_only_kill_says_why_nothing_dropped() {
    let (mut mgr, friendly, hostile) = world();
    let logs = LogCapture::install();
    let _ = kill(&mut mgr, hostile, friendly).await;
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "loot.drop" && c.has_field("event", "skipped"))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row.level, tracing::Level::INFO);
    for (k, v) in [
        ("reason", "npc_only_kill"),
        ("target_eid", hostile.to_string().as_str()),
        ("attacker_id", friendly.to_string().as_str()),
        ("target_faction", "10"),
        ("attacker_faction", "3"),
        ("target_tag", "Castle_Test_Guard"),
    ] {
        assert!(row.has_field(k, v), "field {k}={v} missing: {row:?}");
    }
}

/// A dead NPC leaves every other NPC's threat list at the moment of death,
/// and the player on the same list stays. With the purge reverted the corpse
/// stays on the friendly's list until its next fight pass.
#[tokio::test]
async fn a_dead_npc_leaves_every_threat_list_at_once() {
    let (mut mgr, friendly, hostile) = world();
    let third = mgr.allocate_npc_id();
    mgr.spawn_npc(third, "Castle", [3.0, 0.0, 3.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(third).unwrap().faction = FRIENDLY_FACTION;
    for npc in [friendly, third] {
        let e = mgr.get_entity_mut(npc).unwrap();
        e.threat_list.insert(hostile, 50.0);
        e.threat_list.insert(PLAYER, 5.0);
        crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
    }
    let _ = kill(&mut mgr, hostile, friendly).await;
    for npc in [friendly, third] {
        let e = mgr.get_entity(npc).unwrap();
        assert!(
            !e.threat_list.contains_key(&hostile),
            "NPC {npc} still lists the corpse"
        );
        assert!(e.threat_list.contains_key(&PLAYER), "the player stays");
        assert_eq!(e.ai_state(), AiState::Fighting, "the fight goes on");
    }
}

/// A pet is not a plain NPC: its kill is its owner's and rolls loot
/// (pets PT-06), so [`npc_only_kill`] must not match it.
#[test]
fn a_pet_killer_is_not_an_npc_only_kill() {
    let (mut mgr, friendly, _hostile) = world();
    assert!(npc_only_kill(&mgr, friendly));
    assert!(!npc_only_kill(&mgr, PLAYER));
    assert!(!npc_only_kill(&mgr, 424_242), "a vanished killer");
    mgr.get_entity_mut(friendly).unwrap().extensions.insert(
        cimmeria_entity::cell_entity::PetState::new(PLAYER, vec![], 0b111, 0),
    );
    assert!(!npc_only_kill(&mgr, friendly));
}
