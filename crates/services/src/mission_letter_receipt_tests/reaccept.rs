//! The completed-1360 re-accept refusal (HA-1360b2 review fix A). The attempt is
//! the lifecycle accept of 1360 against the 1360 that the positive path
//! completed, in the Cellblock hydrate. Test-only.

use tracing::Level;

use super::*;
use crate::cell::missions::accept_mission;
use crate::test_support::LogCapture;

/// The Cellblock grant's accept step for 1360 (the step the first grant persisted).
const ACCEPT_STEP_1360: i32 = 4037;

/// A completed 1360 is not accepted again. The attempt calls the lifecycle
/// `accept_mission` for 1360 in a fresh Cellblock hydrate, where 1360 is restored
/// completed. The offer guard refuses it (`repeats` 1 is past the seeded
/// `num_repeats` 0) and logs `repeat_cap_reached`. The accept returns false, the
/// row and the letter count are unchanged, and no accept, step or completion
/// frame is sent.
///
/// This calls the lifecycle directly rather than firing chain 1121: the chain's
/// own `mission_status 1360 eq not_active` gate would refuse before the offer
/// guard ran, so the guard would not be what is under test.
#[tokio::test]
async fn live_db_mission_1360_reaccept_is_refused_after_completion() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 5, TEST_BASE + 6);
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let mut driver = Driver::new(pool.clone(), account_id, player_id, &RECEIPT_CHAINS).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage_at_marsh(&driver, &tx, &mut rx, true).await;
    let frames = deliver(&mut driver, &tx, &mut rx, &mut harset).await;
    driver.route(frames).await;
    let before = driver.row_1360().await;
    assert_eq!(before.0, i32::from(MISSION_COMPLETED));
    assert_eq!(before.6, 1);
    assert_eq!(driver.letter_count().await, 0);

    let capture = LogCapture::install();
    let mut cellblock = driver.hydrate("Castle_CellBlock").await;
    let accepted = accept_mission(
        PLAYER_EID,
        1360,
        ACCEPT_STEP_1360,
        Vec::new(),
        &tx,
        &mut cellblock,
    )
    .await;
    assert!(!accepted, "the offer guard refuses a completed 1360");
    let attempt = drain(&mut rx);
    assert!(
        !frame_methods(&attempt)
            .iter()
            .any(|m| MISSION_FRAMES.contains(m)),
        "a completed 1360 sends no accept, step or completion frame"
    );
    driver.route(attempt).await;
    assert!(
        capture
            .find_event(Level::WARN, "offer refused", "repeat_cap_reached")
            .is_some(),
        "the refusal for the repeat cap is logged"
    );
    assert_eq!(driver.row_1360().await, before, "the row is unchanged");
    assert_eq!(driver.letter_count().await, 0, "no letter is re-granted");

    cleanup(&pool, account_id, player_id).await;
}
