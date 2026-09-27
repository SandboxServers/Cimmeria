//! ORG-05 creation tests, base side.
//!
//! - [`found`]: [`super::found_organization`] against the real schema
//!   (type 3): the CAT-M-03 guards, the D-ORG15 debit, and that a refusal a
//!   player can provoke never draws an organization id.
//! - [`handlers`]: the handlers end to end with a `TestTransport` session
//!   (types 3, 8 and 12): what the client and the cell receive, and the
//!   outcome rows.
//!
//! Live-DB sentinels: this module owns `0x7000_4E00..=0x7000_4EFF` for
//! account and player ids (blocks of 16). Organizations are cleaned up by the
//! exact sentinel characters that lead them and by exact name key.

mod found;
mod handlers;
mod seams;

use cimmeria_entity::organization::{org_text, TextField};
use sqlx::PgPool;

/// First sentinel of this module's block.
const BASE: i32 = 0x7000_4E00;

/// One test's account and characters.
pub(super) struct Fixture {
    pub(super) account_id: i32,
    pub(super) players: Vec<i32>,
    name_keys: Vec<String>,
}

impl Fixture {
    pub(super) fn player(&self, i: usize) -> i32 {
        self.players[i]
    }
}

/// Create one account and one character per entry of `naquadah` (its
/// starting naquadah) in block `block` (0..16), after removing whatever an
/// earlier crashed run left. `names` are the organization names the test
/// may create, for cleanup.
pub(super) async fn setup(pool: &PgPool, block: i32, naquadah: &[i32], names: &[&str]) -> Fixture {
    assert!((0..16).contains(&block) && (1..16).contains(&naquadah.len()));
    let account_id = BASE + block * 16;
    let fx = Fixture {
        account_id,
        players: (1..=naquadah.len() as i32)
            .map(|i| account_id + i)
            .collect(),
        name_keys: names
            .iter()
            .filter_map(|n| org_text::validate(TextField::Name, n).ok())
            .map(|n| org_text::name_key(&n))
            .collect(),
    };
    teardown(pool, &fx).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("org05-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    for (&player_id, &cash) in fx.players.iter().zip(naquadah) {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 7, 0, 3, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, $4)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("org05-{player_id}"))
        .bind(cash)
        .execute(pool)
        .await
        .expect("insert player");
    }
    fx
}

/// Remove the organizations the fixture's characters belong to or whose
/// names it listed, then its characters and account, by exact id.
pub(super) async fn teardown(pool: &PgPool, fx: &Fixture) {
    sqlx::query(
        "DELETE FROM sgw_organizations WHERE org_id IN \
         (SELECT org_id FROM sgw_organization_members WHERE player_id = ANY($1)) \
         OR name_key = ANY($2)",
    )
    .bind(&fx.players)
    .bind(&fx.name_keys)
    .execute(pool)
    .await
    .expect("cleanup organizations");
    sqlx::query("DELETE FROM sgw_organization_events WHERE from_account_id = $1")
        .bind(fx.account_id)
        .execute(pool)
        .await
        .expect("cleanup organization events");
    sqlx::query("DELETE FROM sgw_player WHERE player_id = ANY($1)")
        .bind(&fx.players)
        .execute(pool)
        .await
        .expect("cleanup players");
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(fx.account_id)
        .execute(pool)
        .await
        .expect("cleanup account");
}

/// The organization-id sequence's position, `(last_value, is_called)`: a
/// refused insert that drew an id moves it.
pub(super) async fn org_id_sequence(pool: &PgPool) -> (i64, bool) {
    sqlx::query_as("SELECT last_value, is_called FROM sgw_organizations_org_id_seq")
        .fetch_one(pool)
        .await
        .expect("read the org id sequence")
}

pub(super) async fn naquadah(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("read naquadah")
}

/// How many organizations `player_id` belongs to.
pub(super) async fn memberships(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM sgw_organization_members WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("count memberships")
}
