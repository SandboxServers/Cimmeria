//! Live-DB guards for the GM `.giveability` grant (pets campaign PT-07).
//!
//! Bug shapes: a grant that never reaches the row (so it vanishes on relog);
//! a grant that lands in `trained_abilities` (a respec would then remove it
//! and refund points it never cost); a second grant that duplicates the id;
//! a grant for a character the session no longer plays; and feedback sent to
//! whoever inherited the GM's recycled entity id.
//!
//! Sentinels: `0x7030_0Axx`, accounts and players share the id.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::grant_ability::{persist_ability_grant, GrantWrite};
use super::respec::persist_respec;
use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use super::{handle_gm_grant_ability, AbilityGrant};
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{require_db_or_skip, Captured, LogCapture, TestTransport};

const SUBJECT: u32 = 9_301_000;
const GM: u32 = 9_301_001;
const GM_PLAYER: i32 = 0x7030_0A7F;
const ABILITY: i32 = 2826;

/// `(abilities, trained_abilities, training_points, tree_points_spent)`.
type Row = (Vec<i32>, Vec<i32>, i32, i32);

async fn setup(pool: &sqlx::PgPool, id: i32, abilities: &[i32], trained: &[i32]) {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 5_000).await;
    let r = sqlx::query(
        "UPDATE sgw_player SET abilities = $1, trained_abilities = $2, \
                training_points = 3, tree_points_spent = $3 WHERE player_id = $4",
    )
    .bind(abilities)
    .bind(trained)
    .bind(trained.len() as i32)
    .bind(id)
    .execute(pool)
    .await
    .expect("seed abilities");
    assert_eq!(r.rows_affected(), 1, "fixture row must exist");
}

