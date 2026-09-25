//! Idle proximity aggro (NA13, D-NA01/D-NA02) on a meshless space: the
//! effective-aggression admission (override, else faction reaction), the
//! radius and vertical-band gates, and the GM `.aggro off` switch. The real
//! navmesh cases (walls, storeys) are in [`super::aggro_castle`].

use super::make_aggression_fixture;
use cimmeria_entity::cell_entity::{AiState, MobAggression};
use tokio::sync::mpsc;

use crate::cell::combat::HOSTILE_FACTION;
use crate::cell::space_manager::SpaceManager;

const NPC: u32 = 200_001;
const PLAYER: u32 = 1;

async fn tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

fn engaged(mgr: &SpaceManager) -> bool {
    let npc = mgr.get_entity(NPC).unwrap();
    npc.ai_state() == AiState::Fighting && npc.threat_list.contains_key(&PLAYER)
}

/// D-NA01: a faction-10 NPC with no override is HOSTILE to players through
/// the reaction table and aggroes a player 5 u away. Before NA13 nothing
/// seeded `aggression`, so every NID guard stood idle (audit A1/A2).
#[tokio::test]
async fn faction_10_npc_aggroes_without_any_override() {
    let mut mgr = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [5.0, 0.0, 0.0]);
    tick(&mut mgr).await;
    assert!(engaged(&mgr), "faction-derived HOSTILE must engage");
}

/// Friendly (faction 1) and undefined (faction 0) NPCs never engage on
/// sight, so they stay Idle beside a player.
#[tokio::test]
async fn friendly_and_neutral_factions_stay_idle() {
    for faction in [1u8, 0u8] {
        let mut mgr = make_aggression_fixture(NPC, faction, PLAYER, [5.0, 0.0, 0.0]);
        tick(&mut mgr).await;
        let npc = mgr.get_entity(NPC).unwrap();
        assert_eq!(npc.ai_state(), AiState::Idle, "faction {faction}");
        assert!(npc.threat_list.is_empty(), "faction {faction}");
    }
}

/// D-NA01a: a seeded NEUTRAL override keeps a faction-10 guard passive (the
/// chain-armed spawns 10 and 20); the chain's HOSTILE override then arms it.
#[tokio::test]
async fn neutral_override_holds_until_the_chain_sets_hostile() {
    let mut mgr = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().aggro.override_level = Some(MobAggression::Neutral);
    tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);

    mgr.get_entity_mut(NPC).unwrap().aggro.override_level = Some(MobAggression::Hostile);
    tick(&mut mgr).await;
    assert!(engaged(&mgr), "the chain's HOSTILE override arms it");
}

