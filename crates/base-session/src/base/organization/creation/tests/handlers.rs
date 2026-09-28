//! The creation handlers end to end, with a `TestTransport` session and a
//! live database (TESTING.md types 3, 8 and 12): the packets the founder's
//! client receives, in order, what the cell is told, and the one outcome
//! row per action.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::organization::{OrgRank, OrgType};
use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use super::super::super::persistence::load_roster;
use super::super::handler::{
    already_in_type_text, founded_text, handle_create, handle_gm_create, handle_registrar_open,
    CreationCtx, NAME_TAKEN_TEXT,
};
use super::{memberships, setup, teardown};
use crate::base::ConnectedClientState;
use crate::cell::messages::{BaseToCellMsg, OrgBaseToCell};
use crate::mercury::build_player_entity_method_packet;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, Captured, LogCapture, TestTransport,
};

const ENTITY: u32 = 0x5C05;
const NPC: u32 = 0x5C06;

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

/// One client session for `player_id` at `access_level`, with its client
/// transport and the cell channel's receiver.
struct Session {
    typed: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    addr: SocketAddr,
    connected: Connected,
    entity_to_addr: EntityToAddr,
    pool: Option<Arc<PgPool>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
}

impl Session {
    fn new(pool: &PgPool, account_id: i32, player_id: i32, access_level: u32) -> Self {
        let typed = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed.clone();
        let addr: SocketAddr = "127.0.0.1:55705".parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.account_id = account_id as u32;
        state.active_player_id = Some(player_id);
        state.access_level = access_level;
        state.player_entity_id = Some(ENTITY);
        let (cell_tx, cell_rx) = mpsc::channel(16);
        Self {
            typed,
            transport,
            addr,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
            pool: Some(Arc::new(pool.clone())),
            cell_tx: Some(cell_tx),
            cell_rx,
        }
    }

    fn ctx(&self) -> CreationCtx<'_> {
        CreationCtx {
            db_pool: &self.pool,
            cell_tx: &self.cell_tx,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }

    fn sent(&self) -> Vec<Vec<u8>> {
        self.typed.filter_to(self.addr)
    }

    fn cell(&mut self) -> Vec<OrgBaseToCell> {
        let mut out = Vec::new();
        while let Ok(msg) = self.cell_rx.try_recv() {
            match msg {
                BaseToCellMsg::Org(o) => out.push(o),
                _ => panic!("unexpected non-organization cell message"),
            }
        }
        out
    }
}

/// `(method, args)` as the transport records the `seq`-th reliable send.
fn packets(calls: &[(u16, Vec<u8>)]) -> Vec<Vec<u8>> {
    calls
        .iter()
        .enumerate()
        .map(|(seq, (method, args))| {
            build_player_entity_method_packet(
                &[0u8; 32],
                seq as u32,
                &[],
                ENTITY,
                *method,
                args,
                EncryptionVersion::V1,
            )
        })
        .collect()
}

fn line(text: &str) -> (u16, Vec<u8>) {
    (
        28,
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text),
    )
}

fn rows(capture: &crate::test_support::LogCaptureGuard, event: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "org" && c.level == Level::INFO && c.has_field("event", event))
        .filter(|c| c.fields.contains_key("outcome"))
        .collect()
}

