use super::*; // shared helpers from tests/mod.rs
use tokio::sync::mpsc;

/// `bTurnOn=0` means the client's normal physics/collision just went OFF
/// (fly/ghost engaged) — the validator bypass must turn ON.
#[tokio::test]
async fn feat_onphysics_turn_on_zero_sets_unrestricted_true() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_PHYSICS, &[0u8], &tx, &mut mgr).await);

    assert!(
        mgr.get_entity(1).unwrap().movement_unrestricted,
        "bTurnOn=0 (physics off) must set movement_unrestricted = true"
    );
    let msgs = drain(&mut rx);
    let text = feedback_text(&msgs, 1).expect("must send GM feedback");
    assert!(
        text.contains("disabled"),
        "feedback must describe validation as disabled, got {text:?}"
    );
}

/// `bTurnOn=1` means normal physics is restored — the validator bypass
/// must turn back OFF.
#[tokio::test]
async fn feat_onphysics_turn_on_one_sets_unrestricted_false() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Start from unrestricted=true so the flip is observable, not a no-op.
    mgr.get_entity_mut(1).unwrap().movement_unrestricted = true;

    assert!(dispatch(1, GM_PHYSICS, &[1u8], &tx, &mut mgr).await);

    assert!(
        !mgr.get_entity(1).unwrap().movement_unrestricted,
        "bTurnOn=1 (physics on) must set movement_unrestricted = false"
    );
    let msgs = drain(&mut rx);
    let text = feedback_text(&msgs, 1).expect("must send GM feedback");
    assert!(
        text.contains("restored"),
        "feedback must describe validation as restored, got {text:?}"
    );
}

/// Malformed (truncated) args must reject with feedback and mutate nothing
/// — a dropped/short packet must never silently flip a security-relevant
/// flag in either direction.
#[tokio::test]
async fn feat_onphysics_truncated_args_rejected_without_mutation() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_PHYSICS, &[], &tx, &mut mgr).await);

    assert!(
        !mgr.get_entity(1).unwrap().movement_unrestricted,
        "truncated args must not mutate movement_unrestricted"
    );
    let msgs = drain(&mut rx);
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "truncated args must feed back a rejection"
    );
}
