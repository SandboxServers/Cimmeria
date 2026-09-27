//! Squad invites: issuing, the join fanout, and the CAT-M-18 response
//! guards.

use std::time::Instant;

use cimmeria_entity::organization::{OrgRank, OrgType, SquadLootType, SQUAD_ORG_ID_MIN};
use cimmeria_wire::cell::client_methods::organization::{
    build_on_member_joined_organization, build_on_organization_invite,
    build_on_organization_joined, build_on_organization_roster_info, build_on_squad_loot_type,
    RosterInfo,
};

use super::*;
use crate::test_support::LogCapture;

const SID: i32 = SQUAD_ORG_ID_MIN;

fn ri(name: &str, rank: OrgRank) -> RosterInfo {
    RosterInfo {
        name: name.into(),
        level: 12,
        archetype: 3,
        rank,
        note: String::new(),
        officer_note: String::new(),
    }
}

#[tokio::test]
async fn invite_sends_34_to_the_target_and_confirms_to_the_inviter() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    squad::handle_invite(1, 11, "Bob", &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(
        to(&sent, 12),
        [(
            34,
            build_on_organization_invite("Alice", OrgType::Squad, 1, "", false)
        )]
    );
    assert_eq!(
        to(&sent, 11),
        [(28, line("You invited Bob to your squad."))]
    );
    assert_eq!(sent.len(), 2);
    assert_eq!(mgr.squads.squad_count(), 0, "nobody has accepted yet");
}

/// The first accept creates the squad, and both founders get the whole
/// squad: [35], the roster [38], a [37] for the other founder with their
/// live entity id, then [51].
#[tokio::test]
async fn first_accept_sends_both_founders_the_whole_squad() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    let req = invite_only(&mut mgr, &tx, 11, 12).await;
    drain(&mut rx);
    squad::respond(12, req, true, &tx, &mut mgr).await;
    let sent = drain(&mut rx);

    let roster = build_on_organization_roster_info(
        SID,
        &[ri("Alice", OrgRank::LEADER), ri("Bob", OrgRank::MEMBER)],
    );
    let loot = build_on_squad_loot_type(SID, SquadLootType::RoundRobin);
    assert_eq!(
        to(&sent, 11),
        [
            (
                35,
                build_on_organization_joined(SID, OrgType::Squad, OrgRank::LEADER, true)
            ),
            (38, roster.clone()),
            (
                37,
                build_on_member_joined_organization("Bob", 12, SID, OrgRank::MEMBER, true)
            ),
            (51, loot.clone()),
        ]
    );
    assert_eq!(
        to(&sent, 12),
        [
            (
                35,
                build_on_organization_joined(SID, OrgType::Squad, OrgRank::MEMBER, true)
            ),
            (38, roster),
            (
                37,
                build_on_member_joined_organization("Alice", 11, SID, OrgRank::LEADER, true)
            ),
            (51, loot),
        ]
    );
    assert_eq!(sent.len(), 8);
    assert_eq!(mgr.get_entity(11).unwrap().squad_id, Some(SID));
    assert_eq!(mgr.get_entity(12).unwrap().squad_id, Some(SID));
}

/// A third member: the newcomer gets the whole squad, with a [37] per
/// other member (`aNewMember = 0`, they are not new); each existing member
/// gets exactly one [37] for the newcomer and never a roster.
#[tokio::test]
async fn later_join_sends_the_newcomer_the_squad_and_the_rest_one_37() {
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    invite_accept(&mut mgr, &tx, 11, 12).await;
    let req = invite_only(&mut mgr, &tx, 11, 13).await;
    drain(&mut rx);
    squad::respond(13, req, true, &tx, &mut mgr).await;
    let sent = drain(&mut rx);

    let cara_joined = build_on_member_joined_organization("Cara", 13, SID, OrgRank::MEMBER, true);
    assert_eq!(to(&sent, 11), [(37, cara_joined.clone())]);
    assert_eq!(to(&sent, 12), [(37, cara_joined)]);
    assert_eq!(
        to(&sent, 13),
        [
            (
                35,
                build_on_organization_joined(SID, OrgType::Squad, OrgRank::MEMBER, true)
            ),
            (
                38,
                build_on_organization_roster_info(
                    SID,
                    &[
                        ri("Alice", OrgRank::LEADER),
                        ri("Bob", OrgRank::MEMBER),
                        ri("Cara", OrgRank::MEMBER)
                    ]
                )
            ),
            (
                37,
                build_on_member_joined_organization("Alice", 11, SID, OrgRank::LEADER, false)
            ),
            (
                37,
                build_on_member_joined_organization("Bob", 12, SID, OrgRank::MEMBER, false)
            ),
            (51, build_on_squad_loot_type(SID, SquadLootType::RoundRobin)),
        ]
    );
}