/// A named creation: the founder's client gets 134 `(1, 0)` first, then
/// ORG-06's state push (`push_org_state`, `new_member = true`: one reliable
/// bundle, whose bytes ORG-06's tests pin), then the confirmation line. The
/// cell is told `created`; one `org.create` row says `ok` with the new id
/// and the actor; one `org.state_push` row names the new organization with
/// the founder online.
#[tokio::test]
async fn live_db_create_pushes_the_new_organization_and_tells_the_cell() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 6, &[0], &["Org05 Push"]).await;
    let player = fx.player(0);
    let mut s = Session::new(&pool, fx.account_id, player, 0);
    let capture = LogCapture::install();

    handle_create(&s.ctx(), player, ENTITY, OrgType::Command, "Org05 Push").await;

    let org_id: i32 =
        sqlx::query_scalar("SELECT org_id FROM sgw_organization_members WHERE player_id = $1")
            .bind(player)
            .fetch_one(&pool)
            .await
            .expect("the founder is a member");
    let roster = load_roster(&pool, org_id).await.unwrap();
    teardown(&pool, &fx).await;

    let sent = s.sent();
    assert!(
        sent.len() >= 3,
        "134, the state bundle and the line: {sent:?}"
    );
    assert_eq!(sent[0], packets(&[(134, vec![1, 0])])[0], "134 goes first");
    let last = sent.len() - 1;
    let founded = line(&founded_text(OrgType::Command, "Org05 Push"));
    assert_eq!(
        sent[last],
        build_player_entity_method_packet(
            &[0u8; 32],
            last as u32,
            &[],
            ENTITY,
            founded.0,
            &founded.1,
            EncryptionVersion::V1,
        ),
        "the confirmation line goes last"
    );
    assert_eq!(roster.len(), 1);
    assert_eq!(roster[0].rank, OrgRank::LEADER);
    let pushes: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "org.state_push"))
        .collect();
    assert_eq!(pushes.len(), 1, "{pushes:#?}");
    for (k, v) in [
        ("org_id", org_id.to_string()),
        ("new_member", "true".into()),
        ("roster_size", "1".into()),
        ("online_members", "1".into()),
        ("player_id", player.to_string()),
    ] {
        assert!(
            pushes[0].has_field(k, &v),
            "{k}={v}: {:?}",
            pushes[0].fields
        );
    }
    assert_eq!(
        s.cell(),
        vec![OrgBaseToCell::CreateResult {
            player_id: player,
            entity_id: ENTITY,
            org_type: OrgType::Command,
            created: true,
        }]
    );
    let got = rows(&capture, "org.create");
    assert_eq!(got.len(), 1, "{got:#?}");
    let r = &got[0];
    assert!(r.has_field("outcome", "ok"));
    assert!(r.has_field("org_id", &org_id.to_string()));
    assert!(r.has_field("org_type", "command"), "{r:#?}");
    assert!(r.has_field("account_id", &fx.account_id.to_string()));
    assert!(r.has_field("player_id", &player.to_string()));
    assert!(r.has_field("name_units", "10"));
    assert!(
        !r.fields.contains_key("cost_before"),
        "free creation logs no cost"
    );
}

/// CAT-M-03 on the handler: a taken name is answered with 134 `(0,
/// NAME_TAKEN)` and a line, the cell is told to charge the attempt, the row
/// says `name_taken`, and the second player joins nothing.
#[tokio::test]
async fn live_db_create_duplicate_name_is_refused_visibly() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 7, &[0, 0], &["Org05 Taken"]).await;
    let (first, second) = (fx.player(0), fx.player(1));
    super::super::found_organization(&pool, OrgType::Team, "Org05 Taken", first)
        .await
        .expect("the first founder");
    let mut s = Session::new(&pool, fx.account_id, second, 0);
    let capture = LogCapture::install();

    handle_create(&s.ctx(), second, ENTITY, OrgType::Team, "ORG05 taken").await;
    let joined = memberships(&pool, second).await;
    teardown(&pool, &fx).await;

    assert_eq!(
        s.sent(),
        packets(&[(134, vec![0, 1]), line(NAME_TAKEN_TEXT)])
    );
    assert_eq!(
        s.cell(),
        vec![OrgBaseToCell::CreateResult {
            player_id: second,
            entity_id: ENTITY,
            org_type: OrgType::Team,
            created: false,
        }]
    );
    assert_eq!(joined, 0);
    let got = rows(&capture, "org.create");
    assert_eq!(got.len(), 1, "{got:#?}");
    assert!(got[0].has_field("outcome", "rejected"));
    assert!(got[0].has_field("reason", "name_taken"));
}

