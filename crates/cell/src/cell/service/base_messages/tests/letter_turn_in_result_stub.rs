use super::*;
use cimmeria_wire::cell::messages::{
    LetterTurnInAttempt, LetterTurnInEntryEpoch, LetterTurnInOutcome, LetterTurnInRequest,
    LetterTurnInResult, LetterTurnInToken,
};

#[tokio::test]
async fn result_stub_never_applies_success() {
    let mut space_mgr = SpaceManager::new(1);
    space_mgr.parse_spaces_xml(r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#).unwrap();
    space_mgr
        .create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" /></Spaces>"#,
        )
        .unwrap();
    space_mgr
        .create_entity(421, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    let entity = space_mgr.get_entity_mut(421).unwrap();
    entity.is_player = true;
    entity.player_id = Some(35);
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let request = LetterTurnInRequest {
        entity_id: 421,
        player_id: 35,
        attempt: LetterTurnInAttempt::from_bytes([3; 16]),
        token: LetterTurnInToken::from_bytes([4; 16]),
        generation: std::time::Instant::now(),
        epoch: LetterTurnInEntryEpoch::new(1),
    };
    let result = BaseToCellMsg::LetterTurnInResult(LetterTurnInResult {
        request,
        outcome: LetterTurnInOutcome::UnknownRetryable,
    });
    handle_base_message(result, &tx, &mut space_mgr, &ChainEngine::new(), &[]).await;
    assert!(
        rx.try_recv().is_err(),
        "stub must emit no cell-to-base follow-up"
    );
    let entity = space_mgr.get_entity(421).unwrap();
    assert_eq!(entity.player_id, Some(35));
}
