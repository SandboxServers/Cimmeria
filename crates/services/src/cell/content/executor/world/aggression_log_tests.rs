//! Negative-log guards for `set_aggression` (audit gap T10).
//!
//! A tag that resolves to nothing used to be a silent no-op, so a mistyped
//! chain tag left a guard passive forever and looked exactly like an aggro
//! bug on the floor. Kept apart from `tests.rs`, which is already past the
//! file-size cap.

use super::*;
use crate::test_support::LogCapture;
use tracing::Level;

fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(1).unwrap().is_player = true;
    mgr.create_entity(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(101).unwrap().tag = Some("Drone".to_string());
    mgr
}

/// A tag that matches no entity raises one WARN naming the tag and the
/// chain. Fails if the miss goes back to being silent.
#[test]
fn set_aggression_tag_miss_warns_with_tag_and_chain() {
    let mut mgr = make_space_mgr();
    let logs = LogCapture::install();

    set_aggression("Dorne".to_string(), 1, 1, 1032, &mut mgr);

    let ev = logs
        .find_event(Level::WARN, "matched no entity", "tag_not_found")
        .expect("a tag miss must WARN with reason=tag_not_found");
    assert_eq!(ev.target, "content");
    assert!(ev.has_field("event", "set_aggression_tag_miss"), "{ev:?}");
    assert!(ev.has_field("tag", "Dorne"), "{ev:?}");
    assert!(ev.has_field("chain_id", "1032"), "{ev:?}");
    assert_eq!(mgr.get_entity(101).unwrap().aggression, 0);
}

/// The success path is INFO with `from` / `to`, and raises no WARN.
#[test]
fn set_aggression_hit_logs_from_and_to_at_info() {
    let mut mgr = make_space_mgr();
    let logs = LogCapture::install();

    set_aggression("Drone".to_string(), 2, 1, 1032, &mut mgr);

    assert_eq!(mgr.get_entity(101).unwrap().aggression, 2);
    let ev = logs
        .find_message(Level::INFO, "Content: set aggression")
        .expect("a hit must log at INFO");
    assert!(ev.has_field("event", "set_aggression"), "{ev:?}");
    assert!(ev.has_field("from", "0"), "{ev:?}");
    assert!(ev.has_field("to", "2"), "{ev:?}");
    assert!(
        logs.all().iter().all(|c| c.level != Level::WARN),
        "a hit must not warn"
    );
}

/// NA00 review: the content executor's `GenerateThreat` action is the one
/// real caller of `AggroCause::ContentThreat`. Driving the action (not
/// `combat::generate_threat` directly) pins that it passes the right cause;
/// a wrong cause would label every chain-armed guard (chains 1008, 1032) as
/// `damage` or `proximity`.
#[tokio::test]
async fn generate_threat_action_logs_the_content_threat_cause() {
    let mut mgr = make_space_mgr();
    mgr.get_entity_mut(101).unwrap().class_id = 0x04;
    let (tx, _rx) = tokio::sync::mpsc::channel(16);
    let logs = LogCapture::install();

    generate_threat(Some("Drone".to_string()), 1000, 1, 1008, &tx, &mut mgr).await;

    let all = logs.all();
    let acquired = all
        .iter()
        .find(|c| c.target == "npc_ai.aggro")
        .expect("the Fighting entry must log npc_ai.aggro");
    assert!(
        acquired.has_field("cause", "content_threat"),
        "{acquired:?}"
    );
    let transition = all
        .iter()
        .find(|c| c.target == "npc_ai.transition")
        .expect("and a transition row");
    assert!(transition.has_field("reason", "content"), "{transition:?}");
}
