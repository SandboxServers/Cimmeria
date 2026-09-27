//! ORG-06 handler tests: login push, presence, leave and disband.
//!
//! Live-DB (TESTING.md type 3) against the real schema and trigger, with
//! every client-bound message captured from a `TestTransport`, decrypted and
//! compared byte for byte (types 2 and 8), and every refusal checked with
//! `LogCapture` (type 12). Sentinels: this module owns
//! `0x7000_4C00..=0x7000_4DFF` for account and player ids (32 blocks of
//! 16); organizations are cleaned by exact name key ("Org06 ..." names).

mod disband;
mod leave;
mod presence;
mod push;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_entity::organization::{org_text, OrgRank, OrgType, TextField};
use cimmeria_mercury::channel_bundle::{EXTENDED_ENCODING_MARKER, IDBASE_SGW_PLAYER};
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::parse_incoming;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::{OrgCtx, OrgPlayer};
use crate::base::organization::api::{OrgAccess, SystemActor};
use crate::base::organization::persistence::{add_member, create_org};
use crate::base::ConnectedClientState;
use crate::cell::messages::{BaseToCellMsg, OrgBaseToCell};
use crate::test_support::{test_default_connected_client_state, TestTransport};
use cimmeria_entity::organization::OrgLeaveReason;

const BASE: i32 = 0x7000_4C00;

/// One decoded client-method call: `(method index, args)`.
type Call = (u16, Vec<u8>);

/// One test's characters, their sessions and the capture.
struct Fixture {
    pool: PgPool,
    db_pool: Option<Arc<PgPool>>,
    account_id: i32,
    players: Vec<i32>,
    name_keys: Vec<String>,
    typed: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: Mutex<mpsc::Receiver<BaseToCellMsg>>,
}

