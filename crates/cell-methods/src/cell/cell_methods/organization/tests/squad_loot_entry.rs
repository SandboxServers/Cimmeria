//! CM 18 loot mode (D-ORG16, CAT-M-11) and the world-entry replay.

use cimmeria_entity::organization::{OrgRank, OrgType, SquadLootType, SQUAD_ORG_ID_MIN};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_joined_organization, build_on_organization_joined,
    build_on_organization_roster_info, build_on_squad_loot_type, RosterInfo,
};

use super::*;
use crate::test_support::LogCapture;

const SID: i32 = SQUAD_ORG_ID_MIN;

fn loot(mode: SquadLootType) -> (u16, Vec<u8>) {
    (51, build_on_squad_loot_type(SID, mode))
}

#[tokio::test]
async fn leader_sets_loot_mode_for_everyone() {
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13]);
    squad::set_loot_mode(11, 1, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    for e in 11..=13 {
        assert_eq!(
            to(&sent, e),
            [loot(SquadLootType::FreeForAll)],
            "entity {e}"
        );
    }
    assert_eq!(
        mgr.squads.squad(SID).unwrap().loot(),
        SquadLootType::FreeForAll
    );
}

/// CAT-M-11 / D-ORG16: a member who is not the leader changes the mode.
/// `onErrorCode`, the feedback line and a re-send of the current [51], so
/// the menu snaps back; the mode is unchanged and nobody else hears.
#[tokio::test]
async fn loot_mode_rejects_non_leader() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12]);
    squad::set_loot_mode(12, 1, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    let mut expect = rejection(SID, "Only the squad leader can change the loot mode.");
    expect.push(loot(SquadLootType::RoundRobin));
    assert_eq!(to(&sent, 12), expect);
    assert_eq!(sent.len(), 3, "only the caller hears");
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.loot_mode",
        "not_leader"
    ));
    assert_eq!(
        mgr.squads.squad(SID).unwrap().loot(),
        SquadLootType::RoundRobin
    );
}

/// CAT-M-11 / D-ORG16: only 0 and 1 exist. Even the leader is refused,
/// at WARN (the client's menu cannot send it), and the mode is unchanged.
#[tokio::test]
async fn loot_mode_rejects_out_of_range() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12]);
    squad::set_loot_mode(11, 1, &tx, &mut mgr).await;
    drain(&mut rx);
    for bad in [2, -1] {
        squad::set_loot_mode(11, bad, &tx, &mut mgr).await;
        let sent = drain(&mut rx);
        let mut expect = rejection(SID, "That loot mode does not exist.");
        expect.push(loot(SquadLootType::FreeForAll));
        assert_eq!(to(&sent, 11), expect, "mode {bad}");
        assert_eq!(sent.len(), 3, "mode {bad}: only the caller hears");
    }
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.loot_mode",
        "loot_mode_invalid"
    ));
    assert_eq!(
        mgr.squads.squad(SID).unwrap().loot(),
        SquadLootType::FreeForAll
    );
}

#[tokio::test]
async fn loot_mode_outside_a_squad_is_refused() {
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    squad::set_loot_mode(11, 1, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(0, "You are not in a squad.")
    );
}

/// Gate arrival re-creates the player: the member gets the whole squad
/// again with `aNewMember = 0` and the current loot mode, and `squad_id`
/// is re-stamped. The others are told nothing.
#[tokio::test]
async fn world_entry_replays_the_squad_to_the_member_only() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12]);
    squad::set_loot_mode(11, 1, &tx, &mut mgr).await;
    drain(&mut rx);
    // The re-created entity has no squad_id.
    mgr.get_entity_mut(12).unwrap().squad_id = None;

    squad::on_world_entry(12, 2, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    let ri = |name: &str, rank| RosterInfo {
        name: name.into(),
        level: 12,
        archetype: 3,
        rank,
        note: String::new(),
        officer_note: String::new(),
    };
    assert_eq!(
        to(&sent, 12),
        [
            (
                35,
                build_on_organization_joined(SID, OrgType::Squad, OrgRank::MEMBER, false)
            ),
            (
                38,
                build_on_organization_roster_info(
                    SID,
                    &[ri("Alice", OrgRank::LEADER), ri("Bob", OrgRank::MEMBER)]
                )
            ),
            (
                37,
                build_on_member_joined_organization("Alice", 11, SID, OrgRank::LEADER, false)
            ),
            loot(SquadLootType::FreeForAll),
        ]
    );
    assert_eq!(sent.len(), 4, "the other members are told nothing");
    assert_eq!(mgr.get_entity(12).unwrap().squad_id, Some(SID));
}

/// A first login (no squad) sends nothing and clears any stale id.
#[tokio::test]
async fn world_entry_outside_a_squad_sends_nothing() {
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    mgr.get_entity_mut(11).unwrap().squad_id = Some(SID);
    squad::on_world_entry(11, 1, &tx, &mut mgr).await;
    assert!(drain(&mut rx).is_empty());
    assert_eq!(mgr.get_entity(11).unwrap().squad_id, None);
}
