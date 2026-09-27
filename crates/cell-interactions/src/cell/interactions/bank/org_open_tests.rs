//! The cell half of the Team and Command vault open (bank-vault BV-07):
//! the base's grant opens the window only while the player still has that
//! Banker pinned and in range. Wire-format (TESTING.md type 2) for 107 and
//! 108, and `LogCapture` guards (type 12) for `org_vault_open_rejected`,
//! one per reason, and the org form of `vault_session_opened`.

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::player::{ON_COMMAND_VAULT_OPEN, ON_TEAM_VAULT_OPEN};

use super::tests::{spawn_banker, two_space_manager, PLAYER};
use super::*;
use crate::cell::interactions::handle_interact;
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const ORG: i32 = 0x7000_B9DA;

/// Every client method call sent, in order.
fn methods(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u16, Vec<u8>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            out.push((entity_id, method_index, args));
        }
    }
    out
}

/// A player who clicked a `scope` Banker at `pos` (so it is pinned), with
/// the base's request drained.
async fn clicked(
    scope: VaultScope,
    pos: [f32; 3],
) -> (
    SpaceManager,
    u32,
    mpsc::Sender<CellToBaseMsg>,
    mpsc::Receiver<CellToBaseMsg>,
) {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", pos, scope);
    let (tx, mut rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    while rx.try_recv().is_ok() {}
    (mgr, banker, tx, rx)
}

fn rejected(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "org_vault_open_rejected"))
        .collect()
}

/// Byte-exact: the grant sends `onTeamVaultOpen` (107) for a Team and
/// `onCommandVaultOpen` (108) for a Command, each `(INT32 banker_id,
/// VECTOR3 banker_position)`, and records a session naming the org. Fails
/// if the method index is not the scope's, or the args drift.
#[tokio::test]
async fn a_grant_opens_the_scope_window_and_a_session_naming_the_org() {
    for (scope, method) in [
        (VaultScope::Team, ON_TEAM_VAULT_OPEN),
        (VaultScope::Command, ON_COMMAND_VAULT_OPEN),
    ] {
        let (mut mgr, banker, tx, mut rx) = clicked(scope, [2.0, 0.5, -1.0]).await;

        grant_org_vault(PLAYER, 12, scope, ORG, banker, &tx, &mut mgr).await;

        let mut want = (banker as i32).to_le_bytes().to_vec();
        for axis in [2.0f32, 0.5, -1.0] {
            want.extend_from_slice(&axis.to_le_bytes());
        }
        assert_eq!(methods(&mut rx), vec![(PLAYER, method, want)], "{scope:?}");
        let s = mgr.get_entity(PLAYER).unwrap().vault_session.clone();
        let s = s.expect("the grant opens a session");
        assert_eq!(
            (s.scope, s.org_id, s.banker_id),
            (scope, Some(ORG), Some(banker))
        );
        assert_eq!(
            vault_access(PLAYER, &mgr).open_org_vault(),
            Some((scope, ORG)),
            "{scope:?}: the next move's verdict names the org"
        );
    }
}

/// The org form of `vault_session_opened` carries `org_id` and the scope.
#[tokio::test]
async fn a_grant_logs_vault_session_opened_with_the_org() {
    let (mut mgr, banker, tx, _rx) = clicked(VaultScope::Team, [2.0, 0.0, 0.0]).await;
    let capture = LogCapture::install();

    grant_org_vault(PLAYER, 12, VaultScope::Team, ORG, banker, &tx, &mut mgr).await;

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "vault_session_opened"))
        .collect();
    assert_eq!(rows.len(), 1, "{:#?}", capture.all());
    let row = &rows[0];
    assert_eq!(row.level, Level::DEBUG);
    for (k, v) in [
        ("scope", "team"),
        ("org_id", &ORG.to_string()[..]),
        ("banker_id", &banker.to_string()[..]),
        ("account_id", "6"),
        ("player_id", "12"),
        ("entity_id", "1"),
        ("distance", "2.0"),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {row:#?}");
    }
}