impl Fixture {
    /// One account with `n` characters in block `block` (0..32), named
    /// `Org06P<i>` with level `10 + i` and archetype `i`. No session yet.
    async fn new(pool: &PgPool, block: i32, n: i32, org_names: &[&str]) -> Self {
        assert!((0..32).contains(&block) && (1..16).contains(&n));
        let account_id = BASE + block * 16;
        let typed = Arc::new(TestTransport::new());
        let (cell_tx, cell_rx) = mpsc::channel(64);
        let fx = Self {
            pool: pool.clone(),
            db_pool: Some(Arc::new(pool.clone())),
            account_id,
            players: (1..=n).map(|i| account_id + i).collect(),
            name_keys: org_names
                .iter()
                .map(|n| org_text::name_key(&org_text::validate(TextField::Name, n).unwrap()))
                .collect(),
            transport: typed.clone(),
            typed,
            connected: Arc::new(Mutex::new(HashMap::new())),
            entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
            cell_tx: Some(cell_tx),
            cell_rx: Mutex::new(cell_rx),
        };
        fx.teardown().await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("org06-test-{account_id}"))
            .execute(pool)
            .await
            .expect("insert account");
        for (i, &player_id) in fx.players.iter().enumerate() {
            sqlx::query(
                "INSERT INTO sgw_player (\
                    account_id, player_id, level, alignment, archetype, gender, \
                    player_name, extra_name, world_location, bodyset, \
                    pos_x, pos_y, pos_z, skin_color_id\
                 ) VALUES ($1, $2, $3, 0, $4, 1, $5, '', 'CombatSim', \
                           'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0)",
            )
            .bind(account_id)
            .bind(player_id)
            .bind(10 + i as i32)
            .bind(i as i32)
            .bind(fx.name(i))
            .execute(pool)
            .await
            .expect("insert player");
        }
        fx
    }

    fn ctx(&self) -> OrgCtx<'_> {
        OrgCtx {
            db_pool: &self.db_pool,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
            cell_tx: &self.cell_tx,
        }
    }

    /// Every `OrgMembershipEnded` sent to the cell so far, as
    /// `(player_id, entity_id, org_id, reason)`.
    fn memberships_ended(&self) -> Vec<(i32, u32, i32, OrgLeaveReason)> {
        let mut rx = self.cell_rx.lock().unwrap();
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let BaseToCellMsg::Org(OrgBaseToCell::OrgMembershipEnded {
                player_id,
                entity_id,
                org_id,
                reason,
            }) = msg
            {
                out.push((player_id, entity_id, org_id, reason));
            }
        }
        out
    }

    fn name(&self, i: usize) -> String {
        format!("Org06P{}", self.players[i] - BASE)
    }

    fn player_id(&self, i: usize) -> i32 {
        self.players[i]
    }

    /// Character `i`'s entity id (distinct from every player id).
    fn entity(&self, i: usize) -> u32 {
        0x0006_0000 + (self.players[i] - BASE) as u32
    }

    fn addr(&self, i: usize) -> SocketAddr {
        format!("127.0.0.1:{}", 40000 + (self.players[i] - BASE))
            .parse()
            .unwrap()
    }

    fn player(&self, i: usize) -> OrgPlayer {
        OrgPlayer {
            account_id: Some(self.account_id as u32),
            player_id: self.player_id(i),
            entity_id: self.entity(i),
        }
    }

    /// Put character `i` in the world: a session listed online.
    fn online(&self, i: usize) {
        let mut s = test_default_connected_client_state();
        s.account_id = self.account_id as u32;
        s.active_player_id = Some(self.player_id(i));
        s.player_entity_id = Some(self.entity(i));
        s.player_name = Some(self.name(i));
        s.listed_online = true;
        self.connected.lock().unwrap().insert(self.addr(i), s);
        self.entity_to_addr
            .lock()
            .unwrap()
            .insert(self.entity(i), self.addr(i));
    }

    /// Every client-method call sent to character `i`, in send order.
    fn calls_to(&self, i: usize) -> Vec<Call> {
        self.typed
            .filter_to(self.addr(i))
            .iter()
            .flat_map(|p| decode_bundle(p, self.entity(i)))
            .collect()
    }

    /// The calls sent to character `i`, split per packet (one bundle each).
    fn bundles_to(&self, i: usize) -> Vec<Vec<Call>> {
        self.typed
            .filter_to(self.addr(i))
            .iter()
            .map(|p| decode_bundle(p, self.entity(i)))
            .collect()
    }

    /// Wait until character `i` has received `n` packets (a spawned
    /// fanout), or panic after five seconds.
    async fn wait_for_packets(&self, i: usize, n: usize) {
        for _ in 0..500 {
            if self.typed.filter_to(self.addr(i)).len() >= n {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!(
            "character {i} received {} packets, expected {n}",
            self.typed.filter_to(self.addr(i)).len()
        );
    }

    /// Create an organization led by character `leader`, with `members`
    /// added at the type's lowest rank.
    async fn org(&self, org_type: OrgType, name: &str, leader: usize, members: &[usize]) -> i32 {
        let mut tx = self.pool.begin().await.unwrap();
        let org_id = create_org(&mut tx, org_type, name, self.player_id(leader))
            .await
            .expect("create_org")
            .org_id;
        let actor = OrgAccess::system(
            &mut tx,
            org_id,
            SystemActor::Server {
                source: "org06_test",
            },
        )
        .await
        .unwrap()
        .unwrap();
        let rank = *OrgRank::for_type(org_type).first().unwrap();
        for &m in members {
            add_member(&mut tx, &actor, org_id, self.player_id(m), rank)
                .await
                .expect("add_member");
        }
        tx.commit().await.unwrap();
        org_id
    }

    async fn member_ids(&self, org_id: i32) -> Vec<i32> {
        sqlx::query_scalar(
            "SELECT player_id FROM sgw_organization_members WHERE org_id = $1 ORDER BY player_id",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn org_exists(&self, org_id: i32) -> bool {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sgw_organizations WHERE org_id = $1)")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    async fn teardown(&self) {
        for key in &self.name_keys {
            sqlx::query("DELETE FROM sgw_organizations WHERE name_key = $1")
                .bind(key)
                .execute(&self.pool)
                .await
                .expect("cleanup organizations");
        }
        sqlx::query("DELETE FROM sgw_organization_events WHERE from_account_id = $1")
            .bind(self.account_id)
            .execute(&self.pool)
            .await
            .expect("cleanup organization events");
        for &player_id in &self.players {
            sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
                .bind(player_id)
                .execute(&self.pool)
                .await
                .expect("cleanup player");
        }
        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(self.account_id)
            .execute(&self.pool)
            .await
            .expect("cleanup account");
    }
}

/// Decrypt one packet (the test sessions' all-zero key) and split its body
/// into entity-method calls on `entity_id`: `[0x80 | index][len u16][entity
/// u32][args]` below the player's idbase, `[0xBD][len][entity][index -
/// idbase][args]` at or above it. Panics on anything else, so a stray byte
/// fails the test instead of being skipped.
fn decode_bundle(packet: &[u8], entity_id: u32) -> Vec<Call> {
    let pt = MercuryEncryption::from_session_key([0u8; 32])
        .decrypt(packet)
        .expect("decrypt test packet");
    let parsed = parse_incoming(&pt).expect("parse test packet");
    let body: &[u8] = &parsed.body;
    let mut out = Vec::new();
    let mut o = 0;
    while o < body.len() {
        let id = body[o];
        let len = usize::from(u16::from_le_bytes([body[o + 1], body[o + 2]]));
        let payload = &body[o + 3..o + 3 + len];
        assert_eq!(
            u32::from_le_bytes(payload[..4].try_into().unwrap()),
            entity_id,
            "call addressed to another entity"
        );
        if id == EXTENDED_ENCODING_MARKER {
            out.push((
                u16::from(IDBASE_SGW_PLAYER) + u16::from(payload[4]),
                payload[5..].to_vec(),
            ));
        } else {
            assert!(id & 0x80 != 0, "not an entity-method call: {id:#04x}");
            out.push((u16::from(id & 0x7F), payload[4..].to_vec()));
        }
        o += 3 + len;
    }
    out
}

/// The text of a feedback line (`onPlayerCommunication` [28]).
fn feedback_text(args: &[u8]) -> String {
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let mut o = 4 + speaker_len * 2 + 2;
    let n = u32::from_le_bytes(args[o..o + 4].try_into().unwrap()) as usize;
    o += 4;
    let units: Vec<u16> = (0..n)
        .map(|i| u16::from_le_bytes(args[o + 2 * i..o + 2 * i + 2].try_into().unwrap()))
        .collect();
    String::from_utf16_lossy(&units)
}

/// Every feedback line in `calls`.
fn feedback_lines(calls: &[Call]) -> Vec<String> {
    calls
        .iter()
        .filter(|c| c.0 == 28)
        .map(|c| feedback_text(&c.1))
        .collect()
}
