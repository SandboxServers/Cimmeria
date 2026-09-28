//! ORG-06: `logOff` announces a character in the world offline to its
//! organizations once, and the disconnect that reaps a full exit does not
//! announce it again.
//!
//! Live-DB sentinels: account `0x7000_4DF0`, players `0x7000_4DF1` and
//! `0x7000_4DF2` (the last block of ORG-06's range); the Team is cleaned by
//! its exact name key.

use std::time::Duration;

use super::super::*;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};
use cimmeria_base_session::base::helpers::destroy_client_entities;
use cimmeria_base_session::base::organization::api::{OrgAccess, SystemActor};
use cimmeria_base_session::base::organization::persistence::{add_member, create_org};
use cimmeria_entity::organization::{OrgRank, OrgType};
use sqlx::PgPool;

const ACCOUNT: i32 = 0x7000_4DF0;
const LEAVER: i32 = ACCOUNT + 1;
const STAYER: i32 = ACCOUNT + 2;
const TEAM_KEY: &str = "org06 logoff";

async fn cleanup(pool: &PgPool) {
    let run = |sql: &'static str, id: i32| {
        let pool = pool.clone();
        async move {
            sqlx::query(sql).bind(id).execute(&pool).await.expect(sql);
        }
    };
    sqlx::query("DELETE FROM sgw_organizations WHERE name_key = $1")
        .bind(TEAM_KEY)
        .execute(pool)
        .await
        .unwrap();
    run(
        "DELETE FROM sgw_organization_events WHERE from_account_id = $1",
        ACCOUNT,
    )
    .await;
    run("DELETE FROM sgw_player WHERE player_id = $1", LEAVER).await;
    run("DELETE FROM sgw_player WHERE player_id = $1", STAYER).await;
    run("DELETE FROM account WHERE account_id = $1", ACCOUNT).await;
}

#[tokio::test]
async fn live_db_full_exit_logoff_announces_offline_once() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) VALUES ($1, 'org06-logoff', '')",
    )
    .bind(ACCOUNT)
    .execute(&pool)
    .await
    .unwrap();
    for (pid, name) in [(LEAVER, "Org06Leaver"), (STAYER, "Org06Stayer")] {
        sqlx::query(
            "INSERT INTO sgw_player (account_id, player_id, level, alignment, archetype, gender, \
             player_name, extra_name, world_location, bodyset, pos_x, pos_y, pos_z, skin_color_id) \
             VALUES ($1, $2, 5, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
             0.0, 0.0, 0.0, 0)",
        )
        .bind(ACCOUNT)
        .bind(pid)
        .bind(name)
        .execute(&pool)
        .await
        .unwrap();
    }
    let mut tx = pool.begin().await.unwrap();
    let team = create_org(&mut tx, OrgType::Team, "Org06 Logoff", STAYER)
        .await
        .unwrap()
        .org_id;
    let sys = OrgAccess::system(
        &mut tx,
        team,
        SystemActor::Server {
            source: "org06_test",
        },
    )
    .await
    .unwrap()
    .unwrap();
    add_member(&mut tx, &sys, team, LEAVER, OrgRank::MEMBER)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let leaver_addr: SocketAddr = "127.0.0.1:54710".parse().unwrap();
    let stayer_addr: SocketAddr = "127.0.0.1:54711".parse().unwrap();
    let session = |pid: i32, eid: u32, name: &str| {
        let mut s = test_default_connected_client_state();
        s.account_id = ACCOUNT as u32;
        s.active_player_id = Some(pid);
        s.player_entity_id = Some(eid);
        s.player_name = Some(name.to_string());
        s.listed_online = true;
        s
    };
    let connected = Arc::new(Mutex::new(HashMap::from([
        (leaver_addr, session(LEAVER, 6101, "Org06Leaver")),
        (stayer_addr, session(STAYER, 6102, "Org06Stayer")),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (6101u32, leaver_addr),
        (6102u32, stayer_addr),
    ])));
    let typed = Arc::new(TestTransport::default());
    let transport: Arc<dyn Transport> = typed.clone();
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let db_pool = Some(Arc::new(pool.clone()));

    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &[1],
        &Some("Org06Leaver".to_string()),
        leaver_addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &None,
        &entity_to_addr,
        &db_pool,
    )
    .await
    .expect("logOff must not fail");
    for _ in 0..500 {
        if !typed.filter_to(stayer_addr).is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        typed.filter_to(stayer_addr).len(),
        1,
        "one offline announcement"
    );

    // The client's disconnect reaps the full exit: no second announcement.
    destroy_client_entities(
        &connected,
        &entity_manager,
        leaver_addr,
        &None,
        &entity_to_addr,
        &transport,
        &db_pool,
        "client_disconnect",
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(
        typed.filter_to(stayer_addr).len(),
        1,
        "announced exactly once"
    );
    cleanup(&pool).await;
}