async fn row(pool: &sqlx::PgPool, id: i32) -> Row {
    sqlx::query_as(
        "SELECT abilities, trained_abilities, training_points, tree_points_spent \
           FROM sgw_player WHERE player_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("read player row")
}

/// The one `decision_outcome = outcome` event at `level`, with `reason` if
/// given.
fn outcome_event(
    capture: &crate::test_support::LogCaptureGuard,
    level: tracing::Level,
    outcome: &str,
    reason: Option<&str>,
) -> Captured {
    let all = capture.all();
    let found: Vec<&Captured> = all
        .iter()
        .filter(|c| {
            c.level == level
                && c.has_field("decision_outcome", outcome)
                && reason.is_none_or(|r| c.has_field("reason", r))
        })
        .collect();
    assert_eq!(found.len(), 1, "want one {level} {outcome}: {all:#?}");
    found[0].clone()
}

/// Two sessions: the subject playing `subject_player` and the GM playing
/// `gm_player`.
struct Sessions {
    subject_addr: SocketAddr,
    gm_addr: SocketAddr,
    transport: Arc<TestTransport>,
    connected: Arc<Mutex<HashMap<SocketAddr, super::ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl Sessions {
    fn new(subject_player: i32, gm_player: i32) -> Self {
        let subject_addr: SocketAddr = "127.0.0.1:65391".parse().unwrap();
        let gm_addr: SocketAddr = "127.0.0.1:65392".parse().unwrap();
        Self {
            subject_addr,
            gm_addr,
            transport: Arc::new(TestTransport::new()),
            connected: Arc::new(Mutex::new(HashMap::from([
                (subject_addr, make_connected_state(Some(subject_player))),
                (gm_addr, make_connected_state(Some(gm_player))),
            ]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([
                (SUBJECT, subject_addr),
                (GM, gm_addr),
            ]))),
        }
    }

    /// Run the handler; return what it sent the cell.
    async fn grant(
        &self,
        pool: &sqlx::PgPool,
        player_id: i32,
        gm_player_id: i32,
    ) -> Vec<BaseToCellMsg> {
        let (tx, mut rx) = mpsc::channel(4);
        self.grant_via(pool, player_id, gm_player_id, Some(tx))
            .await;
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    /// Run the handler with `cell_tx` as the base->cell channel.
    async fn grant_via(
        &self,
        pool: &sqlx::PgPool,
        player_id: i32,
        gm_player_id: i32,
        cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    ) {
        let transport: Arc<dyn Transport> = self.transport.clone();
        handle_gm_grant_ability(
            AbilityGrant {
                entity_id: SUBJECT,
                player_id,
                ability_id: ABILITY,
                gm_entity_id: GM,
                gm_player_id,
            },
            &Some(Arc::new(pool.clone())),
            &transport,
            &self.connected,
            &self.entity_to_addr,
            &cell_tx,
        )
        .await;
    }
}

/// The grant reaches `abilities` only: the respec provenance and both point
/// counters are untouched, and the cell and the GM both hear about it.
#[tokio::test]
async fn live_db_giveability_persists_to_abilities_only_and_tells_the_cell_and_the_gm() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0A01;
    setup(&pool, ID, &[592, 1643], &[1643]).await;
    let s = Sessions::new(ID, GM_PLAYER);

    let capture = LogCapture::install();
    let to_cell = s.grant(&pool, ID, GM_PLAYER).await;

    // Rule 5: the GM is the actor, the character granted is the subject.
    let e = outcome_event(&capture, tracing::Level::INFO, "granted", None);
    for (k, v) in [
        ("persisted", "true".to_string()),
        ("player_id", GM_PLAYER.to_string()),
        ("account_id", "0".to_string()),
        ("subject_player_id", ID.to_string()),
        ("ability_id", ABILITY.to_string()),
    ] {
        assert!(e.has_field(k, &v), "{k}={v}: {e:#?}");
    }
    assert_eq!(
        row(&pool, ID).await,
        (vec![592, 1643, ABILITY], vec![1643], 3, 1),
        "only `abilities` gains the id; trained_abilities and both counters stay"
    );
    assert!(
        matches!(
            to_cell.as_slice(),
            [BaseToCellMsg::GmAbilityGranted {
                entity_id: SUBJECT,
                player_id: ID,
                ability_id: ABILITY,
            }]
        ),
        "exactly one GmAbilityGranted for the subject ({} messages)",
        to_cell.len()
    );
    assert_eq!(s.transport.send_count_to(s.gm_addr), 1, "the GM hears it");
    assert_eq!(
        s.transport.send_count_to(s.subject_addr),
        0,
        "the subject gets no GM feedback line"
    );
    cleanup(&pool, ID).await;
}

/// A second grant of the same id changes nothing and tells the cell nothing,
/// but still answers the GM.
#[tokio::test]
async fn live_db_giveability_twice_does_not_duplicate_the_ability() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0A02;
    setup(&pool, ID, &[592], &[]).await;
    let s = Sessions::new(ID, GM_PLAYER);

    s.grant(&pool, ID, GM_PLAYER).await;
    let capture = LogCapture::install();
    let second = s.grant(&pool, ID, GM_PLAYER).await;
    let e = outcome_event(
        &capture,
        tracing::Level::DEBUG,
        "refused",
        Some("already_known"),
    );
    assert!(e.has_field("persisted", "false"), "{e:#?}");

    assert_eq!(row(&pool, ID).await.0, vec![592, ABILITY]);
    assert!(second.is_empty(), "a no-op grant tells the cell nothing");
    assert_eq!(
        s.transport.send_count_to(s.gm_addr),
        2,
        "both grants are answered"
    );
    assert_eq!(
        persist_ability_grant(&pool, ID, ABILITY).await.unwrap(),
        GrantWrite::AlreadyKnown
    );
    cleanup(&pool, ID).await;
}

/// The subject's session moved on to another character between the cell's
/// send and the base: the resolved character must not be written.
#[tokio::test]
async fn live_db_giveability_for_a_character_the_session_no_longer_plays_is_refused() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0A03;
    setup(&pool, ID, &[592], &[]).await;
    let s = Sessions::new(ID + 1, GM_PLAYER);

    let capture = LogCapture::install();
    let to_cell = s.grant(&pool, ID, GM_PLAYER).await;
    let e = outcome_event(
        &capture,
        tracing::Level::WARN,
        "refused",
        Some("session_mismatch"),
    );
    assert!(
        e.has_field("persisted", "false") && e.has_field("subject_player_id", &ID.to_string()),
        "{e:#?}"
    );

    assert_eq!(row(&pool, ID).await.0, vec![592], "the row never moved");
    assert!(to_cell.is_empty());
    assert_eq!(s.transport.send_count_to(s.gm_addr), 1, "refusal answered");
    cleanup(&pool, ID).await;
}

/// The GM relogged (its entity id now plays another character): the grant
/// still lands, but the feedback line must not reach the stranger.
#[tokio::test]
async fn live_db_giveability_feedback_skips_a_recycled_gm_entity() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0A04;
    setup(&pool, ID, &[592], &[]).await;
    let s = Sessions::new(ID, GM_PLAYER + 1);

    let capture = LogCapture::install();
    let to_cell = s.grant(&pool, ID, GM_PLAYER).await;
    // The recycled entity's account is not the GM's: no `account_id` at all.
    let e = outcome_event(&capture, tracing::Level::INFO, "granted", None);
    assert!(!e.fields.contains_key("account_id"), "{e:#?}");
    outcome_event(
        &capture,
        tracing::Level::DEBUG,
        "feedback_dropped",
        Some("gm_session_gone"),
    );

    assert_eq!(row(&pool, ID).await.0, vec![592, ABILITY]);
    assert_eq!(to_cell.len(), 1, "the cell still mirrors the grant");
    assert_eq!(
        s.transport.send_count_to(s.gm_addr),
        0,
        "no feedback to the entity that no longer plays the GM"
    );
    cleanup(&pool, ID).await;
}

