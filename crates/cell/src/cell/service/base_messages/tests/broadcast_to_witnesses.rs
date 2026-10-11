//! `BaseToCellMsg::BroadcastToWitnesses` — base-built entity-method calls
//! (a rebuilt `BeingAppearance`, …) fanned out to the players who can see
//! the entity.

use super::*;

const OBSERVEE: u32 = 1;
const OBSERVER: u32 = 2;
const FAR_AWAY: u32 = 3;

/// Three connected players in one shared space: the observee, one player
/// standing next to it, one on the far side of the map.
fn shared_space() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-8000" MaxX="8000" MinY="-8000" MaxY="8000" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for (id, pos) in [
        (OBSERVEE, [0.0, 0.0, 0.0]),
        (OBSERVER, [5.0, 0.0, 5.0]),
        (FAR_AWAY, [5000.0, 0.0, 5000.0]),
    ] {
        mgr.create_entity(id, "Castle", pos, [0.0; 3]).unwrap();
        mgr.connect_entity(id);
    }
    // Establish the witness sets the fan-out reads.
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// Regression guard for "other players never see a weapon draw / gear
/// change": the base's rebuilt `BeingAppearance` must reach exactly the
/// players who witness the observee — not the observee's own client (the
/// base already sent that), and not a player out of range.
#[tokio::test]
async fn fans_out_to_witnessing_players_only() {
    let mut mgr = shared_space();
    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let args = vec![0xAA, 0xBB, 0xCC];

    handle_base_message(
        BaseToCellMsg::BroadcastToWitnesses {
            entity_id: OBSERVEE,
            method_index: crate::mercury::method_idx::BEING_APPEARANCE,
            args: args.clone(),
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let mut recipients = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args: sent,
                entity_is_player,
            } => {
                assert_eq!(entity_id, OBSERVEE);
                assert_eq!(method_index, crate::mercury::method_idx::BEING_APPEARANCE);
                assert_eq!(sent, args);
                assert!(
                    entity_is_player,
                    "a player ghost must be encoded with the SGWPlayer idbase"
                );
                recipients.push(witness_id);
            }
            other => panic!("unexpected cell->base message: {other:?}"),
        }
    }
    assert_eq!(recipients, vec![OBSERVER]);
}

/// One connected player alone in a space: nobody witnesses it.
fn lone_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-8000" MaxX="8000" MinY="-8000" MaxY="8000" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(OBSERVEE, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(OBSERVEE);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// Revert-verifier: deleting the `aoi.appearance` INFO row fails this test.
/// With one witness the row carries `witness_count=1`; with none it carries
/// `witness_count=0`.
#[tokio::test]
async fn appearance_rebroadcast_counts_witnesses() {
    use crate::test_support::LogCapture;

    let rebroadcast_rows = |capture: &crate::test_support::LogCaptureGuard| {
        capture
            .all()
            .into_iter()
            .filter(|c| {
                c.target == "aoi.appearance" && c.has_field("event", "appearance_rebroadcast")
            })
            .collect::<Vec<_>>()
    };
    let msg = || BaseToCellMsg::BroadcastToWitnesses {
        entity_id: OBSERVEE,
        method_index: crate::mercury::method_idx::BEING_APPEARANCE,
        args: vec![0xAA],
    };

    // One witness: OBSERVER stands beside OBSERVEE.
    let capture = LogCapture::install();
    let mut mgr = shared_space();
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    handle_base_message(msg(), &tx, &mut mgr, &engine, &[]).await;
    let rows = rebroadcast_rows(&capture);
    assert_eq!(rows.len(), 1, "one rebroadcast row: {:#?}", capture.all());
    assert!(
        rows[0].has_field("witness_count", "1"),
        "one witness must give witness_count=1: {:#?}",
        rows[0]
    );
    drop(capture);

    // No witness: the observee stands alone.
    let capture = LogCapture::install();
    let mut mgr = lone_player();
    handle_base_message(msg(), &tx, &mut mgr, &engine, &[]).await;
    let rows = rebroadcast_rows(&capture);
    assert_eq!(rows.len(), 1, "one rebroadcast row: {:#?}", capture.all());
    assert!(
        rows[0].has_field("witness_count", "0"),
        "no witness must give witness_count=0: {:#?}",
        rows[0]
    );
}
