//! Team vault expansion from the treasury (bank-vault BV-09; TESTING.md
//! types 2, 3, 5 and 12): live-DB guards on the size, the treasury and the
//! cash log, the buyer's `onBagInfo` and the members' treasury update
//! matched byte for byte, and a `LogCapture` check on every `expand` /
//! `expand_rejected` / `org_cash_transfer` row.
//!
//! Sentinels (the BV-09 block): accounts and characters
//! `0x7000_BF00 + 16 * block + i`, `block` in `0..14`; entities
//! `0x0008_BF00 + 16 * block + i` and ports `43200 + 16 * block + i` (not
//! database rows). Organizations are named `Bv09 <block> Team|Command` and
//! cleaned up by exact name key, their cash log and event rows first. The
//! prices are the seeded `resources.bank_expansion_price` rows (100 a step).
//! Skip when `DATABASE_URL` is unset.
//!
//! Not tested here: a missing or zero price (`price_missing`). The price
//! table is shared seed data (`resources.bank_expansion_price`, one row per
//! step, every one 100), so a test that removed a row would race every
//! other test reading it; the refusal is the same `filter(p > 0)` shape as
//! BV-05's guarded `price_missing`, and the cash log's `amount > 0` CHECK
//! stops a free step even without it.

mod purchase;
mod refusals;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::organization::api::{OrgAccess, SystemActor};
use cimmeria_base_session::base::organization::handlers::OrgCtx;
use cimmeria_base_session::base::organization::persistence::{add_member, create_org};
use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::organization::{org_text, OrgRank, OrgType, TextField};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tracing::Level;

use super::super::expand::{handle_org_vault_expand, OrgVaultExpandRequest};
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{
    test_default_connected_client_state, Captured, LogCaptureGuard, TestTransport,
};

const BASE: i32 = 0x7000_BF00;
const ACTOR: SystemActor<'static> = SystemActor::Server {
    source: "bv09_test",
};

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
    /// `n` characters on one account in `block`, after removing whatever a
    /// crashed run left there.
    pub(super) async fn new(pool: &PgPool, block: i32, n: i32) -> Fx {
        assert!((0..14).contains(&block) && (1..16).contains(&n));
        let account_id = BASE + 16 * block;
        let typed = Arc::new(TestTransport::new());
        let fx = Fx {
            pool: pool.clone(),
            db_pool: Some(Arc::new(pool.clone())),
            account_id,
            players: (1..=n).map(|i| account_id + i).collect(),
            names: vec![
                (OrgType::Team, format!("Bv09 {block} Team")),
                (OrgType::Command, format!("Bv09 {block} Command")),
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
            .bind(format!("bv09-test-{account_id}"))
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
                           0.0, 0.0, 0.0, 0, 500)",
            )
            .bind(account_id)
            .bind(player_id)
            .bind(format!("bv09-{player_id}"))
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
        0x0008_BF00 + self.offset(i)
    }

    pub(super) fn addr(&self, i: usize) -> SocketAddr {
        format!("127.0.0.1:{}", 43200 + self.offset(i))
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

    /// Character `i` types `.orgvaultexpand [scope] [from]`.
    pub(super) async fn expand(&self, i: usize, scope: VaultScope, from_slots: Option<i16>) {
        let req = OrgVaultExpandRequest {
            entity_id: self.entity(i),
            account_id: Some(self.account_id as u32),
            player_id: self.player(i),
            scope,
            from_slots,
        };
        handle_org_vault_expand(req, &self.ctx()).await;
    }

    /// Create the block's Team or Command led by character `leader`, with
    /// `members` at the type's lowest rank, holding `cash`.
    pub(super) async fn org(
        &self,
        org_type: OrgType,
        leader: usize,
        members: &[usize],
        cash: i64,
    ) -> i32 {
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
        sqlx::query("UPDATE sgw_organizations SET cash = $2 WHERE org_id = $1")
            .bind(org)
            .bind(cash)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        org
    }

    pub(super) async fn set_slots(&self, org_id: i32, slots: i16) {
        sqlx::query("UPDATE sgw_organizations SET vault_slots = $2 WHERE org_id = $1")
            .bind(org_id)
            .bind(slots)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    /// `(vault_slots, cash)` as committed.
    pub(super) async fn state(&self, org_id: i32) -> (i16, i64) {
        sqlx::query_as("SELECT vault_slots, cash FROM sgw_organizations WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// Every cash log row of `org_id`: `(player_id, direction, amount,
    /// player_cash_before, org_cash_before, org_cash_after,
    /// vault_slots_before, vault_slots_after)`.
    #[allow(clippy::type_complexity)]
    pub(super) async fn cash_log(
        &self,
        org_id: i32,
    ) -> Vec<(
        i32,
        String,
        i64,
        Option<i32>,
        i64,
        i64,
        Option<i16>,
        Option<i16>,
    )> {
        sqlx::query_as(
            "SELECT player_id, direction, amount, player_cash_before, org_cash_before, \
                    org_cash_after, vault_slots_before, vault_slots_after \
             FROM sgw_organization_cash_log WHERE org_id = $1 ORDER BY log_id",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    /// Every packet to character `i`, decrypted (the test key is all zeros).
    fn plaintexts(&self, i: usize) -> Vec<Vec<u8>> {
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        self.typed
            .filter_to(self.addr(i))
            .iter()
            .filter_map(|p| enc.decrypt(p).ok())
            .collect()
    }

    /// Whether any packet to character `i` carries `bytes`.
    pub(super) fn saw_bytes(&self, i: usize, bytes: &[u8]) -> bool {
        self.plaintexts(i)
            .iter()
            .any(|p| p.windows(bytes.len()).any(|w| w == bytes))
    }

    /// Whether any packet to character `i` carries `text` as UTF-16LE.
    pub(super) fn saw_text(&self, i: usize, text: &str) -> bool {
        let want: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        self.saw_bytes(i, &want)
    }

    pub(super) fn clear_sends(&self) {
        self.typed.clear();
    }

    fn name_keys(&self) -> Vec<String> {
        self.names
            .iter()
            .map(|(_, n)| org_text::name_key(&org_text::validate(TextField::Name, n).unwrap()))
            .collect()
    }

    /// Remove everything the block owns.
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
