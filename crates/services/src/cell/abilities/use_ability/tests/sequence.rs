//! NA43: the attack `onSequence` routing and its NPC negative logs.
//!
//! - A player's Ability_End goes to their own client **and** to every other
//!   player who can see them, as Python `AbilityManager.playSequence` did.
//!   Before NA43 it went through `send_entity_method` (player → self only),
//!   so nobody else ever saw a player fire.
//! - An NPC attack that cannot animate WARNs on `abilities.sequence`, once
//!   per ability per window, with `suppressed`.

use std::time::{Duration, Instant};

use tracing::Level;

use super::*;
use crate::cell::abilities::use_ability::sequence::SEQUENCE_WARN_INTERVAL;
use crate::cell::spawner::EVENT_ABILITY_END;
use crate::test_support::{Captured, LogCapture};

const SHOOTER: u32 = 1;
const NPC: u32 = 2;
const ONLOOKER: u32 = 3;
const ABILITY: i32 = 559;
/// Automatic Weapon Auto Attack's event set and its Ability_End sequence
/// (`KIS-SA_Burst_Source`), as seeded.
const EVENT_SET: i32 = 15;
const END_SEQ: i32 = 15;

/// One non-instanced Castle_CellBlock space: players 1 and 3 five units
/// apart, both connected and in each other's AoI, and a hostile NPC 2 ten
/// units in front of player 1. Ability 559 (event set 15 → Ability_End 15)
/// is loaded and known by player 1.
fn scene() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    for (id, pos) in [(SHOOTER, [0.0, 0.0, 0.0]), (ONLOOKER, [5.0, 0.0, 0.0])] {
        mgr.create_entity(id, "Castle_CellBlock", pos, [0.0; 3])
            .unwrap();
        let p = mgr.get_entity_mut(id).unwrap();
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
        mgr.connect_entity(id);
    }
    mgr.get_entity_mut(SHOOTER)
        .unwrap()
        .abilities
        .add_ability(ABILITY);
    mgr.create_entity(NPC, "Castle_CellBlock", [0.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(NPC).unwrap().faction = crate::cell::combat::HOSTILE_FACTION;
    let mut def = make_ability(ABILITY, 0, 40);
    def.event_set_id = Some(EVENT_SET);
    mgr.ability_defs.insert(ABILITY, def);
    mgr.sequence_map
        .insert((EVENT_SET, EVENT_ABILITY_END), END_SEQ);
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_witnesses_of(SHOOTER).contains(&ONLOOKER),
        "fixture: the onlooker must have the shooter in AoI"
    );
    mgr
}

fn first_u32(args: &[u8]) -> i32 {
    i32::from_le_bytes(args[..4].try_into().unwrap())
}

/// §26-style fan-out guard for players (NA43, handoff §13). Player 1 fires
/// 559 at the NPC: their own client gets the self `EntityMethodCall`, and
/// player 3, standing next to them, gets exactly one `WitnessEntityMethod`
/// for method 1 whose sequence id is Ability_End 15, source player 1,
/// flagged as a player ghost so the base encodes it under the SGWPlayer
/// idbase.
///
/// Reverting the send to `send_entity_method` (player → self only) leaves
/// player 3 with nothing and fails the witness assertion.
#[tokio::test]
async fn a_players_shot_is_seen_by_a_second_player_standing_next_to_them() {
    let mut mgr = scene();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(SHOOTER, ABILITY, NPC as i32, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);

    let own: Vec<&Vec<u8>> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: SHOOTER,
                method_index: method_idx::ON_SEQUENCE,
                args,
            } => Some(args),
            _ => None,
        })
        .collect();
    assert_eq!(own.len(), 1, "the shooter's own client plays the shot once");
    assert_eq!(first_u32(own[0]), END_SEQ);

    let seen: Vec<(&Vec<u8>, bool)> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: ONLOOKER,
                entity_id: SHOOTER,
                method_index: method_idx::ON_SEQUENCE,
                args,
                entity_is_player,
            } => Some((args, *entity_is_player)),
            _ => None,
        })
        .collect();
    assert_eq!(
        seen.len(),
        1,
        "the second player must see the shot exactly once; zero means the onSequence \
         went to the shooter's client only: {msgs:#?}"
    );
    let (args, entity_is_player) = seen[0];
    assert_eq!(args.len(), 26, "onSequence args are 26 bytes");
    assert_eq!(first_u32(args), END_SEQ, "Ability_End sequence id first");
    assert_eq!(
        i32::from_le_bytes(args[4..8].try_into().unwrap()),
        SHOOTER as i32,
        "SourceID is the shooter"
    );
    assert_eq!(
        i32::from_le_bytes(args[8..12].try_into().unwrap()),
        NPC as i32,
        "TargetID is the NPC"
    );
    assert!(
        entity_is_player,
        "a player ghost's method must be encoded under the SGWPlayer idbase"
    );
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::WitnessEntityMethod {
                witness_id: SHOOTER,
                method_index: method_idx::ON_SEQUENCE,
                ..
            }
        )),
        "the shooter is not its own witness; a second copy would double the animation"
    );
}

/// Make NPC 2 know `ability_id` and give it a target (player 1).
fn arm_npc(mgr: &mut SpaceManager, npc: u32, ability_id: i32, event_set_id: Option<i32>) {
    if mgr.get_entity(npc).is_none() {
        mgr.create_entity(npc, "Castle_CellBlock", [0.0, 0.0, 10.0], [0.0; 3])
            .unwrap();
    }
    mgr.get_entity_mut(npc)
        .unwrap()
        .abilities
        .add_ability(ability_id);
    let mut def = make_ability(ability_id, 0, 40);
    def.event_set_id = event_set_id;
    mgr.ability_defs.insert(ability_id, def);
}