/// The registrar's eligibility check (D-ORG18): an eligible player's click
/// goes back to the cell as `RegistrarEligible` with nothing sent to the
/// client yet (the cell opens the dialog and writes the row); a player who
/// already leads a Team gets a line and a `not_eligible` row, and the cell
/// hears nothing.
#[tokio::test]
async fn live_db_registrar_open_checks_eligibility_per_type() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 8, &[0], &["Org05 Elig"]).await;
    let player = fx.player(0);
    let mut s = Session::new(&pool, fx.account_id, player, 0);

    handle_registrar_open(&s.ctx(), player, ENTITY, NPC, OrgType::Team).await;
    assert!(s.sent().is_empty());
    assert_eq!(
        s.cell(),
        vec![OrgBaseToCell::RegistrarEligible {
            player_id: player,
            entity_id: ENTITY,
            npc_entity_id: NPC,
            org_type: OrgType::Team,
        }]
    );

    super::super::found_organization(&pool, OrgType::Team, "Org05 Elig", player)
        .await
        .unwrap();
    let capture = LogCapture::install();
    handle_registrar_open(&s.ctx(), player, ENTITY, NPC, OrgType::Team).await;
    handle_registrar_open(&s.ctx(), player, ENTITY, NPC, OrgType::Command).await;
    teardown(&pool, &fx).await;

    assert_eq!(
        s.sent(),
        packets(&[line(&already_in_type_text(OrgType::Team))])
    );
    assert_eq!(
        s.cell(),
        vec![OrgBaseToCell::RegistrarEligible {
            player_id: player,
            entity_id: ENTITY,
            npc_entity_id: NPC,
            org_type: OrgType::Command,
        }],
        "a Team leader may still found a Command"
    );
    let got = rows(&capture, "org.registrar_open");
    assert_eq!(got.len(), 1, "only the refusal is the base's row: {got:#?}");
    assert!(got[0].has_field("reason", "not_eligible"));
    assert!(got[0].has_field("npc_entity_id", &NPC.to_string()));
    assert!(got[0].has_field("org_type", "team"));
}

/// D-ORG13: `.org_create` is honoured only for a session at GameMaster or
/// above, read from the base's own session. A player is refused with a
/// line and founds nothing; a GM founds and gets the push, and the cell is
/// told nothing (there is no pending creation to settle).
#[tokio::test]
async fn live_db_gm_create_rechecks_the_access_level_on_the_base() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 9, &[0], &["Org05 Gm"]).await;
    let player = fx.player(0);
    let capture = LogCapture::install();

    let mut player_session = Session::new(&pool, fx.account_id, player, 0);
    handle_gm_create(
        &player_session.ctx(),
        player,
        ENTITY,
        OrgType::Team,
        "Org05 Gm",
    )
    .await;
    let after_refusal = memberships(&pool, player).await;

    let mut gm = Session::new(&pool, fx.account_id, player, 2);
    handle_gm_create(&gm.ctx(), player, ENTITY, OrgType::Team, "Org05 Gm").await;
    let after_gm = memberships(&pool, player).await;
    teardown(&pool, &fx).await;

    assert_eq!(after_refusal, 0);
    assert_eq!(
        player_session.sent(),
        packets(&[line(".org_create is a GM command.")])
    );
    assert!(player_session.cell().is_empty());
    assert_eq!(after_gm, 1);
    assert!(gm.sent().len() >= 3, "134, the state bundle and the line");
    assert!(gm.cell().is_empty());
    let got = rows(&capture, "org.gm_action");
    assert_eq!(got.len(), 2, "{got:#?}");
    assert!(got[0].has_field("reason", "not_gm"));
    assert!(got[1].has_field("outcome", "ok"));
    assert!(got.iter().all(|r| r.has_field("action", "gm_org_create")));
}

/// A message for an entity that is no longer the named character acts for
/// nobody: WARN `org.actor_mismatch`, a `rejected` row, no database write,
/// and the cell is still told, so the offer is not left in flight.
#[tokio::test]
async fn live_db_create_for_a_recycled_entity_acts_for_nobody() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 10, &[0], &["Org05 Stale"]).await;
    let player = fx.player(0);
    // The session now plays someone else.
    let mut s = Session::new(&pool, fx.account_id, player + 1000, 0);
    let capture = LogCapture::install();

    handle_create(&s.ctx(), player, ENTITY, OrgType::Team, "Org05 Stale").await;
    let joined = memberships(&pool, player).await;
    teardown(&pool, &fx).await;

    assert_eq!(joined, 0);
    assert!(s.sent().is_empty());
    assert!(matches!(
        s.cell().as_slice(),
        [OrgBaseToCell::CreateResult { created: false, .. }]
    ));
    assert!(capture.all().iter().any(|c| c.level == Level::WARN
        && c.has_field("event", "org.actor_mismatch")
        && c.has_field("reason", "actor_mismatch")));
    let got = rows(&capture, "org.create");
    assert_eq!(got.len(), 1);
    assert!(got[0].has_field("reason", "actor_mismatch"));
}
