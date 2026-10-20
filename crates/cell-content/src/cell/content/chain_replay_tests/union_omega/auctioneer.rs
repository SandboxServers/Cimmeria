//! UO-32: the Auction House auctioneer (spawn 63819, template 305) and its
//! chain 63850 on `UnionOmega_Auctioneer`. D-UO19 c, D-UO36; F38.
//!
//! The Black Market window needs the Cimmeria client patch; the chat line is
//! the first-press feedback without it.

use cimmeria_entity::cell_entity::NpcInteractionType;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::services_common::*;
use crate::test_support::require_db_or_skip;

const SPAWN: i32 = 63819;
const CHAIN: i32 = 63850;
const TAG: &str = "UnionOmega_Auctioneer";
const OPENED_LINE: &str =
    "The auctioneer opens the Black Market. (No window? The Black Market needs the Cimmeria client patch.)";

/// The Auction House auctioneer is spawned as an `Auctioneer`, and clicking
/// him runs chain 63850: `onBMOpen` to the player naming him, the open line,
/// and a Black Market session at him.
#[tokio::test]
async fn live_db_omega_auctioneer_opens_the_black_market() {
    let pool = require_db_or_skip!();
    let (mut mgr, ids) = hub_with(&pool, &[SPAWN], SPAWN).await;
    let a = ids[&SPAWN];
    assert_eq!(
        mgr.get_entity(a).unwrap().interaction_type,
        Some(NpcInteractionType::Auctioneer),
        "spawn {SPAWN} (template 305) derives Auctioneer at spawn"
    );
    let engine = engine_with(&pool, &[CHAIN]).await;

    let (handled, sent) = click(&mut mgr, &engine, TAG, a).await;

    assert!(handled, "chain {CHAIN} claims the click on spawn {SPAWN}");
    let opens: Vec<_> = sent.iter().filter(|c| c.1 == ON_BM_OPEN).collect();
    assert_eq!(opens.len(), 1, "spawn {SPAWN} sends one onBMOpen: {sent:?}");
    assert_eq!(opens[0].0, PLAYER);
    assert_eq!(opens[0].2, (a as i32).to_le_bytes().to_vec());
    let line = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, OPENED_LINE);
    assert!(
        sent.iter()
            .any(|c| c.0 == PLAYER && c.1 == ON_PLAYER_COMMUNICATION && c.2 == line),
        "chain {CHAIN} tells the player the Black Market opened: {sent:?}"
    );
    assert_eq!(
        mgr.black_market
            .get(PLAYER_ID)
            .and_then(|s| s.auctioneer_id),
        Some(a),
        "chain {CHAIN} records a Black Market session at spawn {SPAWN}"
    );
}

/// The tag names spawn 63819 of template 305 and no other placement.
#[tokio::test]
async fn live_db_omega_auctioneer_tag_is_on_template_305_only() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32)> =
        sqlx::query_as("SELECT spawn_id, template_id FROM resources.spawnlist WHERE tag = $1")
            .bind(TAG)
            .fetch_all(&pool)
            .await
            .expect("query resources.spawnlist by tag");
    assert_eq!(
        rows,
        vec![(SPAWN, 305)],
        "tag {TAG} must name spawn {SPAWN} of template 305 only"
    );
}
