//! `BaseToCellMsg::Org` routing (ORG-01): each nested variant reaches
//! `org::handle`, logs on the `squad` target and sends nothing to the base.

use super::*;
use crate::cell::messages::OrgBaseToCell;
use crate::test_support::LogCapture;

#[tokio::test]
async fn every_org_variant_reaches_the_org_arm() {
    let capture = LogCapture::install();
    let mut mgr = SpaceManager::new(1);
    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let msgs = [
        (
            OrgBaseToCell::SquadInvite {
                player_id: 11,
                entity_id: 21,
                target_name: "Bo".into(),
            },
            "squad.invite_unimplemented",
        ),
        (
            OrgBaseToCell::SquadKick {
                player_id: 11,
                entity_id: 21,
                org_id: 0x4000_0001,
                target_name: "Bo".into(),
            },
            "squad.kick_unimplemented",
        ),
    ];
    for (msg, event) in msgs {
        handle_base_message(BaseToCellMsg::Org(msg), &tx, &mut mgr, &engine, &[]).await;
        let ev = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", event))
            .unwrap_or_else(|| panic!("{event} not logged"));
        assert_eq!(ev.target, "squad");
        assert_eq!(ev.level, tracing::Level::DEBUG);
        assert!(ev.has_field("player_id", "11") && ev.has_field("entity_id", "21"));
    }
    assert!(
        rx.try_recv().is_err(),
        "a no-op arm sends nothing to the base"
    );
}
