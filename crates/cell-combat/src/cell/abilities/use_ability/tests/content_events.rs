//! The content events `handle_use_ability_with_kill_credit` raises, and their
//! order (§2E of docs/architecture/services-crate-split.md).
//!
//! The wrapper reaches the content engine only through `ContentEvents`, so a
//! `RecordingContentEvents` sees exactly what production's `EngineEvents`
//! would dispatch. Per cast the order is: the health-below drain, then one
//! `entity_death` for the primary target. The drain goes first so
//! `pct_after` is read as close to the hit as possible, and a killing blow is
//! suppressed inside the drain because the corpse already carries `BSF_DEAD`.
//! The content-side half (which chain each event fires) is pinned in
//! `content::event_dispatch::lifecycle::tests`; these pin the combat half,
//! which the dispatcher tests cannot see.

use crate::cell::combat::{HealthBelowSample, HealthPct, HOSTILE_FACTION};
use crate::cell::content_events::ContentEvents;
use crate::test_support::{RecordedContentEvent, RecordingContentEvents};
use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::stats::HEALTH;

use super::*;

const PLAYER_EID: u32 = 1;
const PLAYER_ID: i32 = 100;
const NPC_EID: u32 = 50;
const NPC_TAG: &str = "Hallway01_Guard";
const ABILITY_ID: i32 = 7;

/// A player at the origin and one tagged hostile NPC five units away at
/// 100/100 health, with an ability dealing `health_damage` per hit.
fn duel(health_damage: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER_EID, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(NPC_EID, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(NPC_EID) {
        npc.faction = HOSTILE_FACTION;
        npc.tag = Some(NPC_TAG.to_string());
        if let Some(stat) = npc.stats.get_mut(HEALTH) {
            stat.update(0, 100, 100);
            stat.clear_dirty();
        }
    }

    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), health_damage.to_string());
    mgr.effect_defs.insert(
        100,
        EffectDef {
            effect_id: 100,
            ability_id: ABILITY_ID,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        ABILITY_ID,
        AbilityDef {
            // No cooldown, so a second cast is judged on its target alone.
            cooldown: 0.0,
            effect_ids: vec![100],
            ..make_ability(ABILITY_ID, 0, 30)
        },
    );
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
        p.abilities.add_ability(ABILITY_ID);
        p.weapon_holstered = false;
    }
    mgr.connect_entity(PLAYER_EID);
    let _ = mgr.compute_aoi_changes();
    mgr
}

fn npc_health(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(NPC_EID)
        .and_then(|e| e.stats.get(HEALTH))
        .map(|s| s.cur)
        .expect("the NPC must still exist with a HEALTH stat")
}

/// The pre-hit sample the damage seam queues for a hit on the full-health
/// NPC.
fn full_health_sample() -> HealthBelowSample {
    HealthBelowSample {
        attacker_entity_id: PLAYER_EID,
        target_entity_id: NPC_EID,
        pct_before: HealthPct(100.0),
    }
}

async fn cast(mgr: &mut SpaceManager, events: &RecordingContentEvents) -> bool {
    let (tx, _rx) = mpsc::channel(128);
    let events: &dyn ContentEvents = events;
    handle_use_ability_with_kill_credit(PLAYER_EID, ABILITY_ID, NPC_EID as i32, events, &tx, mgr)
        .await
}

/// **Order guard.** A killing blow raises the health-below drain first and
/// the death second. Swapping the two calls in `kill_credit.rs` fails this:
/// no dispatcher-level test can, because the real drain drops a lethal hit's
/// sample (`pct_after <= 0`) whichever side of the death it runs on.
#[tokio::test]
async fn a_killing_cast_drains_health_below_then_raises_the_death() {
    let mut mgr = duel(9999);
    let recorder = RecordingContentEvents::new();

    assert!(cast(&mut mgr, &recorder).await, "the cast must commit");
    assert_eq!(npc_health(&mgr), 0, "test fixture: 9999 damage must kill");

    assert_eq!(
        recorder.events(),
        vec![
            RecordedContentEvent::PendingHealthBelow {
                samples: vec![full_health_sample()],
            },
            RecordedContentEvent::EntityDeath {
                killer_entity_id: PLAYER_EID,
                player_id: PLAYER_ID,
                entity_tag: NPC_TAG.to_string(),
            },
        ],
    );
}

/// A wounding hit raises only the drain, carrying the one pre-hit sample.
#[tokio::test]
async fn a_wounding_cast_raises_only_the_health_below_drain() {
    let mut mgr = duel(5);
    let recorder = RecordingContentEvents::new();

    assert!(cast(&mut mgr, &recorder).await, "the cast must commit");
    let hp = npc_health(&mgr);
    assert!(
        hp > 0 && hp < 100,
        "test fixture: the hit must wound, hp {hp}"
    );

    assert_eq!(
        recorder.events(),
        vec![RecordedContentEvent::PendingHealthBelow {
            samples: vec![full_health_sample()],
        }],
    );
}

/// A second cast at the corpse raises no second death: kill credit is paid
/// once per alive-to-dead transition, so a kill-count mission never counts
/// the same corpse twice. The drain still runs after the cast, on an empty
/// queue (a corpse is not sampled).
#[tokio::test]
async fn a_second_cast_at_the_corpse_raises_no_second_death() {
    let mut mgr = duel(9999);
    let recorder = RecordingContentEvents::new();
    cast(&mut mgr, &recorder).await;
    let after_kill = recorder.events().len();

    cast(&mut mgr, &recorder).await;

    let second: Vec<_> = recorder.events().split_off(after_kill);
    assert_eq!(
        second,
        vec![RecordedContentEvent::PendingHealthBelow { samples: vec![] }],
    );
}