/// A decline consumes the invite and tells the inviter.
#[tokio::test]
async fn decline_tells_the_inviter() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    let req = invite_only(&mut mgr, &tx, 11, 12).await;
    drain(&mut rx);
    squad::respond(12, req, false, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(
        sent,
        [(11, 28, line("Bob declined your squad invitation."))]
    );
    assert_eq!(mgr.squads.pending_for(2, Instant::now()), 0);
    assert_eq!(mgr.squads.squad_count(), 0);
}

/// Each target-resolution refusal: feedback, no [34], no invite stored.
#[tokio::test]
async fn invite_refuses_unresolvable_targets() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Twin", "Twin2"]);
    // Two entities named "Twin" (the uniqueness invariant broken).
    mgr.get_entity_mut(14).unwrap().character_name = Some("Twin".into());
    // Entity 15: connected but not yet initialised (no player_id).
    mgr.create_entity(15, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.connect_entity(15);
    mgr.get_entity_mut(15).unwrap().character_name = Some("Fresh".into());
    // Entity 16: named but not in any space's player set (in transit).
    mgr.create_entity(16, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(16).unwrap().character_name = Some("Gone".into());
    let (tx, mut rx) = channel();

    let cases = [
        (
            "Nobody",
            "No player named Nobody is online.",
            "target_not_found",
            Level::INFO,
        ),
        (
            "Gone",
            "Gone is travelling. Try again in a moment.",
            "target_in_transition",
            Level::INFO,
        ),
        (
            "Twin",
            "More than one player is called Twin; the invitation was not sent.",
            "target_ambiguous",
            Level::INFO,
        ),
        (
            "Alice",
            "You cannot invite yourself.",
            "self_target",
            Level::INFO,
        ),
        (
            "Fresh",
            "No player named Fresh is online.",
            "not_a_player",
            Level::INFO,
        ),
    ];
    for (name, text, reason, level) in cases {
        squad::handle_invite(1, 11, name, &tx, &mut mgr).await;
        let sent = drain(&mut rx);
        assert_eq!(sent.len(), 2, "{name}: exactly the refusal pair");
        assert_eq!(to(&sent, 11), rejection(0, text), "{name}");
        assert!(
            squad_event(&capture, level, "squad.invite", reason),
            "{name}: {reason} at {level}"
        );
    }
    let now = Instant::now();
    for p in 1..=4 {
        assert!(mgr.squads.pending_requests(p, now).is_empty());
    }
}

/// A registry refusal (target already squadded) reaches the player too.
#[tokio::test]
async fn invite_refuses_a_squadded_target() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    invite_accept(&mut mgr, &tx, 12, 13).await;
    drain(&mut rx);
    squad::handle_invite(1, 11, "Cara", &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(0, "Cara is already in a squad.")
    );
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.invite",
        "already_in_squad"
    ));
}

