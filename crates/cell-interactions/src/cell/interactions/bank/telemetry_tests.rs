//! `LogCapture` guards (TESTING.md type 12) for the BV-02 events of the
//! D-BV19 telemetry contract: `vault_session_opened`,
//! `vault_session_closed reason=re_pin`, every `vault_open_rejected`
//! reason this crate emits, the two send-failure negative logs, and the
//! `bank.banker_interact` span. `space_change` / `logout` closes are pinned
//! in `cimmeria-cell-world` (`space_manager::vault_session_end`), and
//! `not_gm` in `cimmeria-cell-console` (`tests::bv02_bank`).

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_entity::cell_entity::VaultScope;

use super::tests::{spawn_banker, two_space_manager, PLAYER};
use super::*;
use crate::cell::interactions::handle_interact;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

/// Every `bank` row carrying `event = name`.
fn rows(capture: &LogCaptureGuard, name: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", name))
        .collect()
}

/// Exactly one `name` row, at `level`, carrying the three correlators and
/// every `(field, value)` in `want`.
fn one(capture: &LogCaptureGuard, name: &str, level: Level, want: &[(&str, &str)]) -> Captured {
    let found = rows(capture, name);
    assert_eq!(found.len(), 1, "exactly one {name}: {:#?}", capture.all());
    let row = found.into_iter().next().unwrap();
    assert_eq!(row.level, level, "{name} level");
    for (k, v) in [("account_id", "6"), ("player_id", "12"), ("entity_id", "1")]
        .iter()
        .chain(want)
    {
        assert!(row.has_field(k, v), "{name}: {k}={v} missing: {row:#?}");
    }
    row
}

fn span_opened(capture: &LogCaptureGuard, name: &str) -> bool {
    capture
        .all()
        .iter()
        .any(|c| c.target == format!("span:{name}") && c.level == Level::INFO)
}

/// Banker open: DEBUG `vault_session_opened` with scope, Banker, space and
/// distance, inside the INFO `bank.banker_interact` span.
#[tokio::test]
async fn banker_open_logs_vault_session_opened_in_the_interact_span() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let space = mgr.get_entity_space_id(PLAYER).unwrap().to_string();
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    handle_interact(PLAYER, banker, &tx, &mut mgr).await;

    one(
        &capture,
        "vault_session_opened",
        Level::DEBUG,
        &[
            ("scope", "personal"),
            ("banker_id", &banker.to_string()),
            ("gm_override", "false"),
            ("space_id", &space),
            ("distance", "2.0"),
        ],
    );
    assert!(span_opened(&capture, "bank.banker_interact"));
}

/// GM open: `gm_override=true`, and no `banker_id` or `distance` field at
/// all (no sentinel values).
#[tokio::test]
async fn gm_open_logs_vault_session_opened_with_gm_override() {
    let mut mgr = two_space_manager();
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    assert!(open_vault_gm(PLAYER, &tx, &mut mgr).await);

    let row = one(
        &capture,
        "vault_session_opened",
        Level::DEBUG,
        &[("scope", "personal"), ("gm_override", "true")],
    );
    assert!(!row.fields.contains_key("banker_id"), "{row:#?}");
    assert!(!row.fields.contains_key("distance"), "{row:#?}");
}

/// Re-pin: DEBUG `vault_session_closed reason=re_pin` with the scope and
/// how long the window was open.
#[tokio::test]
async fn re_pin_logs_vault_session_closed_with_reason_re_pin() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let other = mgr.allocate_npc_id();
    mgr.spawn_npc(other, "Agnos", [0.0, 0.0, 2.0], [0.0; 3])
        .unwrap();
    let (tx, _rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    let capture = LogCapture::install();

    handle_interact(PLAYER, other, &tx, &mut mgr).await;

    let row = one(
        &capture,
        "vault_session_closed",
        Level::DEBUG,
        &[("reason", "re_pin"), ("scope", "personal")],
    );
    assert!(row.fields.contains_key("open_ms"), "{row:#?}");
}

