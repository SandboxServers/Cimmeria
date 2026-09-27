//! Live-DB tests for the Team and Command vaults on the base (bank-vault
//! BV-07; TESTING.md types 2, 3 and 12).
//!
//! Sentinels (the BV-07 block): accounts and players
//! `0x7000_B800 + 16 * block`, `block` in `0..18`; item ids (vault or
//! carried) `0x7000_B900 + 4 * block + k`; entities
//! `0x7000_B9E0 + 2 * block + i`; item types
//! `0x7000_B980` (bankable, `{1,17}`, max stack 20), `0x7000_B981`
//! (a mission item that also lists 17), `0x7000_B982` (carried only,
//! `{1}`). Organizations are named `Bv07 <block> <kind>` and cleaned up by
//! exact name key; their vault and log rows first, by org id. Ports
//! `40870 + block`. Skip when `DATABASE_URL` is unset.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::organization::api::{OrgAccess, SystemActor};
use cimmeria_base_session::base::organization::persistence::{add_member, create_org};
use cimmeria_entity::organization::{org_text, OrgRank, OrgType, TextField};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, TestTransport};

mod delete_race;
mod fanout;
mod move_bits;
mod move_shapes;
mod moves;
mod open;

pub(crate) const BANKABLE: i32 = 0x7000_B980;
pub(crate) const MISSION: i32 = 0x7000_B981;
pub(crate) const CARRIED_ONLY: i32 = 0x7000_B982;

const BASE: i32 = 0x7000_B800;
const ACTOR: SystemActor<'static> = SystemActor::Server {
    source: "bv07_test",
};

/// One test's account, characters and organizations.
pub(crate) struct Fx {
    pub(crate) pool: PgPool,
    pub(crate) block: i32,
    pub(crate) account_id: i32,
    pub(crate) players: Vec<i32>,
    names: Vec<(OrgType, String)>,
}

