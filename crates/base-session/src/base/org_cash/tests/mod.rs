//! Treasury deposit and withdrawal tests (bank-vault BV-08; TESTING.md types
//! 2, 3, 5, 8 and 12): live-DB guards on both balances and the cash log,
//! the members' fan-out and the actor's sends decoded call by call, and a
//! `LogCapture` check for every `org_cash_transfer` / `org_cash_rejected`
//! row.
//!
//! Sentinels (the BV-08 block): accounts and characters
//! `0x7000_BE00 + 16 * block + i`, `block` in `0..14`; entities
//! `0x0008_BE00 + 16 * block + i` and ports `42800 + 16 * block + i` (not
//! database rows). Organizations are named `Bv08 <block> Team|Command` and
//! cleaned up by exact name key, their cash log and event rows first.
//! Skip when `DATABASE_URL` is unset.

mod race;
mod refusals;
mod transfers;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::organization::{
    org_text, CashDir, OrgPermission, OrgRank, OrgType, TextField,
};
use cimmeria_mercury::channel_bundle::{EXTENDED_ENCODING_MARKER, IDBASE_SGW_PLAYER};
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::parse_incoming;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tracing::Level;

use super::handle_transfer_cash;
use crate::base::organization::api::{OrgAccess, SystemActor};
use crate::base::organization::handlers::OrgCtx;
use crate::base::organization::persistence::{add_member, create_org};
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{
    test_default_connected_client_state, Captured, LogCaptureGuard, TestTransport,
};

const BASE: i32 = 0x7000_BE00;
const ACTOR: SystemActor<'static> = SystemActor::Server {
    source: "bv08_test",
};

/// `onCashChanged`, `onOrganizationCashUpdate` and `onPlayerCommunication`.
const CASH: u16 = 75;
const ORG_CASH: u16 = 48;
const CHAT: u16 = 28;

/// One decoded client-method call: `(method index, args)`.
type Call = (u16, Vec<u8>);

/// One test's account, characters, sessions and organizations.
pub(super) struct Fx {
    pool: PgPool,
    db_pool: Option<Arc<PgPool>>,
    account_id: i32,
    players: Vec<i32>,
    names: Vec<(OrgType, String)>,
    typed: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
}

