//! UO-33: the Infirmary medic (spawn 63818, template 63855, tag
//! `UnionOmega_Medic`). Chain 63851 refills the clicking player's HEALTH and
//! barks screen 63905 to that player only (D-UO19 c, F40). Every press speaks
//! the line, even at full health.

use cimmeria_entity::stats::HEALTH;

use super::services_common::*;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_STAT_UPDATE;
use crate::test_support::require_db_or_skip;

const SPAWN: i32 = 63818;
const CHAIN: i32 = 63851;
const SCREEN: i32 = 63905;
const TAG: &str = "UnionOmega_Medic";
/// UO-15's speaker string, verbatim.
const SPEAKER: &str = "Castle Medic";

/// Sets the player's HEALTH to `cur` of 1000 and drains the dirty flags, so
/// the send list reads only what the chain wrote.
fn set_player_health(mgr: &mut SpaceManager, cur: i32) {
    let pl = mgr
        .get_entity_mut(PLAYER)
        .expect("the player must exist in the hub");
    let h = pl
        .stats
        .get_mut(HEALTH)
        .expect("the player must carry a HEALTH stat");
    h.update(0, cur, 1000);
    pl.stats.clear_dirty();
}

/// The argument bytes of every send to the player with `method`.
fn sends_to_player(sent: &[(u32, u16, Vec<u8>)], method: u16) -> Vec<&Vec<u8>> {
    sent.iter()
        .filter(|(e, m, _)| *e == PLAYER && *m == method)
        .map(|(_, _, args)| args)
        .collect()
}

/// The player's current HEALTH value.
fn player_health(mgr: &mut SpaceManager) -> i32 {
    mgr.get_entity_mut(PLAYER)
        .expect("the player must exist in the hub")
        .stats
        .get_mut(HEALTH)
        .expect("the player must carry a HEALTH stat")
        .cur
}

#[tokio::test]
async fn live_db_omega_medic_heals_and_speaks() {
    let pool = require_db_or_skip!();
    let (mut mgr, ids) = hub_with(&pool, &[SPAWN], SPAWN).await;
    let medic = *ids
        .get(&SPAWN)
        .expect("spawn 63818 (UnionOmega_Medic) must map to an entity");
    let engine = engine_with(&pool, &[CHAIN]).await;
    set_player_health(&mut mgr, 100);

    let (handled, sent) = click(&mut mgr, &engine, TAG, medic).await;
    assert!(
        handled,
        "a click on spawn 63818 (UnionOmega_Medic) must be claimed by chain 63851"
    );
    assert_eq!(
        player_health(&mut mgr),
        1000,
        "chain 63851 change_stat must refill HEALTH to 1000"
    );
    assert_eq!(
        sends_to_player(&sent, ON_STAT_UPDATE).len(),
        1,
        "chain 63851 change_stat must send one onStatUpdate to the player"
    );
    let text = screen_text(&pool, SCREEN).await;
    let barks = sends_to_player(&sent, ON_PLAYER_COMMUNICATION);
    assert_eq!(
        barks.len(),
        1,
        "chain 63851 npc_bark must speak screen 63905 to the player once"
    );
    assert_eq!(
        barks[0],
        &bark_bytes(SPEAKER, &text),
        "chain 63851 npc_bark must send screen 63905's text as Castle Medic"
    );
}

#[tokio::test]
async fn live_db_omega_medic_speaks_at_full_health() {
    let pool = require_db_or_skip!();
    let (mut mgr, ids) = hub_with(&pool, &[SPAWN], SPAWN).await;
    let medic = *ids
        .get(&SPAWN)
        .expect("spawn 63818 (UnionOmega_Medic) must map to an entity");
    let engine = engine_with(&pool, &[CHAIN]).await;
    set_player_health(&mut mgr, 1000);

    let (handled, sent) = click(&mut mgr, &engine, TAG, medic).await;
    assert!(
        handled,
        "a click on spawn 63818 (UnionOmega_Medic) at full health must be claimed by chain 63851"
    );
    assert_eq!(
        player_health(&mut mgr),
        1000,
        "chain 63851 must leave HEALTH at max for a player at full health"
    );
    let text = screen_text(&pool, SCREEN).await;
    let barks = sends_to_player(&sent, ON_PLAYER_COMMUNICATION);
    assert_eq!(
        barks.len(),
        1,
        "chain 63851 must still bark screen 63905 at full health (first-press feedback)"
    );
    assert_eq!(
        barks[0],
        &bark_bytes(SPEAKER, &text),
        "chain 63851 must send screen 63905's text as Castle Medic at full health"
    );
}

#[tokio::test]
async fn live_db_omega_medic_line_is_screen_index_4() {
    let pool = require_db_or_skip!();
    let row: (i32, i32, i32) = sqlx::query_as(
        "SELECT dialog_id, index, speaker_id FROM resources.dialog_screens WHERE screen_id = $1",
    )
    .bind(SCREEN)
    .fetch_one(&pool)
    .await
    .unwrap_or_else(|e| panic!("dialog screen {SCREEN} must be seeded: {e}"));
    assert_eq!(
        row,
        (63900, 4, 0),
        "screen 63905 (the medic line for chain 63851) must sit on dialog 63900 at index 4 with speaker_id 0"
    );
    let text = screen_text(&pool, SCREEN).await.to_lowercase();
    for banned in ["cimmeria", "authored", "reconstruction", "placeholder"] {
        assert!(
            !text.contains(banned),
            "screen 63905 (the medic line) must not contain '{banned}'"
        );
    }
}
