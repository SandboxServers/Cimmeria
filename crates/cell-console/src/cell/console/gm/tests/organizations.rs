//! ORG-10: `gmReloadOrganizations` (164) is inside the gated `SGWGmPlayer`
//! tail, reaches its handler, and is forwarded to the base as
//! `OrgCellToBase::GmReload` with the GM's own character.

use super::*;
use crate::cell::dispatch::gm_gate::{enforce_gm_gate, requires_gm, SGWGMPLAYER_CELL_METHOD_BASE};
use crate::cell::messages::OrgCellToBase;
use crate::test_support::LogCapture;

/// The index is the def's, and the dispatch gate refuses it for a non-GM
/// before any handler runs: without the gate a player could make the base
/// re-send organization state on demand.
#[tokio::test]
async fn gm_reload_organizations_is_inside_the_gated_tail() {
    assert_eq!(GM_RELOAD_ORGANIZATIONS, 164);
    const { assert!(GM_RELOAD_ORGANIZATIONS >= SGWGMPLAYER_CELL_METHOD_BASE) };
    assert!(requires_gm(GM_RELOAD_ORGANIZATIONS));

    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().access_level = 0;
    let (tx, mut rx) = mpsc::channel(8);
    assert!(
        !enforce_gm_gate(1, GM_RELOAD_ORGANIZATIONS, &tx, &mgr).await,
        "a player may not call gmReloadOrganizations"
    );
    assert!(
        !drain(&mut rx)
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::Org(_))),
        "nothing reached the base"
    );
    mgr.get_entity_mut(1).unwrap().access_level = 2;
    assert!(enforce_gm_gate(1, GM_RELOAD_ORGANIZATIONS, &tx, &mgr).await);
}

/// The GM's call is handled (not the unimplemented arm) and forwarded with
/// the GM's own character and entity.
#[tokio::test]
async fn gm_reload_organizations_forwards_to_the_base() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    assert!(
        dispatch(
            1,
            GM_RELOAD_ORGANIZATIONS,
            &[],
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    let msgs = drain(&mut rx);
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(matches!(
        &msgs[0],
        CellToBaseMsg::Org(OrgCellToBase::GmReload {
            player_id: 100,
            entity_id: 1,
        })
    ));
}

/// A caller with no character id is refused on the cell with feedback and
/// its `org.gm_action` row; nothing reaches the base as an org message.
#[tokio::test]
async fn gm_reload_organizations_without_a_character_stays_on_the_cell() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().player_id = None;
    let (tx, mut rx) = mpsc::channel(8);
    let capture = LogCapture::install();
    assert!(
        dispatch(
            1,
            GM_RELOAD_ORGANIZATIONS,
            &[],
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    let msgs = drain(&mut rx);
    assert!(!msgs.iter().any(|m| matches!(m, CellToBaseMsg::Org(_))));
    assert!(!msgs.is_empty(), "the GM got a feedback line");
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "org.gm_action"))
        .expect("org.gm_action row");
    assert_eq!(row.target, "org");
    assert_eq!(row.level, tracing::Level::INFO);
    assert!(row.has_field("action", "gm_reload_organizations"));
    assert!(row.has_field("reason", "caller_not_player"));
}