impl Fx {
    /// `n` characters on one account in `block`, holding `naquadah` each,
    /// after removing whatever a crashed run left there.
    pub(super) async fn new(pool: &PgPool, block: i32, n: i32, naquadah: i32) -> Fx {
        assert!((0..14).contains(&block) && (1..16).contains(&n));
        let account_id = BASE + 16 * block;
        let typed = Arc::new(TestTransport::new());
        let fx = Fx {
            pool: pool.clone(),
            db_pool: Some(Arc::new(pool.clone())),
            account_id,
            players: (1..=n).map(|i| account_id + i).collect(),
            names: vec![
                (OrgType::Team, format!("Bv08 {block} Team")),
                (OrgType::Command, format!("Bv08 {block} Command")),
            ],
            transport: typed.clone(),
            typed,
            connected: Arc::new(Mutex::new(HashMap::new())),
            entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
            cell_tx: None,
        };
        fx.teardown().await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("bv08-test-{account_id}"))
            .execute(pool)
            .await
            .expect("insert account");
        for &player_id in &fx.players {
            sqlx::query(
                "INSERT INTO sgw_player (\
                    account_id, player_id, level, alignment, archetype, gender, \
                    player_name, extra_name, world_location, bodyset, \
                    pos_x, pos_y, pos_z, skin_color_id, naquadah\
                 ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                           0.0, 0.0, 0.0, 0, $4)",
            )
            .bind(account_id)
            .bind(player_id)
            .bind(format!("bv08-{player_id}"))
            .bind(naquadah)
            .execute(pool)
            .await
            .expect("insert player");
        }
        fx
    }

    pub(super) fn ctx(&self) -> OrgCtx<'_> {
        OrgCtx {
            db_pool: &self.db_pool,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
            cell_tx: &self.cell_tx,
        }
    }

    pub(super) fn player(&self, i: usize) -> i32 {
        self.players[i]
    }

    fn offset(&self, i: usize) -> u32 {
        (self.players[i] - BASE) as u32
    }

    pub(super) fn entity(&self, i: usize) -> u32 {
        0x0008_BE00 + self.offset(i)
    }

    fn addr(&self, i: usize) -> SocketAddr {
        format!("127.0.0.1:{}", 42800 + self.offset(i))
            .parse()
            .unwrap()
    }

    /// Put character `i` in the world: a session listed online.
    pub(super) fn online(&self, i: usize) {
        let mut s = test_default_connected_client_state();
        s.account_id = self.account_id as u32;
        s.active_player_id = Some(self.player(i));
        s.player_entity_id = Some(self.entity(i));
        s.listed_online = true;
        self.connected.lock().unwrap().insert(self.addr(i), s);
        self.entity_to_addr
            .lock()
            .unwrap()
            .insert(self.entity(i), self.addr(i));
    }

    /// Character `i` asks to move `dir` into or out of `org_id`.
    pub(super) async fn transfer(&self, i: usize, org_id: i32, dir: CashDir) {
        handle_transfer_cash(&self.ctx(), self.player(i), self.entity(i), org_id, dir).await;
    }

    /// Create the block's Team (`org_type` Team) or Command led by
    /// character `leader`, with `members` at the type's lowest rank (which
    /// holds `DepositCash` but not `WithdrawCash`, D-BV12).
    pub(super) async fn org(&self, org_type: OrgType, leader: usize, members: &[usize]) -> i32 {
        let name = &self.names.iter().find(|(t, _)| *t == org_type).unwrap().1;
        let mut tx = self.pool.begin().await.unwrap();
        let org = create_org(&mut tx, org_type, name, self.player(leader))
            .await
            .expect("create_org")
            .org_id;
        let actor = OrgAccess::system(&mut tx, org, ACTOR)
            .await
            .unwrap()
            .unwrap();
        let rank = *OrgRank::for_type(org_type).first().unwrap();
        for &m in members {
            add_member(&mut tx, &actor, org, self.player(m), rank)
                .await
                .expect("add_member");
        }
        tx.commit().await.unwrap();
        org
    }

    /// Set, or with `grant = false` clear, `bits` on `org_id`'s lowest rank.
    pub(super) async fn set_member_bits(&self, org_id: i32, bits: OrgPermission, grant: bool) {
        let org_type: i16 =
            sqlx::query_scalar("SELECT org_type FROM sgw_organizations WHERE org_id = $1")
                .bind(org_id)
                .fetch_one(&self.pool)
                .await
                .unwrap();
        let t = if org_type == 1 {
            OrgType::Team
        } else {
            OrgType::Command
        };
        let rank = i16::from(OrgRank::for_type(t).first().unwrap().as_u8());
        let sql = if grant {
            "UPDATE sgw_organization_ranks SET permissions = permissions | $3 \
             WHERE org_id = $1 AND rank = $2"
        } else {
            "UPDATE sgw_organization_ranks SET permissions = permissions & ~$3 \
             WHERE org_id = $1 AND rank = $2"
        };
        sqlx::query(sql)
            .bind(org_id)
            .bind(rank)
            .bind(bits.bits() as i32)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    pub(super) async fn set_org_cash(&self, org_id: i32, cash: i64) {
        sqlx::query("UPDATE sgw_organizations SET cash = $2 WHERE org_id = $1")
            .bind(org_id)
            .bind(cash)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    pub(super) async fn set_wallet(&self, i: usize, naquadah: i32) {
        sqlx::query("UPDATE sgw_player SET naquadah = $2 WHERE player_id = $1")
            .bind(self.player(i))
            .bind(naquadah)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    pub(super) async fn wallet(&self, i: usize) -> i32 {
        sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
            .bind(self.player(i))
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    pub(super) async fn org_cash(&self, org_id: i32) -> i64 {
        sqlx::query_scalar("SELECT cash FROM sgw_organizations WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// Every cash log row of `org_id`, oldest first: `(player_id,
    /// account_id, direction, amount, player_cash_before, player_cash_after,
    /// org_cash_before, org_cash_after)`.
    #[allow(clippy::type_complexity)]
    pub(super) async fn cash_log(
        &self,
        org_id: i32,
    ) -> Vec<(i32, i32, String, i64, Option<i32>, Option<i32>, i64, i64)> {
        sqlx::query_as(
            "SELECT player_id, account_id, direction, amount, player_cash_before, \
                    player_cash_after, org_cash_before, org_cash_after \
             FROM sgw_organization_cash_log WHERE org_id = $1 ORDER BY log_id",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    /// Every client-method call sent to character `i`, in send order.
    pub(super) fn calls_to(&self, i: usize) -> Vec<Call> {
        self.typed
            .filter_to(self.addr(i))
            .iter()
            .flat_map(|p| decode_bundle(p, self.entity(i)))
            .collect()
    }

    /// Drop everything sent so far.
    pub(super) fn clear_sends(&self) {
        self.typed.clear();
    }

    fn name_keys(&self) -> Vec<String> {
        self.names
            .iter()
            .map(|(_, n)| org_text::name_key(&org_text::validate(TextField::Name, n).unwrap()))
            .collect()
    }

    /// Remove everything the block owns, the log and events before the
    /// organizations and the organizations before the characters.
    pub(super) async fn teardown(&self) {
        let orgs: Vec<i32> =
            sqlx::query_scalar("SELECT org_id FROM sgw_organizations WHERE name_key = ANY($1)")
                .bind(self.name_keys())
                .fetch_all(&self.pool)
                .await
                .unwrap();
        for sql in [
            "DELETE FROM sgw_organization_cash_log WHERE org_id = ANY($1)",
            "DELETE FROM sgw_organization_events WHERE org_id = ANY($1)",
            "UPDATE sgw_organizations SET cash = 0 WHERE org_id = ANY($1)",
            "DELETE FROM sgw_organizations WHERE org_id = ANY($1)",
        ] {
            sqlx::query(sql)
                .bind(&orgs)
                .execute(&self.pool)
                .await
                .unwrap();
        }
        sqlx::query("DELETE FROM sgw_organization_cash_log WHERE account_id = $1")
            .bind(self.account_id)
            .execute(&self.pool)
            .await
            .unwrap();
        for &player_id in &self.players {
            sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
                .bind(player_id)
                .execute(&self.pool)
                .await
                .unwrap();
        }
        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(self.account_id)
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

/// Decrypt one packet (the test sessions' all-zero key) and split its body
/// into entity-method calls on `entity_id`. Panics on anything else, so a
/// stray byte fails the test instead of being skipped.
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

/// The feedback lines among `calls`.
fn lines(calls: &[Call]) -> Vec<String> {
    calls
        .iter()
        .filter(|c| c.0 == CHAT)
        .map(|(_, args)| {
            let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
            let mut o = 4 + speaker_len * 2 + 2;
            let n = u32::from_le_bytes(args[o..o + 4].try_into().unwrap()) as usize;
            o += 4;
            let units: Vec<u16> = (0..n)
                .map(|i| u16::from_le_bytes(args[o + 2 * i..o + 2 * i + 2].try_into().unwrap()))
                .collect();
            String::from_utf16_lossy(&units)
        })
        .collect()
}

/// The `onCashChanged` balances among `calls`.
fn cash_updates(calls: &[Call]) -> Vec<i32> {
    calls
        .iter()
        .filter(|c| c.0 == CASH)
        .map(|c| i32::from_le_bytes(c.1[..4].try_into().unwrap()))
        .collect()
}

/// The `onOrganizationCashUpdate` args among `calls`, as `(org_id, cash)`.
fn org_cash_updates(calls: &[Call]) -> Vec<(i32, u64)> {
    calls
        .iter()
        .filter(|c| c.0 == ORG_CASH)
        .map(|c| {
            (
                i32::from_le_bytes(c.1[..4].try_into().unwrap()),
                u64::from_le_bytes(c.1[4..12].try_into().unwrap()),
            )
        })
        .collect()
}

/// The `bank` rows named `event`.
fn bank_rows(capture: &LogCaptureGuard, event: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", event))
        .collect()
}

/// Exactly one `event` row at `level` for character `i`, with the
/// correlators and every `(field, value)` in `want`.
fn one(
    fx: &Fx,
    capture: &LogCaptureGuard,
    event: &str,
    level: Level,
    i: usize,
    want: &[(&str, &str)],
) -> Captured {
    let found = bank_rows(capture, event);
    assert_eq!(found.len(), 1, "exactly one {event}: {found:#?}");
    let row = found.into_iter().next().unwrap();
    assert_eq!(row.level, level, "{event} level: {row:#?}");
    let account = fx.account_id.to_string();
    let player = fx.player(i).to_string();
    let entity = fx.entity(i).to_string();
    for (k, v) in [
        ("account_id", account.as_str()),
        ("player_id", player.as_str()),
        ("entity_id", entity.as_str()),
    ]
    .iter()
    .chain(want)
    {
        assert!(row.has_field(k, v), "{event}: {k}={v} missing: {row:#?}");
    }
    row
}
