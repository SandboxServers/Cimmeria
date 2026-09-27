//! The base invite inbox (D-ORG06, CAT-M-18) with injected instants: the
//! composite key, single use, the 60 s edge, the caps and the logoff clear.

use super::*;

const INVITEE: i32 = 11;
const OTHER_INVITEE: i32 = 12;
const INVITER: i32 = 21;
const ORG: i32 = 7;

fn issue(s: &mut OrgInviteState, invitee: i32, inviter: i32, now: Instant) -> PendingOrgInvite {
    s.issue(invitee, inviter, "Inviter", ORG, OrgType::Command, now)
        .expect("issue")
}

#[test]
fn request_ids_carry_the_base_flag_and_count_up() {
    let a = next_request_id().unwrap();
    let b = next_request_id().unwrap();
    for id in [a, b] {
        assert_ne!(id & BASE_INVITE_REQUEST_FLAG, 0, "{id:#x}");
        assert!(id > 0, "never negative: {id:#x}");
        assert_eq!(id & (1 << 30), 0, "never the squad org-id bit: {id:#x}");
    }
    assert!(b > a, "monotonic, never reused");
}

/// CAT-M-18: the id alone finds nothing; the entry answers only for the
/// invitee it was issued to, and a miss leaves it in place.
#[test]
fn a_foreign_request_id_misses_and_leaves_the_invite() {
    let now = Instant::now();
    let mut s = OrgInviteState::default();
    let inv = issue(&mut s, INVITEE, INVITER, now);
    assert_eq!(
        s.take(OTHER_INVITEE, inv.request_id, now),
        Err(TakeMiss::Unknown)
    );
    assert!(s.holds(inv.request_id));
    assert_eq!(s.take(INVITEE, inv.request_id, now), Ok(inv));
}

/// CAT-M-18: consumed by the first response; the replay finds nothing.
#[test]
fn an_invite_is_single_use() {
    let now = Instant::now();
    let mut s = OrgInviteState::default();
    let inv = issue(&mut s, INVITEE, INVITER, now);
    assert!(s.take(INVITEE, inv.request_id, now).is_ok());
    assert_eq!(s.take(INVITEE, inv.request_id, now), Err(TakeMiss::Unknown));
}

/// Answers at 59.999 s, not at 60 s; the expiry is reported as expired and
/// recorded for the `invite_expired` log.
#[test]
fn an_invite_expires_at_sixty_seconds() {
    let t0 = Instant::now();
    let mut s = OrgInviteState::default();
    let live = issue(&mut s, INVITEE, INVITER, t0);
    let just = t0 + INVITE_TTL - Duration::from_millis(1);
    assert!(s.take(INVITEE, live.request_id, just).is_ok());

    let stale = issue(&mut s, INVITEE, INVITER, t0);
    assert_eq!(
        s.take(INVITEE, stale.request_id, t0 + INVITE_TTL),
        Err(TakeMiss::Expired)
    );
    assert_eq!(s.drain_expired(), vec![stale]);
    assert!(s.drain_expired().is_empty());
}

#[test]
fn one_pending_invite_per_pair_and_five_per_invitee() {
    let now = Instant::now();
    let mut s = OrgInviteState::default();
    issue(&mut s, INVITEE, INVITER, now);
    assert_eq!(
        s.issue(INVITEE, INVITER, "x", ORG, OrgType::Team, now),
        Err(IssueReject::InviteLimit)
    );
    for inviter in 1..MAX_PENDING_PER_INVITEE as i32 {
        issue(&mut s, INVITEE, 100 + inviter, now);
    }
    assert_eq!(s.pending_for(INVITEE, now), MAX_PENDING_PER_INVITEE);
    assert_eq!(
        s.issue(INVITEE, 999, "x", ORG, OrgType::Team, now),
        Err(IssueReject::InviteLimit)
    );
}

/// The sender's limit slides: five in 30 s, the sixth only once the first
/// leaves the window.
#[test]
fn the_send_limit_slides() {
    let t0 = Instant::now();
    let mut s = OrgInviteState::default();
    for i in 0..INVITE_RATE_MAX as u64 {
        let t = t0 + Duration::from_secs(i);
        assert!(s.may_send(t));
        s.record_sent(t);
    }
    assert!(!s.may_send(t0 + INVITE_RATE_WINDOW - Duration::from_millis(1)));
    assert!(s.may_send(t0 + INVITE_RATE_WINDOW));
}

/// Review fix: a return to character select drops the character's invites,
/// so they neither answer for, nor count against, the next character; the
/// send history survives the switch.
#[test]
fn logoff_clears_held_invites_but_not_the_send_history() {
    let now = Instant::now();
    let mut s = OrgInviteState::default();
    let inv = issue(&mut s, INVITEE, INVITER, now);
    s.record_sent(now);
    assert_eq!(s.clear_for_logoff(), 1);
    assert_eq!(s.take(INVITEE, inv.request_id, now), Err(TakeMiss::Unknown));
    assert_eq!(s.pending_for(INVITEE, now), 0);
    assert_eq!(s.sent.len(), 1);
}