/// CAT-M-18: Bob holds his own invite and answers Cara's request id. The
/// composite key finds nothing; Cara's invite survives and still works.
#[tokio::test]
async fn invite_response_rejects_foreign_request_id() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();
    let bob_req = invite_only(&mut mgr, &tx, 11, 12).await;
    let cara_req = invite_only(&mut mgr, &tx, 11, 13).await;
    assert_ne!(bob_req, cara_req);
    drain(&mut rx);

    squad::respond(12, cara_req, true, &tx, &mut mgr).await;
    let sent = drain(&mut rx);
    assert_eq!(sent.len(), 2);
    assert_eq!(
        to(&sent, 12),
        rejection(0, "That invitation is no longer valid.")
    );
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.invite_response",
        "invite_foreign"
    ));
    assert_eq!(mgr.squads.squad_count(), 0);
    assert_eq!(mgr.squads.pending_requests(3, Instant::now()), [cara_req]);
    assert_eq!(mgr.squads.pending_requests(2, Instant::now()), [bob_req]);

    squad::respond(13, cara_req, true, &tx, &mut mgr).await;
    assert_eq!(mgr.squads.squad_of(3), Some(SID));
}

/// CAT-M-18: an invite is single-use. Decline, then accept the same id:
/// refused as unknown (not "already in a squad", which would also stop a
/// second accept). Then accept, leave, and replay the id: refused as
/// unknown again, and the player stays out.
#[tokio::test]
async fn invite_response_rejects_replay() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice", "Bob", "Cara"]);
    let (tx, mut rx) = channel();

    let req = invite_only(&mut mgr, &tx, 11, 12).await;
    squad::respond(12, req, false, &tx, &mut mgr).await;
    drain(&mut rx);
    squad::respond(12, req, true, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 12),
        rejection(0, "That invitation is no longer valid.")
    );
    assert_eq!(mgr.squads.squad_of(2), None);

    // Squad of three so Bob's leave does not dissolve it.
    invite_accept(&mut mgr, &tx, 11, 13).await;
    let req = invite_accept(&mut mgr, &tx, 11, 12).await;
    squad::leave(12, SID, &tx, &mut mgr).await;
    drain(&mut rx);
    squad::respond(12, req, true, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 12),
        rejection(0, "That invitation is no longer valid.")
    );
    assert_eq!(mgr.squads.squad_of(2), None);
    assert_eq!(mgr.squads.squad(SID).unwrap().members().len(), 2);
    let unknown = capture
        .all()
        .iter()
        .filter(|c| {
            c.level == Level::INFO
                && c.has_field("event", "squad.invite_response")
                && c.has_field("reason", "invite_unknown")
        })
        .count();
    assert_eq!(unknown, 2);
}

/// CAT-M-18: two accepts into a five-member squad leave six, never seven.
/// The loser is refused with feedback, stays squadless, and nobody is told
/// about them.
#[tokio::test]
async fn squad_accept_rejects_when_full() {
    let capture = LogCapture::install();
    let mut mgr = world(&["A", "B", "C", "D", "E", "F", "G"]);
    let (tx, mut rx) = channel();
    seed_squad(&mut mgr, 11, &[12, 13, 14, 15]);
    let f = invite_only(&mut mgr, &tx, 11, 16).await;
    let g = invite_only(&mut mgr, &tx, 11, 17).await;
    drain(&mut rx);

    squad::respond(16, f, true, &tx, &mut mgr).await;
    drain(&mut rx);
    squad::respond(17, g, true, &tx, &mut mgr).await;
    let sent = drain(&mut rx);

    assert_eq!(sent.len(), 2, "only the loser's refusal: {sent:?}");
    assert_eq!(to(&sent, 17), rejection(0, "That squad is full."));
    assert!(squad_event(
        &capture,
        Level::INFO,
        "squad.invite_response",
        "squad_full"
    ));
    assert_eq!(mgr.squads.squad(SID).unwrap().members().len(), 6);
    assert_eq!(mgr.squads.squad_of(7), None);
    assert_eq!(mgr.get_entity(17).unwrap().squad_id, None);
}

/// The inviter logged off before the accept: their invites went with them.
#[tokio::test]
async fn accept_after_the_inviter_disconnected_is_refused() {
    let mut mgr = world(&["Alice", "Bob"]);
    let (tx, mut rx) = channel();
    let req = invite_only(&mut mgr, &tx, 11, 12).await;
    squad::on_disconnect(11, &tx, &mut mgr).await;
    drain(&mut rx);
    squad::respond(12, req, true, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 12),
        rejection(0, "That invitation is no longer valid.")
    );
    assert_eq!(mgr.squads.squad_count(), 0);
}