/// A HOSTILE override arms a mob whose faction would not (faction 1).
#[tokio::test]
async fn hostile_override_arms_a_friendly_faction() {
    let mut mgr = make_aggression_fixture(NPC, 1, PLAYER, [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().aggro.override_level = Some(MobAggression::Hostile);
    tick(&mut mgr).await;
    assert!(engaged(&mgr));
}

/// A5: only level 1 aggroes. The pre-NA13 rule was `aggression > 0`, which
/// read SUSPICIOUS..DEFAULT (2-5) as hostile too.
#[tokio::test]
async fn levels_two_to_five_do_not_aggro() {
    for level in [
        MobAggression::Suspicious,
        MobAggression::Neutral,
        MobAggression::Friendly,
        MobAggression::Default,
    ] {
        let mut mgr = make_aggression_fixture(NPC, 1, PLAYER, [5.0, 0.0, 0.0]);
        mgr.get_entity_mut(NPC).unwrap().aggro.override_level = Some(level);
        tick(&mut mgr).await;
        assert_eq!(
            mgr.get_entity(NPC).unwrap().ai_state(),
            AiState::Idle,
            "{level:?} must not aggro on sight"
        );
    }
}

/// A HOSTILE override on an NPC sharing the player's server faction (0) is
/// still skipped by the `same_faction` gate; the `spawn_entity` executor
/// warns about this shape at spawn time.
#[tokio::test]
async fn same_server_faction_is_skipped() {
    let mut mgr = make_aggression_fixture(NPC, 0, PLAYER, [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().aggro.override_level = Some(MobAggression::Hostile);
    tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
}

/// Radius gate (A4): inside 18 u engages, outside does not, although both
/// players are AoI witnesses (radius 100). Before NA13 any witness engaged.
#[tokio::test]
async fn aggro_radius_gates_on_horizontal_distance() {
    let mut near = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [15.0, 0.0, 0.0]);
    tick(&mut near).await;
    assert!(engaged(&near), "15 u is inside the 18 u default");

    let mut far = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [25.0, 0.0, 0.0]);
    assert!(
        far.get_witnesses_of(NPC).contains(&PLAYER),
        "fixture: the 25 u player is a witness, so only the radius stops it"
    );
    tick(&mut far).await;
    assert_eq!(far.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
}

/// The template's `aggro_radius` replaces the default.
#[tokio::test]
async fn template_aggro_radius_overrides_the_default() {
    let mut mgr = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [25.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().aggro.radius_override = Some(30.0);
    tick(&mut mgr).await;
    assert!(engaged(&mgr), "25 u is inside a 30 u template radius");
}

/// Vertical band: a player 5 u above the NPC (another storey) is not a
/// candidate even at 3 u horizontally; 3 u above still is.
#[tokio::test]
async fn vertical_band_rejects_another_storey() {
    let mut up = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [3.0, 5.0, 0.0]);
    tick(&mut up).await;
    assert_eq!(up.get_entity(NPC).unwrap().ai_state(), AiState::Idle);

    let mut step = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [3.0, 3.0, 0.0]);
    tick(&mut step).await;
    assert!(engaged(&step), "within the 4 u band");
}

/// Dead players are skipped.
#[tokio::test]
async fn dead_player_is_skipped() {
    let mut mgr = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [5.0, 0.0, 0.0]);
    mgr.get_entity_mut(PLAYER).unwrap().state_field |= crate::cell::combat::BSF_DEAD;
    tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
}

/// D-NA02: mobs aggro onto GMs, except while the GM's `.aggro off` switch is
/// set. The switch is honoured only while the entity still has GM access.
#[tokio::test]
async fn gm_is_aggroed_unless_the_switch_is_off() {
    let gm = |off: bool, access_level: u32| {
        let mut mgr = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [5.0, 0.0, 0.0]);
        mgr.get_entity_mut(PLAYER).unwrap().access_level = access_level;
        if off {
            mgr.gm_aggro_off.insert(PLAYER as i32);
        }
        mgr
    };

    let mut on = gm(false, 2);
    tick(&mut on).await;
    assert!(engaged(&on), "a GM is aggroed by default");

    let mut off = gm(true, 2);
    tick(&mut off).await;
    assert_eq!(
        off.get_entity(NPC).unwrap().ai_state(),
        AiState::Idle,
        "`.aggro off` hides the GM from the proximity scan"
    );

    let mut demoted = gm(true, 0);
    tick(&mut demoted).await;
    assert!(
        engaged(&demoted),
        "a stale switch without GM access is ignored"
    );
}

/// NA00 review: the Idle scan is the one real caller of
/// `AggroCause::Proximity`. Driven through the dispatcher, its entry into
/// Fighting must say `cause=proximity` on the aggro row and
/// `reason=auto_aggro` on the transition row.
#[tokio::test]
async fn idle_auto_aggro_logs_the_proximity_cause() {
    let mut mgr = make_aggression_fixture(NPC, HOSTILE_FACTION, PLAYER, [5.0, 0.0, 0.0]);
    let logs = crate::test_support::LogCapture::install();

    tick(&mut mgr).await;

    let all = logs.all();
    let acquired = all
        .iter()
        .find(|c| c.target == "npc_ai.aggro")
        .expect("auto-aggro must log npc_ai.aggro");
    assert!(acquired.has_field("cause", "proximity"), "{acquired:?}");
    let transition = all
        .iter()
        .find(|c| c.target == "npc_ai.transition" && c.has_field("to", "fighting"))
        .expect("and an Idle -> Fighting transition");
    assert!(
        transition.has_field("reason", "auto_aggro"),
        "{transition:?}"
    );
}
