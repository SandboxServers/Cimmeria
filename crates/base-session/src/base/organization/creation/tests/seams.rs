//! The base's negative seams (TESTING.md type 12): a database that fails, a
//! client send that fails and a cell that cannot be reached each log a WARN
//! with `reason`, and the action still ends in one `rejected` row. No live
//! database: the pool is one that cannot connect.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::organization::OrgType;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tracing::Level;

use super::super::handler::{handle_create, handle_registrar_open, CreationCtx};
use crate::test_support::{test_default_connected_client_state, Captured, LogCapture};

const ENTITY: u32 = 0x5C07;
const PLAYER_ID: i32 = 0x5C08;
const ACCOUNT_ID: u32 = 0x5C09;

/// A transport that refuses every send.
struct FailingTransport;

impl Transport for FailingTransport {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _bytes: &'life1 [u8],
        _addr: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async { Err(std::io::Error::other("link down")) })
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

fn unreachable_pool() -> Option<Arc<PgPool>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    Some(Arc::new(pool))
}

type Connected = Arc<Mutex<HashMap<SocketAddr, crate::base::ConnectedClientState>>>;

fn session() -> (Connected, Arc<Mutex<HashMap<u32, SocketAddr>>>) {
    let addr: SocketAddr = "127.0.0.1:55706".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT_ID;
    state.active_player_id = Some(PLAYER_ID);
    state.player_entity_id = Some(ENTITY);
    (
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
    )
}

fn warn(all: &[Captured], event: &str) -> Captured {
    all.iter()
        .find(|c| c.level == Level::WARN && c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("WARN {event} not logged: {all:#?}"))
}

fn row(all: &[Captured], event: &str) -> Captured {
    let rows: Vec<&Captured> = all
        .iter()
        .filter(|c| c.level == Level::INFO && c.has_field("event", event))
        .filter(|c| c.fields.contains_key("outcome"))
        .collect();
    assert_eq!(rows.len(), 1, "{event}: {rows:#?}");
    rows[0].clone()
}

/// A creation whose database fails: WARN `org.create_failed` (`db_error`),
/// the 134 refusal cannot be sent (WARN `org.send_failed`, `send_error`,
/// method 134), the cell cannot be told (WARN `org.cell_send_failed`,
/// `cell_unreachable`), and one `org.create` row says `db_error`.
#[tokio::test]
async fn a_failing_database_client_and_cell_each_warn() {
    let (connected, entity_to_addr) = session();
    let transport: Arc<dyn Transport> = Arc::new(FailingTransport);
    let pool = unreachable_pool();
    let ctx = CreationCtx {
        db_pool: &pool,
        cell_tx: &None,
        transport: &transport,
        connected: &connected,
        entity_to_addr: &entity_to_addr,
    };
    let capture = LogCapture::install();
    cimmeria_observability::testing::install();
    let labels = [
        ("action", "create"),
        ("outcome", "rejected"),
        ("reason", "db_error"),
    ];
    let counted = cimmeria_observability::testing::counter_total("org_actions_total", &labels);

    handle_create(&ctx, PLAYER_ID, ENTITY, OrgType::Team, "Org05 Seam").await;

    assert_eq!(
        cimmeria_observability::testing::counter_total("org_actions_total", &labels) - counted,
        1,
        "the row counts once on org_actions_total"
    );
    let all = capture.all();
    assert!(warn(&all, "org.create_failed").has_field("reason", "db_error"));
    let send = warn(&all, "org.send_failed");
    assert!(send.has_field("reason", "send_error") && send.has_field("method_index", "134"));
    let cell = warn(&all, "org.cell_send_failed");
    assert!(
        cell.has_field("reason", "cell_unreachable") && cell.has_field("kind", "create_result")
    );
    let r = row(&all, "org.create");
    assert!(r.has_field("reason", "db_error"));
    assert!(r.has_field("account_id", &ACCOUNT_ID.to_string()));
    assert!(r.has_field("player_id", &PLAYER_ID.to_string()));
}

/// The registrar's eligibility read failing: WARN
/// `org.registrar_open_lookup_failed` (`db_error`) and one `db_error` row;
/// the cell is never asked to open the dialog.
#[tokio::test]
async fn a_failing_eligibility_read_warns_and_opens_nothing() {
    let (connected, entity_to_addr) = session();
    let transport: Arc<dyn Transport> = Arc::new(FailingTransport);
    let pool = unreachable_pool();
    let (cell_tx, mut cell_rx) = tokio::sync::mpsc::channel(4);
    let cell_tx = Some(cell_tx);
    let ctx = CreationCtx {
        db_pool: &pool,
        cell_tx: &cell_tx,
        transport: &transport,
        connected: &connected,
        entity_to_addr: &entity_to_addr,
    };
    let capture = LogCapture::install();

    handle_registrar_open(&ctx, PLAYER_ID, ENTITY, 77, OrgType::Command).await;

    let all = capture.all();
    assert!(warn(&all, "org.registrar_open_lookup_failed").has_field("reason", "db_error"));
    assert!(row(&all, "org.registrar_open").has_field("reason", "db_error"));
    assert!(
        cell_rx.try_recv().is_err(),
        "the cell must not open a dialog"
    );
}
