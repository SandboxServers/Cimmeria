//! Live-DB tests for organization persistence (TESTING.md type 3).
//!
//! Every test runs against a database loaded from `db/database.sql`, so the
//! schema, the constraints and the member-delete trigger under test are the
//! real ones. Sentinels: this module owns `0x7000_4800..=0x7000_4BFF` for
//! account and player ids (each test takes one of 64 blocks of 16).
//! Organization ids come from the sequence and are cleaned up by exact id; organization names
//! start with "Org02 " and are cleaned up by exact name key, so a crashed
//! run cannot make the next one collide on `UNIQUE (org_type, name_key)`.

/// Run a persistence mutation as a system actor for `$org`, read in the
/// same transaction: `as_sys!(set_rank, tx, org, player, rank)` is
/// `set_rank(&mut tx, &actor, org, player, rank).await`.
macro_rules! as_sys {
    ($f:path, $tx:ident, $org:expr $(, $arg:expr)* $(,)?) => {{
        let org_id = $org;
        let actor = sys(&mut $tx, org_id).await;
        $f(&mut $tx, &actor, org_id $(, $arg)*).await
    }};
}

mod audit;
mod authority;
mod constraints;
mod mutations;
mod telemetry;
mod trigger;
mod vault;

use std::time::Duration;

use cimmeria_entity::organization::{org_text, OrgType, TextField};
use sqlx::PgPool;

use super::super::api::{OrgAccess, SystemActor};
use super::{create_org, CreatedOrg};

/// The system actor the fixtures act as.
const TEST_ACTOR: SystemActor<'static> = SystemActor::Server {
    source: "org02_test",
};

/// A system [`OrgAccess`] for `org_id`, read in `tx`.
async fn sys(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, org_id: i32) -> OrgAccess {
    OrgAccess::system(tx, org_id, TEST_ACTOR)
        .await
        .expect("system access")
        .expect("the organization exists")
}

/// First sentinel of this module's block.
const BASE: i32 = 0x7000_4800;

/// One test's characters and organizations.
struct Fixture {
    account_id: i32,
    players: Vec<i32>,
    /// Name keys of every organization the test may create.
    name_keys: Vec<String>,
}

impl Fixture {
    fn player(&self, i: usize) -> i32 {
        self.players[i]
    }
}

/// Create one account and `n` characters in block `block` (0..64), after
/// removing whatever an earlier crashed run left there. `names` are the
/// organization names the test will use.
async fn setup(pool: &PgPool, block: i32, n: i32, names: &[&str]) -> Fixture {
    assert!((0..64).contains(&block) && (1..16).contains(&n));
    let account_id = BASE + block * 16;
    let fx = Fixture {
        account_id,
        players: (1..=n).map(|i| account_id + i).collect(),
        name_keys: names
            .iter()
            .map(|n| org_text::name_key(&org_text::validate(TextField::Name, n).unwrap()))
            .collect(),
    };
    teardown(pool, &fx).await;

    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("org02-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    for &player_id in &fx.players {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id\
             ) VALUES ($1, $2, 7, 0, 3, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("org02-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }
    fx
}

/// Remove the fixture's organizations (by exact name key), characters and
/// account (by exact id). Organizations go first, so the trigger never has
/// to disband one on the way.
async fn teardown(pool: &PgPool, fx: &Fixture) {
    for key in &fx.name_keys {
        sqlx::query("DELETE FROM sgw_organizations WHERE name_key = $1")
            .bind(key)
            .execute(pool)
            .await
            .expect("cleanup organizations");
    }
    // Audit rows are keyed by the exact sentinel account they name.
    sqlx::query("DELETE FROM sgw_organization_events WHERE from_account_id = $1")
        .bind(fx.account_id)
        .execute(pool)
        .await
        .expect("cleanup organization events");
    for &player_id in &fx.players {
        sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await
            .expect("cleanup player");
    }
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(fx.account_id)
        .execute(pool)
        .await
        .expect("cleanup account");
}

/// `create_org` in its own committed transaction.
async fn create(pool: &PgPool, org_type: OrgType, name: &str, leader: i32) -> CreatedOrg {
    let mut tx = pool.begin().await.unwrap();
    let org = create_org(&mut tx, org_type, name, leader)
        .await
        .expect("create_org");
    tx.commit().await.unwrap();
    org
}

/// `(player_id, rank)` of every member, by player id.
async fn member_ranks(pool: &PgPool, org_id: i32) -> Vec<(i32, i16)> {
    sqlx::query_as(
        "SELECT player_id, rank FROM sgw_organization_members WHERE org_id = $1 \
         ORDER BY player_id",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn org_exists(pool: &PgPool, org_id: i32) -> bool {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sgw_organizations WHERE org_id = $1)")
        .bind(org_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn rank_row_count(pool: &PgPool, org_id: i32) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM sgw_organization_ranks WHERE org_id = $1")
        .bind(org_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The constraint a failed statement violated.
fn violated(e: &sqlx::Error) -> Option<String> {
    match e {
        sqlx::Error::Database(db) => db.constraint().map(str::to_owned),
        _ => None,
    }
}

/// Poll until some other session in this database waits on a lock, so a
/// concurrency test cannot pass without its race.
async fn wait_until_blocked(pool: &PgPool) {
    for _ in 0..100 {
        if lock_waiters(pool).await > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the concurrent statement never waited on a lock");
}

/// How many other sessions in this database wait on a lock.
async fn lock_waiters(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_stat_activity \
         WHERE wait_event_type = 'Lock' AND datname = current_database() \
           AND pid <> pg_backend_pid()",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}