/// One refusal: exactly one WARN `org_vault_open_rejected` with `reason`
/// and the org, no session, and a line iff `line`.
async fn assert_refused(
    capture: &LogCaptureGuard,
    mgr: &SpaceManager,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    reason: &str,
    line: bool,
) {
    let rows = rejected(capture);
    assert_eq!(rows.len(), 1, "{reason}: {:#?}", capture.all());
    let row = &rows[0];
    assert_eq!(row.level, Level::WARN);
    for (k, v) in [
        ("reason", reason),
        ("org_id", &ORG.to_string()[..]),
        ("player_id", "12"),
        ("scope", "team"),
        ("org_type", "team"),
    ] {
        assert!(row.has_field(k, v), "{reason}: {k}={v}: {row:#?}");
    }
    let sent: Vec<u16> = methods(rx).into_iter().map(|(_, m, _)| m).collect();
    let want = if line {
        vec![ON_PLAYER_COMMUNICATION]
    } else {
        vec![]
    };
    assert_eq!(
        sent, want,
        "{reason}: a line iff the player can be told, no window"
    );
    if let Some(p) = mgr.get_entity(PLAYER) {
        assert_eq!(p.vault_session, None, "{reason}: no session");
    }
}

/// The player clicked something else while the base answered.
#[tokio::test]
async fn a_grant_after_a_re_pin_is_refused_banker_not_pinned() {
    let (mut mgr, banker, tx, mut rx) = clicked(VaultScope::Team, [2.0, 0.0, 0.0]).await;
    mgr.get_entity_mut(PLAYER).unwrap().last_interaction_target = Some(banker + 1);
    let capture = LogCapture::install();
    grant_org_vault(PLAYER, 12, VaultScope::Team, ORG, banker, &tx, &mut mgr).await;
    assert_refused(&capture, &mgr, &mut rx, "banker_not_pinned", true).await;
}

/// The player walked away while the base answered.
#[tokio::test]
async fn a_grant_after_walking_away_is_refused_out_of_range() {
    let (mut mgr, banker, tx, mut rx) = clicked(VaultScope::Team, [2.0, 0.0, 0.0]).await;
    mgr.get_entity_mut(PLAYER).unwrap().position.x = 40.0;
    let capture = LogCapture::install();
    grant_org_vault(PLAYER, 12, VaultScope::Team, ORG, banker, &tx, &mut mgr).await;
    assert_refused(&capture, &mgr, &mut rx, "out_of_range", true).await;
}

/// The Banker despawned while the base answered.
#[tokio::test]
async fn a_grant_for_a_gone_banker_is_refused_banker_missing() {
    let (mut mgr, banker, tx, mut rx) = clicked(VaultScope::Team, [2.0, 0.0, 0.0]).await;
    mgr.destroy_entity(banker);
    let capture = LogCapture::install();
    grant_org_vault(PLAYER, 12, VaultScope::Team, ORG, banker, &tx, &mut mgr).await;
    assert_refused(&capture, &mgr, &mut rx, "banker_missing", true).await;
}

/// The entity now plays another character: refused, and nobody is told
/// (the line would reach the wrong player).
#[tokio::test]
async fn a_grant_for_another_character_is_refused_stale_entity() {
    let (mut mgr, banker, tx, mut rx) = clicked(VaultScope::Team, [2.0, 0.0, 0.0]).await;
    mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(99);
    let capture = LogCapture::install();
    grant_org_vault(PLAYER, 12, VaultScope::Team, ORG, banker, &tx, &mut mgr).await;
    assert_refused(&capture, &mgr, &mut rx, "stale_entity", false).await;
}

/// The player's entity is gone (logout, gate travel).
#[tokio::test]
async fn a_grant_for_a_missing_entity_is_refused_player_entity_missing() {
    let (mut mgr, banker, tx, mut rx) = clicked(VaultScope::Team, [2.0, 0.0, 0.0]).await;
    mgr.destroy_entity(PLAYER);
    let capture = LogCapture::install();
    grant_org_vault(PLAYER, 12, VaultScope::Team, ORG, banker, &tx, &mut mgr).await;
    assert_refused(&capture, &mgr, &mut rx, "player_entity_missing", false).await;
}

/// `org_vault_open_rejected`'s cell-side reasons are stable strings.
#[test]
fn org_grant_reasons_are_stable() {
    let all = [
        OrgGrantReject::PlayerEntityMissing,
        OrgGrantReject::StaleEntity,
        OrgGrantReject::BankerNotPinned,
        OrgGrantReject::BankerMissing,
        OrgGrantReject::OutOfRange,
    ];
    let reasons: Vec<_> = all.iter().map(|r| r.reason()).collect();
    assert_eq!(
        reasons,
        [
            "player_entity_missing",
            "stale_entity",
            "banker_not_pinned",
            "banker_missing",
            "out_of_range"
        ]
    );
}