async fn npc_fires(mgr: &mut SpaceManager, npc: u32, ability_id: i32) {
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        handle_use_ability(npc, ability_id, SHOOTER as i32, &tx, mgr).await,
        "control: the NPC's cast commits"
    );
}

fn sequence_warns(all: &[Captured], outcome: &str) -> Vec<Captured> {
    all.iter()
        .filter(|c| {
            c.level == Level::WARN
                && c.target == "abilities.sequence"
                && c.has_field("outcome", outcome)
        })
        .cloned()
        .collect()
}

/// §26 test 18's runtime half. An NPC firing an ability whose event set is
/// NULL deals damage and plays nothing; that used to log nothing at all. It
/// now WARNs `abilities.sequence outcome=no_event_set` naming the ability.
///
/// Deleting the WARN arm in `sequence.rs` fails this.
#[tokio::test]
async fn an_npc_attack_with_no_event_set_warns() {
    let mut mgr = scene();
    arm_npc(&mut mgr, NPC, 9001, None);
    let logs = LogCapture::install();

    npc_fires(&mut mgr, NPC, 9001).await;

    let warns = sequence_warns(&logs.all(), "no_event_set");
    assert_eq!(warns.len(), 1, "{:#?}", logs.all());
    assert!(warns[0].has_field("ability_id", "9001"), "{:?}", warns[0]);
    assert!(warns[0].has_field("source_id", "2"), "{:?}", warns[0]);
    assert!(warns[0].has_field("suppressed", "0"), "{:?}", warns[0]);
}

/// The second way an NPC attack goes unanimated: the event set exists but
/// has no Ability_End (1001) sequence. It was a DEBUG with no target.
#[tokio::test]
async fn an_npc_attack_with_no_ability_end_sequence_warns() {
    let mut mgr = scene();
    arm_npc(&mut mgr, NPC, 9002, Some(424_242));
    let logs = LogCapture::install();

    npc_fires(&mut mgr, NPC, 9002).await;

    let warns = sequence_warns(&logs.all(), "no_end_sequence");
    assert_eq!(warns.len(), 1, "{:#?}", logs.all());
    assert!(
        warns[0].has_field("event_set_id", "424242"),
        "{:?}",
        warns[0]
    );
}

/// Control: the WARN is for NPC attackers only. Most player abilities have
/// no event set (1851 of 1886 seeded), and a player firing one must not
/// WARN on every click.
#[tokio::test]
async fn a_player_ability_with_no_event_set_does_not_warn() {
    let mut mgr = scene();
    mgr.ability_defs.get_mut(&ABILITY).unwrap().event_set_id = None;
    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(SHOOTER, ABILITY, NPC as i32, &tx, &mut mgr).await);

    assert!(
        logs.all()
            .iter()
            .all(|c| !(c.level == Level::WARN && c.target == "abilities.sequence")),
        "{:#?}",
        logs.all()
    );
}

/// Pattern D, both guards. The key is the ability, not the NPC:
///
/// 1. **Burst.** A second NPC firing the same broken ability inside the
///    window writes no row; when the window has passed, the next row carries
///    the count elided meanwhile.
/// 2. **Independence.** A different broken ability's first occurrence is not
///    swallowed by the first ability's open window.
#[tokio::test]
async fn the_npc_sequence_warn_is_throttled_per_ability_with_a_suppressed_count() {
    const OTHER_NPC: u32 = 4;
    const THIRD_NPC: u32 = 5;
    let mut mgr = scene();
    arm_npc(&mut mgr, NPC, 9001, None);
    arm_npc(&mut mgr, OTHER_NPC, 9001, None);
    arm_npc(&mut mgr, THIRD_NPC, 9003, None);
    let logs = LogCapture::install();

    npc_fires(&mut mgr, NPC, 9001).await;
    npc_fires(&mut mgr, OTHER_NPC, 9001).await;
    assert_eq!(
        sequence_warns(&logs.all(), "no_event_set").len(),
        1,
        "the second NPC repeats the same seed fact inside the window"
    );

    npc_fires(&mut mgr, THIRD_NPC, 9003).await;
    let warns = sequence_warns(&logs.all(), "no_event_set");
    assert_eq!(
        warns.len(),
        2,
        "a different ability's first WARN still lands"
    );
    assert!(warns[1].has_field("ability_id", "9003"));

    // Wind 9001's window back past the interval: the elided shot is reported
    // on the next row that emits.
    let past = Instant::now()
        .checked_sub(SEQUENCE_WARN_INTERVAL + Duration::from_secs(5))
        .expect("monotonic clock is past the interval");
    mgr.ability_sequence_log.forget(9001);
    assert_eq!(
        mgr.ability_sequence_log
            .admit(9001, "no_event_set", past, SEQUENCE_WARN_INTERVAL),
        Some(0)
    );
    for _ in 0..3 {
        let _ = mgr.ability_sequence_log.admit(
            9001,
            "no_event_set",
            past + Duration::from_secs(1),
            SEQUENCE_WARN_INTERVAL,
        );
    }
    mgr.get_entity_mut(NPC)
        .unwrap()
        .abilities
        .clear_all_cooldowns();
    npc_fires(&mut mgr, NPC, 9001).await;
    let warns = sequence_warns(&logs.all(), "no_event_set");
    assert_eq!(warns.len(), 3, "{warns:#?}");
    assert!(warns[2].has_field("suppressed", "3"), "{:?}", warns[2]);
}
