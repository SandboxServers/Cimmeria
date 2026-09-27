//! `sendGMShout` (CM 222, SS-C2): scope, delivery bytes, and refusals.
//!
//! The non-GM refusal is the dispatch-layer gate's and is pinned end to end
//! in `cimmeria-cell`'s `dispatch::tests::gm_shout_rejected_for_player`.

use super::*; // shared helpers from tests/mod.rs
use crate::test_support::LogCapture;
use cimmeria_wire::cell::chat::serialize_gm_broadcast;
use cimmeria_wire::cell::messages::ChatCellToBase;
use tokio::sync::mpsc;

const GM: u32 = 1;
const SAME_SPACE: u32 = 2;
const OTHER_WORLD: u32 = 3;
const OTHER_INSTANCE: u32 = 4;
/// A second instance of the GM's own world, so "space" is pinned to the
/// instance and not to the world name.
const OTHER_INSTANCE_SPACE: u32 = 99;

/// GM (1) and a player (2) in Castle; a player (3) in Harset; a player (4)
/// in a second Castle instance (space 99).
fn two_worlds() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
        <Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        <Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /><Space WorldName="Harset" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_space_instance(OTHER_INSTANCE_SPACE, "Castle");
    for (eid, world) in [
        (GM, "Castle"),
        (SAME_SPACE, "Castle"),
        (OTHER_WORLD, "Harset"),
    ] {
        mgr.create_entity(eid, world, [0.0; 3], [0.0; 3]).unwrap();
    }
    mgr.create_entity_in_space(OTHER_INSTANCE, OTHER_INSTANCE_SPACE, [0.0; 3], [0.0; 3])
        .unwrap();
    for eid in [GM, SAME_SPACE, OTHER_WORLD, OTHER_INSTANCE] {
        mgr.connect_entity(eid);
        let e = mgr.get_entity_mut(eid).unwrap();
        e.player_id = Some(100 + eid as i32);
        e.access_level = if eid == GM { 2 } else { 0 };
    }
    let gm = mgr.get_entity_mut(GM).unwrap();
    gm.character_name = Some("Gm".to_string());
    gm.account_id = Some(7);
    assert_ne!(
        mgr.get_entity_space_id(GM),
        mgr.get_entity_space_id(OTHER_INSTANCE),
        "fixture: the second Castle instance must be a different space"
    );
    mgr
}

fn shout_args(is_global: u8, text: &str) -> Vec<u8> {
    let mut args = vec![is_global];
    write_wstring_arg(&mut args, text);
    args
}

/// Every `onPlayerCommunication` (m28) recipient and its args.
fn lines(msgs: &[CellToBaseMsg]) -> Vec<(u32, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: 28,
                args,
            } => Some((*entity_id, args.clone())),
            _ => None,
        })
        .collect()
}

fn global_forwards(msgs: &[CellToBaseMsg]) -> Vec<&ChatCellToBase> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::Chat(c) => Some(c),
            _ => None,
        })
        .collect()
}

