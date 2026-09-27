//! `Action::SendSystemMail` (SS-U3) on the cell: the two ways a firing
//! sends nothing, each with its `content.send_system_mail reason=` row
//! (TESTING.md type 12). The seeded chain's one-mail send is guarded by
//! `chain_replay_tests/debug_hub_mail_clerk.rs`.

use super::*;
use crate::test_support::LogCapture;

fn mail_action() -> Action {
    Action::SendSystemMail {
        sender_name: "Gate Mail Clerk".into(),
        subject: "Test".into(),
        body: String::new(),
        cash: 50,
        item: Some((2893, 5)),
        cooldown_secs: Some(600),
    }
}

fn resolved() -> ResolvedActions {
    ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(7011, mail_action())],
    }
}

fn refusal(capture: &crate::test_support::LogCaptureGuard, reason: &str) -> bool {
    capture.all().iter().any(|e| {
        e.target == "content"
            && e.level == tracing::Level::WARN
            && e.has_field("event", "content.send_system_mail")
            && e.has_field("reason", reason)
            && e.has_field("chain_id", "7011")
    })
}

/// A chain fired for an entity that is not the named player (an NPC's
/// event, `player_id` 0) mails nobody and says why.
#[tokio::test]
async fn send_system_mail_refuses_a_non_player_entity() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(21, "Agnos", [1.0; 3], [0.0; 3]).unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    execute_actions(resolved(), 21, 0, &tx, &mut mgr, &ChainEngine::new()).await;

    assert!(rx.try_recv().is_err(), "nothing may reach the base");
    assert!(refusal(&capture, "no_player"), "{:#?}", capture.all());
}

/// A closed cell-to-base channel is logged with the player's ids, not
/// swallowed.
#[tokio::test]
async fn send_system_mail_warns_when_cell_to_base_channel_closed() {
    let capture = LogCapture::install();
    let mut mgr = make_space_mgr();
    mgr.create_entity(22, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(22).unwrap();
    p.is_player = true;
    p.player_id = Some(42);
    let (tx, rx) = mpsc::channel(8);
    drop(rx);

    execute_actions(resolved(), 22, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    assert!(
        refusal(&capture, "base_channel_closed"),
        "{:#?}",
        capture.all()
    );
}