/// Out of range: WARN `vault_open_rejected reason=out_of_range` with the
/// Banker and the distance. A far click on an NPC that is not a Banker
/// logs nothing under `bank`.
#[tokio::test]
async fn far_banker_click_logs_vault_open_rejected_out_of_range() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [10.0, 0.0, 0.0], VaultScope::Personal);
    let plain = mgr.allocate_npc_id();
    mgr.spawn_npc(plain, "Agnos", [0.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    one(
        &capture,
        "vault_open_rejected",
        Level::WARN,
        &[
            ("reason", "out_of_range"),
            ("banker_id", &banker.to_string()),
            ("distance", "10.0"),
        ],
    );

    handle_interact(PLAYER, plain, &tx, &mut mgr).await;
    assert_eq!(rows(&capture, "vault_open_rejected").len(), 1);
}

/// Org Bankers: WARN `vault_open_rejected reason=org_vault_not_available`
/// for both scopes, with the Banker and distance.
#[tokio::test]
async fn org_banker_click_logs_vault_open_rejected_org_vault_not_available() {
    for scope in [VaultScope::Team, VaultScope::Command] {
        let mut mgr = two_space_manager();
        let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], scope);
        let (tx, _rx) = mpsc::channel(16);
        let capture = LogCapture::install();

        handle_interact(PLAYER, banker, &tx, &mut mgr).await;

        one(
            &capture,
            "vault_open_rejected",
            Level::WARN,
            &[
                ("reason", "org_vault_not_available"),
                ("banker_id", &banker.to_string()),
                ("distance", "2.0"),
            ],
        );
        assert!(rows(&capture, "vault_session_opened").is_empty());
    }
}

/// Banker lookup miss: WARN `vault_open_rejected reason=banker_missing`,
/// and the player still gets a line.
#[tokio::test]
async fn missing_banker_logs_vault_open_rejected_banker_missing() {
    let mut mgr = two_space_manager();
    let (tx, mut rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    open_vault_at_banker(PLAYER, 999_999, VaultScope::Personal, &tx, &mut mgr).await;

    one(
        &capture,
        "vault_open_rejected",
        Level::WARN,
        &[("reason", "banker_missing"), ("banker_id", "999999")],
    );
    assert!(rx.try_recv().is_ok(), "a feedback line was sent");
    assert!(mgr.get_entity(PLAYER).unwrap().vault_session.is_none());
}

/// Player lookup miss on `.bank`: WARN `reason=player_missing`. No
/// correlators exist for an unknown entity, so only the reason is checked.
#[tokio::test]
async fn gm_open_for_a_missing_entity_logs_player_missing() {
    let mut mgr = two_space_manager();
    let (tx, _rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    assert!(!open_vault_gm(4242, &tx, &mut mgr).await);

    let found = rows(&capture, "vault_open_rejected");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].level, Level::WARN);
    assert!(found[0].has_field("reason", "player_missing"));
}

/// Closed base channel on the open: WARN `vault_open_send_failed
/// reason=base_channel_closed` (a silent `send` would hide a window that
/// never appeared).
#[tokio::test]
async fn closed_channel_logs_vault_open_send_failed() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, rx) = mpsc::channel(16);
    drop(rx);
    let capture = LogCapture::install();

    handle_interact(PLAYER, banker, &tx, &mut mgr).await;

    one(
        &capture,
        "vault_open_send_failed",
        Level::WARN,
        &[("reason", "base_channel_closed")],
    );
}

/// Closed base channel on a refusal line: WARN `bank_feedback_send_failed`.
#[tokio::test]
async fn closed_channel_logs_bank_feedback_send_failed() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Team);
    let (tx, rx) = mpsc::channel(16);
    drop(rx);
    let capture = LogCapture::install();

    handle_interact(PLAYER, banker, &tx, &mut mgr).await;

    one(
        &capture,
        "bank_feedback_send_failed",
        Level::WARN,
        &[("reason", "base_channel_closed")],
    );
}

/// Every `vault_open_rejected` reason is one of the D-BV19 catalog strings
/// (plus `player_missing`, the lookup-miss addition).
#[test]
fn vault_open_reject_reasons_are_the_catalog_strings() {
    let all = [
        VaultOpenReject::OutOfRange,
        VaultOpenReject::OrgVaultNotAvailable,
        VaultOpenReject::NotGm,
        VaultOpenReject::BankerMissing,
        VaultOpenReject::PlayerMissing,
    ];
    let reasons: Vec<_> = all.iter().map(|r| r.reason()).collect();
    assert_eq!(
        reasons,
        [
            "out_of_range",
            "org_vault_not_available",
            "not_gm",
            "banker_missing",
            "player_missing"
        ]
    );
}