/// A trainer respec removes only trainer-bought abilities: the GM-granted id
/// stays and the refund is exactly the trainer spend.
#[tokio::test]
async fn live_db_giveability_survives_a_respec_and_refunds_nothing() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0A05;
    setup(&pool, ID, &[592, 1643], &[1643]).await;
    let s = Sessions::new(ID, GM_PLAYER);
    s.grant(&pool, ID, GM_PLAYER).await;

    persist_respec(&pool, ID, 1_000)
        .await
        .expect("respec query")
        .expect("player row exists");

    let (abilities, trained, training_points, spent) = row(&pool, ID).await;
    assert_eq!(abilities, vec![592, ABILITY], "1643 refunded, 2826 kept");
    assert!(trained.is_empty());
    assert_eq!((training_points, spent), (4, 0), "3 + the 1-point spend");
    cleanup(&pool, ID).await;
}

/// Span fields are not copied onto OTLP log records, so a mirror failure
/// names the GM actor on the event itself: closed channel (ERROR) and no
/// channel (WARN).
#[tokio::test]
async fn live_db_giveability_mirror_failure_names_the_gm_and_the_subject() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0A06;
    let s = Sessions::new(ID, GM_PLAYER);
    // NT-22 (Rule 6): the GM, the subject and the ability are named.
    cimmeria_entity::known_names::remember_player(GM_PLAYER, "George Hammond");
    cimmeria_entity::known_names::remember_player(ID, "Cameron Mitchell");
    let mut book = cimmeria_names::NameBook::empty();
    book.insert(
        cimmeria_names::Table::Abilities,
        i64::from(ABILITY),
        "Staff Blast",
    );
    cimmeria_names::global().store(book);
    let closed = {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        Some(tx)
    };
    for (cell_tx, level, reason) in [
        (closed, tracing::Level::ERROR, "base_to_cell_closed"),
        (None, tracing::Level::WARN, "no_cell_channel"),
    ] {
        setup(&pool, ID, &[592], &[]).await;
        let capture = LogCapture::install();
        s.grant_via(&pool, ID, GM_PLAYER, cell_tx).await;
        let e = outcome_event(&capture, level, "mirror_send_failed", Some(reason));
        for (k, v) in [
            ("entity_id", GM.to_string()),
            ("account_id", "0".to_string()),
            ("player_id", GM_PLAYER.to_string()),
            ("subject_entity_id", SUBJECT.to_string()),
            ("subject_player_id", ID.to_string()),
            ("ability_id", ABILITY.to_string()),
            ("player_name", "George Hammond".to_string()),
            ("entity_name", "George Hammond".to_string()),
            ("subject_player_name", "Cameron Mitchell".to_string()),
            ("subject_entity_name", "Cameron Mitchell".to_string()),
            ("ability_name", "Staff Blast".to_string()),
        ] {
            assert!(e.has_field(k, &v), "{reason}: {k}={v}: {e:#?}");
        }
        cleanup(&pool, ID).await;
    }
}

/// The character's row is gone (deleted after the session check): the GM is
/// told there is no saved record, not that it "already knows" the ability,
/// and the refusal carries its own reason.
#[tokio::test]
async fn live_db_giveability_for_a_missing_player_row_says_so() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0A07;
    cleanup(&pool, ID).await;
    let s = Sessions::new(ID, GM_PLAYER);

    let capture = LogCapture::install();
    let to_cell = s.grant(&pool, ID, GM_PLAYER).await;

    let e = outcome_event(
        &capture,
        tracing::Level::WARN,
        "refused",
        Some("player_row_missing"),
    );
    assert!(
        e.has_field("persisted", "false")
            && e.has_field("subject_player_id", &ID.to_string())
            && e.has_field("player_id", &GM_PLAYER.to_string()),
        "{e:#?}"
    );
    assert!(to_cell.is_empty(), "nothing to mirror");
    assert_eq!(
        persist_ability_grant(&pool, ID, ABILITY).await.unwrap(),
        GrantWrite::PlayerRowMissing
    );
    assert_eq!(s.transport.send_count_to(s.gm_addr), 1, "the GM hears it");
    cleanup(&pool, ID).await;
}