impl Fx {
    /// `n` characters on one account in `block`, after removing whatever a
    /// crashed run left there.
    pub(crate) async fn new(pool: &PgPool, block: i32, n: i32) -> Fx {
        assert!((0..18).contains(&block) && (1..16).contains(&n));
        let account_id = BASE + 16 * block;
        let fx = Fx {
            pool: pool.clone(),
            block,
            account_id,
            players: (1..=n).map(|i| account_id + i).collect(),
            names: vec![
                (OrgType::Team, format!("Bv07 {block} Team")),
                (OrgType::Command, format!("Bv07 {block} Command")),
                (OrgType::Team, format!("Bv07 {block} Other")),
            ],
        };
        fx.teardown().await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("bv07-test-{account_id}"))
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
                           0.0, 0.0, 0.0, 0, 0)",
            )
            .bind(account_id)
            .bind(player_id)
            .bind(format!("bv07-{player_id}"))
            .execute(pool)
            .await
            .expect("insert player");
        }
        for (id, sets) in [
            (BANKABLE, "{1,17}"),
            (MISSION, "{1,2,17}"),
            (CARRIED_ONLY, "{1}"),
        ] {
            sqlx::query(
                "INSERT INTO resources.items (\
                    item_id, description, name, quality_id, tech_comp, tier, \
                    max_stack_size, container_sets \
                 ) VALUES ($1, '', 'bv07-vault', 'ITEM_QUALITY_Normal', 0, 1, 20, $2::integer[]) \
                 ON CONFLICT (item_id) DO UPDATE SET container_sets = EXCLUDED.container_sets, \
                     max_stack_size = EXCLUDED.max_stack_size",
            )
            .bind(id)
            .bind(sets)
            .execute(pool)
            .await
            .expect("insert synthetic item type");
        }
        fx
    }

    pub(crate) fn player(&self, i: usize) -> i32 {
        self.players[i]
    }

    /// This block's entity id for character `i`.
    pub(crate) fn entity(&self, i: usize) -> u32 {
        0x7000_B9E0 + self.block as u32 * 2 + i as u32
    }

    /// Vault item id `k` (0..4) of this block.
    pub(crate) fn item(&self, k: i32) -> i32 {
        0x7000_B900 + 4 * self.block + k
    }

    fn name_keys(&self) -> Vec<String> {
        self.names
            .iter()
            .map(|(_, n)| org_text::name_key(&org_text::validate(TextField::Name, n).unwrap()))
            .collect()
    }

    /// Create the block's organization `which` (0 Team, 1 Command, 2 a
    /// second Team) led by character `leader`, with `members` at `Member`
    /// rank (which holds `DepositBank` but not `WithdrawBank`, D-BV12).
    pub(crate) async fn org(&self, which: usize, leader: usize, members: &[usize]) -> i32 {
        let (org_type, name) = &self.names[which];
        let mut tx = self.pool.begin().await.unwrap();
        let org = create_org(&mut tx, *org_type, name, self.player(leader))
            .await
            .expect("create_org")
            .org_id;
        let actor = OrgAccess::system(&mut tx, org, ACTOR)
            .await
            .unwrap()
            .unwrap();
        for &m in members {
            add_member(&mut tx, &actor, org, self.player(m), OrgRank::MEMBER)
                .await
                .expect("add_member");
        }
        tx.commit().await.unwrap();
        org
    }

    /// Put `stack` of `type_id` in `org_id`'s vault at `slot`.
    pub(crate) async fn put(&self, org_id: i32, item_id: i32, slot: i32, type_id: i32, stack: i32) {
        sqlx::query(
            "INSERT INTO sgw_organization_vault_items \
             (item_id, org_id, org_type, container_id, slot_id, type_id, stack_size, charges, \
              durability, flags, bound, ammo, cur_ammo_type, ammo_type, ammo_types, \
              deposited_by_player_id) \
             SELECT $1, o.org_id, o.org_type, CASE o.org_type WHEN 1 THEN 19 ELSE 20 END, $3, \
                    $4, $5, 0, -1, 0, false, 0, 0, 'AMMO_NONE', '{}', 0 \
             FROM sgw_organizations o WHERE o.org_id = $2",
        )
        .bind(item_id)
        .bind(org_id)
        .bind(slot)
        .bind(type_id)
        .bind(stack)
        .execute(&self.pool)
        .await
        .expect("insert vault row");
    }

    /// Put a carried row for character `who`.
    pub(crate) async fn carry(
        &self,
        who: usize,
        item_id: i32,
        container: i32,
        slot: i32,
        type_id: i32,
        stack: i32,
        bound: bool,
    ) {
        sqlx::query(
            "INSERT INTO sgw_inventory (item_id, character_id, container_id, slot_id, type_id, \
             stack_size, bound) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(item_id)
        .bind(self.player(who))
        .bind(container)
        .bind(slot)
        .bind(type_id)
        .bind(stack)
        .bind(bound)
        .execute(&self.pool)
        .await
        .expect("insert carried row");
    }

    /// `(item_id, slot, stack)` of every row in `org_id`'s vault.
    pub(crate) async fn vault(&self, org_id: i32) -> Vec<(i32, i32, i32)> {
        sqlx::query_as(
            "SELECT item_id, slot_id, stack_size FROM sgw_organization_vault_items \
             WHERE org_id = $1 ORDER BY slot_id",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    /// `(item_id, container, slot, stack)` of every row character `who` holds.
    pub(crate) async fn bag(&self, who: usize) -> Vec<(i32, i32, i32, i32)> {
        sqlx::query_as(
            "SELECT item_id, container_id, slot_id, stack_size FROM sgw_inventory \
             WHERE character_id = $1 ORDER BY container_id, slot_id",
        )
        .bind(self.player(who))
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    /// `(direction, kind, item_id, quantity, account_id, player_id)` of every
    /// log row of `org_id`, oldest first.
    pub(crate) async fn log(&self, org_id: i32) -> Vec<(String, String, i32, i32, i32, i32)> {
        sqlx::query_as(
            "SELECT direction, kind, item_id, quantity, account_id, player_id \
             FROM sgw_organization_vault_log WHERE org_id = $1 ORDER BY log_id",
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    /// Item ids that appear in more than one of `sgw_inventory`, the vault
    /// and mail escrow: always empty (the tables share one id sequence, and
    /// a move deletes what it copies). Over the whole tables, since a
    /// split's new id comes from the sequence, not the sentinel range.
    pub(crate) async fn duplicated_ids(&self) -> Vec<i32> {
        sqlx::query_scalar(
            "SELECT item_id FROM (\
                 SELECT item_id FROM sgw_inventory \
                 UNION ALL SELECT item_id FROM sgw_organization_vault_items \
                 UNION ALL SELECT item_id FROM sgw_gate_mail_item\
             ) t GROUP BY item_id HAVING count(*) > 1",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    /// Remove everything the block owns, vault rows before organizations.
    pub(crate) async fn teardown(&self) {
        let keys = self.name_keys();
        let orgs: Vec<i32> =
            sqlx::query_scalar("SELECT org_id FROM sgw_organizations WHERE name_key = ANY($1)")
                .bind(&keys)
                .fetch_all(&self.pool)
                .await
                .unwrap();
        for sql in [
            "DELETE FROM sgw_organization_vault_items WHERE org_id = ANY($1)",
            "DELETE FROM sgw_organization_vault_log WHERE org_id = ANY($1)",
            "DELETE FROM sgw_organization_events WHERE org_id = ANY($1)",
            "DELETE FROM sgw_organizations WHERE org_id = ANY($1)",
        ] {
            sqlx::query(sql)
                .bind(&orgs)
                .execute(&self.pool)
                .await
                .unwrap();
        }
        sqlx::query("DELETE FROM sgw_organization_vault_items WHERE item_id BETWEEN $1 AND $1 + 3")
            .bind(self.item(0))
            .execute(&self.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM sgw_inventory WHERE character_id = ANY($1) OR item_id BETWEEN $2 AND $2 + 3")
            .bind(&self.players)
            .bind(self.item(0))
            .execute(&self.pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(self.account_id)
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

/// A client in world as `entity_id`, so every send reaches it.
pub(crate) struct Client {
    pub(crate) transport: Arc<TestTransport>,
    pub(crate) dyn_transport: Arc<dyn Transport>,
    pub(crate) addr: SocketAddr,
    pub(crate) e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub(crate) conn: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

impl Client {
    pub(crate) fn in_world(entity_id: u32, port: u16) -> Client {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(entity_id);
        Client {
            transport,
            dyn_transport,
            addr,
            e2a: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
            conn: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        }
    }

    /// Every packet to the client, decrypted (the test key is all zeros).
    pub(crate) fn plaintexts(&self) -> Vec<Vec<u8>> {
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        self.transport
            .filter_to(self.addr)
            .iter()
            .filter_map(|p| enc.decrypt(p).ok())
            .collect()
    }

    /// Whether any packet carries `bytes`.
    pub(crate) fn saw_bytes(&self, bytes: &[u8]) -> bool {
        self.plaintexts()
            .iter()
            .any(|p| p.windows(bytes.len()).any(|w| w == bytes))
    }

    /// Whether any packet carries `needle` as UTF-16LE (a chat line).
    pub(crate) fn saw_text(&self, needle: &str) -> bool {
        let want: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
        self.saw_bytes(&want)
    }
}

/// A pool that never connects, for the infrastructure refusals.
pub(crate) fn unreachable_pool() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
        .expect("connect_lazy must succeed for any well-formed URL")
}