/// Type 8 fan-out: `isGlobal = 0` reaches exactly the players in the GM's
/// space instance (the GM included), with the GM line byte for byte; a
/// player in another world and a player in another instance of the same
/// world get nothing, and nothing goes to the base's global fan-out.
#[tokio::test]
async fn gm_shout_space_scope_stays_in_space() {
    let mut mgr = two_worlds();
    let (tx, mut rx) = mpsc::channel(32);

    let text = "Castle event starts now";
    assert!(
        dispatch(
            GM,
            GM_SEND_GM_SHOUT,
            &shout_args(0, text),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );

    let msgs = drain(&mut rx);
    let expected = serialize_gm_broadcast("Gm", text);
    let mut got = lines(&msgs);
    got.sort_by_key(|(eid, _)| *eid);
    assert_eq!(
        got,
        vec![(GM, expected.clone()), (SAME_SPACE, expected)],
        "a space shout reaches exactly the GM's space instance"
    );
    assert!(
        global_forwards(&msgs).is_empty(),
        "a space shout must never reach the base's global fan-out"
    );
}

/// `isGlobal = 1` hands one `GmBroadcast` to the base, carrying the GM's
/// ids from the cell and the GM line; the cell addresses nobody itself.
#[tokio::test]
async fn gm_shout_global_forwards_one_broadcast_to_base() {
    let mut mgr = two_worlds();
    let (tx, mut rx) = mpsc::channel(32);
    let capture = LogCapture::install();

    let text = "Server restart in 5 minutes";
    assert!(
        dispatch(
            GM,
            GM_SEND_GM_SHOUT,
            &shout_args(1, text),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );

    let msgs = drain(&mut rx);
    assert!(lines(&msgs).is_empty(), "the base does the global fan-out");
    assert_eq!(
        global_forwards(&msgs),
        vec![&ChatCellToBase::GmBroadcast {
            entity_id: GM,
            player_id: Some(101),
            account_id: Some(7),
            source: "native",
            args: serialize_gm_broadcast("Gm", text),
        }]
    );
    let audit = capture
        .find_message(tracing::Level::INFO, "GM broadcast accepted")
        .expect("every broadcast logs chat.gm_broadcast");
    assert!(audit.has_field("event", "chat.gm_broadcast"));
    assert!(audit.has_field("scope", "global"));
    assert!(audit.has_field("player_id", "101"));
    assert!(audit.has_field("account_id", "7"));
    assert!(audit.has_field("source", "native"));
}

/// Refusals (type 12): blank text, text over the D-SS12 cap, and args that
/// do not decode each log `chat.gm_broadcast_rejected` with its reason,
/// tell the GM why, and broadcast nothing.
#[tokio::test]
async fn gm_shout_refusals_send_feedback_and_nothing_else() {
    let too_long = "x".repeat(256);
    let cases: [(Vec<u8>, &str, &str); 4] = [
        (shout_args(1, "   "), "empty_text", "nothing to announce"),
        (shout_args(0, &too_long), "too_long", "too long"),
        (vec![1, 5, 0, 0], "malformed_args", "could not be read"),
        (vec![], "malformed_args", "could not be read"),
    ];
    for (args, reason, feedback) in cases {
        let mut mgr = two_worlds();
        let (tx, mut rx) = mpsc::channel(32);
        let capture = LogCapture::install();

        assert!(dispatch(GM, GM_SEND_GM_SHOUT, &args, &tx, &mut mgr, &test_engine()).await);

        assert!(
            capture
                .find_event(tracing::Level::WARN, "rejected", reason)
                .is_some(),
            "{reason}: must log chat.gm_broadcast_rejected, got {:#?}",
            capture.all()
        );
        let msgs = drain(&mut rx);
        assert!(global_forwards(&msgs).is_empty(), "{reason}: no broadcast");
        let got = lines(&msgs);
        assert_eq!(got.len(), 1, "{reason}: only the GM's feedback line");
        assert_eq!(got[0].0, GM);
        let text = feedback_text(&msgs, GM).unwrap();
        assert!(
            text.contains(feedback),
            "{reason}: feedback {text:?} must say {feedback:?}"
        );
    }
}

/// Type 12: when the base channel is gone, neither scope may end on the
/// "accepted" audit row alone. Global logs one
/// `chat.gm_broadcast_send_failed` for the lost hand-off; space logs one per
/// lost recipient. Both carry the GM's ids and `reason`; the space row also
/// names the recipient.
#[tokio::test]
async fn gm_shout_base_channel_closed_logs_send_failed() {
    for (is_global, scope) in [(1u8, "global"), (0u8, "space")] {
        let mut mgr = two_worlds();
        let (tx, rx) = mpsc::channel(32);
        drop(rx);
        let capture = LogCapture::install();

        assert!(
            dispatch(
                GM,
                GM_SEND_GM_SHOUT,
                &shout_args(is_global, "hello"),
                &tx,
                &mut mgr,
                &test_engine()
            )
            .await
        );

        let failures: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == tracing::Level::WARN
                    && c.has_field("event", "chat.gm_broadcast_send_failed")
            })
            .collect();
        assert!(
            !failures.is_empty(),
            "{scope}: a lost hand-off must log chat.gm_broadcast_send_failed, got {:#?}",
            capture.all()
        );
        for f in &failures {
            assert!(
                f.has_field("reason", "base_channel_closed"),
                "{scope}: {f:?}"
            );
            assert!(f.has_field("scope", scope), "{scope}: {f:?}");
            assert!(f.has_field("account_id", "7"), "{scope}: {f:?}");
            assert!(f.has_field("player_id", "101"), "{scope}: {f:?}");
        }
        if scope == "space" {
            assert_eq!(failures.len(), 2, "one row per lost recipient (GM + 1)");
            assert!(failures
                .iter()
                .any(|f| f.has_field("target_entity_id", "2")
                    && f.has_field("target_player_id", "102")));
        }
    }
}
