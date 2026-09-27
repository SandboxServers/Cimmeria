//! `gmGiveTrainingPoints` (cell method 137): index, routing and refusals.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

/// The client's own entity file, not a constant we typed.
const SGW_GM_PLAYER_DEF: &str = include_str!("../../../../../../../entities/defs/SGWGmPlayer.def");

/// Flattened index of the SGWGmPlayer own `<Exposed/>` cell method `name`:
/// 109 plus its document-order position among the exposed methods of
/// `<CellMethods>` (non-exposed ones such as `gmSetCallback` take no slot).
fn def_index(name: &str) -> Option<u16> {
    let start = SGW_GM_PLAYER_DEF.find("<CellMethods>")?;
    let end = SGW_GM_PLAYER_DEF[start..].find("</CellMethods>")? + start;
    let mut current = "";
    let mut position = 0u16;
    for line in SGW_GM_PLAYER_DEF[start..end].lines().map(str::trim) {
        // A method opens on a line of its own: `<gmGiveXp>`.
        if let Some(tag) = line.strip_prefix('<').and_then(|l| l.strip_suffix('>')) {
            if !tag.contains(['/', ' ', '<', '!']) {
                current = tag;
            }
        }
        if line.contains("<Exposed/>") {
            if current == name {
                return Some(109 + position);
            }
            position += 1;
        }
    }
    None
}

#[test]
fn gm_give_training_points_index_is_derived_from_the_def() {
    // The parser must first reproduce the pcap-anchored indices, or its
    // answer for 137 proves nothing.
    assert_eq!(def_index("gmGiveItem"), Some(GM_GIVE_ITEM));
    assert_eq!(def_index("gmGotoXYZ"), Some(GM_GOTO_XYZ));
    assert_eq!(
        def_index("gmGiveTrainingPoints"),
        Some(GM_GIVE_TRAINING_POINTS),
        "the constant must match the client's method table"
    );
}

#[tokio::test]
async fn gm_give_training_points_routes_137_to_a_grant_for_the_caller() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(
        dispatch(1, 137, &7i32.to_le_bytes(), &tx, &mut mgr, &test_engine()).await,
        "137 must be handled, not fall through to the unimplemented arm"
    );
    let msgs = drain(&mut rx);
    assert_eq!(
        msgs.len(),
        1,
        "only the grant; the base owns the definitive feedback: {msgs:?}"
    );
    match &msgs[0] {
        CellToBaseMsg::GrantTrainingPoints {
            entity_id,
            player_id,
            amount,
            gm_feedback_to,
        } => {
            assert_eq!(
                (*entity_id, *player_id, *amount, *gm_feedback_to),
                (1, 100, 7, Some(1)),
                "grant to the caller, feedback to the caller"
            );
        }
        other => panic!("expected GrantTrainingPoints, got {other:?}"),
    }
}

#[tokio::test]
async fn gm_give_training_points_refuses_a_non_positive_amount_with_feedback() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    for amount in [0i32, -3, i32::MIN] {
        assert!(
            dispatch(
                1,
                GM_GIVE_TRAINING_POINTS,
                &amount.to_le_bytes(),
                &tx,
                &mut mgr,
                &test_engine()
            )
            .await
        );
        let msgs = drain(&mut rx);
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::GrantTrainingPoints { .. })),
            "amount {amount} must not reach the base"
        );
        assert_eq!(
            feedback_text(&msgs, 1).as_deref(),
            Some("gmGiveTrainingPoints: amount must be positive"),
            "amount {amount} must answer the GM"
        );
    }
}

#[tokio::test]
async fn gm_give_training_points_refuses_truncated_args_with_feedback() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(
        dispatch(
            1,
            GM_GIVE_TRAINING_POINTS,
            &[1, 0],
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    let msgs = drain(&mut rx);
    assert!(!msgs
        .iter()
        .any(|m| matches!(m, CellToBaseMsg::GrantTrainingPoints { .. })));
    assert_eq!(
        feedback_text(&msgs, 1).as_deref(),
        Some("gmGiveTrainingPoints: missing INT32 amount")
    );
}
