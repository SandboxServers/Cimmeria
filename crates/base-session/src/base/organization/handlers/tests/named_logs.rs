//! Rule 6 on the organization outcome rows: every id on a row has its name
//! next to it (`account_name`, `player_name`, `entity_name`).
//!
//! These run without a database: the no-database refusal is a real path of
//! each handler and ends in the same outcome row as any other refusal, so
//! the row's shape is what is under test. Without the name fields on the
//! row, each assertion fails (TESTING.md type 12, capture layer).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::base::organization::handlers::chat::ChatSpeaker;
use crate::base::organization::handlers::{handle_kick, handle_leave, relay_org_chat};
use crate::base::organization::handlers::{OrgCtx, OrgPlayer};
use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use tracing::Level;

const ACCOUNT: u32 = 5;
const PLAYER: i32 = 12;
const ENTITY: u32 = 77;

struct World {
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db: Option<Arc<sqlx::PgPool>>,
    cell_tx: Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    addr: SocketAddr,
}

impl World {
    /// One online session: account `sgc_login`, character `Teal'c`.
    fn new() -> Self {
        let addr: SocketAddr = "127.0.0.1:20077".parse().unwrap();
        let mut s = test_default_connected_client_state();
        s.account_id = ACCOUNT;
        s.account_name = Some("sgc_login".into());
        s.active_player_id = Some(PLAYER);
        s.player_name = Some("Teal'c".into());
        s.player_entity_id = Some(ENTITY);
        s.listed_online = true;
        Self {
            transport: Arc::new(TestTransport::new()),
            connected: Arc::new(Mutex::new(HashMap::from([(addr, s)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
            db: None,
            cell_tx: None,
            addr,
        }
    }

    fn ctx(&self) -> OrgCtx<'_> {
        OrgCtx {
            db_pool: &self.db,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
            cell_tx: &self.cell_tx,
        }
    }

    fn player(&self) -> OrgPlayer {
        OrgPlayer {
            account_id: Some(ACCOUNT),
            player_id: PLAYER,
            entity_id: ENTITY,
        }
    }
}

fn row_of(
    capture: &crate::test_support::LogCaptureGuard,
    event: &str,
) -> crate::test_support::Captured {
    capture
        .all()
        .into_iter()
        .find(|c| c.level == Level::INFO && c.has_field("event", event))
        .unwrap_or_else(|| panic!("no INFO {event} row in {:#?}", capture.all()))
}

fn assert_actor_named(row: &crate::test_support::Captured) {
    assert!(row.has_field("account_id", &ACCOUNT.to_string()), "{row:?}");
    assert!(row.has_field("account_name", "sgc_login"), "{row:?}");
    assert!(row.has_field("player_id", &PLAYER.to_string()), "{row:?}");
    assert!(row.has_field("player_name", "Teal'c"), "{row:?}");
    assert!(row.has_field("entity_id", &ENTITY.to_string()), "{row:?}");
    assert!(row.has_field("entity_name", "Teal'c"), "{row:?}");
}

/// `org.leave`: the leave outcome row names the leaver.
#[tokio::test]
async fn leave_outcome_row_names_the_actor() {
    let w = World::new();
    let capture = LogCapture::install();
    let _ = handle_leave(&w.ctx(), &w.player(), 9).await;
    let row = row_of(&capture, "org.leave");
    assert!(row.has_field("outcome", "rejected"), "{row:?}");
    assert_actor_named(&row);
}

/// `org.kick`: the kick outcome row names the kicker.
#[tokio::test]
async fn kick_outcome_row_names_the_actor() {
    let w = World::new();
    let capture = LogCapture::install();
    let _ = handle_kick(&w.ctx(), &w.player(), 9, "Daniel").await;
    let row = row_of(&capture, "org.kick");
    assert!(row.has_field("outcome", "rejected"), "{row:?}");
    assert_actor_named(&row);
}

/// `org.chat`: the chat outcome row names the speaker, and never carries
/// the text.
#[tokio::test]
async fn chat_outcome_row_names_the_speaker() {
    let w = World::new();
    let capture = LogCapture::install();
    let speaker = ChatSpeaker {
        addr: w.addr,
        name: "Teal'c",
        flags: 0,
        account_id: Some(ACCOUNT),
        player_id: Some(PLAYER),
        entity_id: Some(ENTITY),
    };
    relay_org_chat(&w.ctx(), speaker, 3, "secret words").await;
    let row = row_of(&capture, "org.chat");
    assert_actor_named(&row);
    assert!(
        capture
            .all()
            .iter()
            .all(|c| !format!("{c:?}").contains("secret words")),
        "chat text must not be logged"
    );
}
