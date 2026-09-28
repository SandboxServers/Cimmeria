//! Presence: the offline fanout from every session teardown, exactly once.

use cimmeria_entity::manager::EntityManager;
use cimmeria_wire::cell::client_methods::organization::build_on_member_joined_organization;
use tracing::Level;

use super::*;
use crate::base::helpers::destroy_client_entities;
use crate::base::organization::handlers::announce_offline;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Every `destroy_client_entities` reason (client disconnect, inactivity
/// timeout, send error, duplicate login, account log-off) tells the online
/// members of the character's organization that it went offline: [37] with
/// the name, id 0 and the rank, never [39]. The presence row carries the
/// teardown's `disconnect_reason`.
///
/// Guard for audit A-35: before ORG-06 the teardown told nobody.
#[tokio::test]
async fn live_db_offline_fanout_on_every_disconnect_path() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 1, 2, &["Org06 Presence"]).await;
    let team = fx.org(OrgType::Team, "Org06 Presence", 0, &[1]).await;
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let offline = build_on_member_joined_organization(&fx.name(0), 0, team, OrgRank::LEADER, false);

    for (n, reason) in [
        "client_disconnect",
        "inactivity_timeout",
        "send_error",
        "duplicate_login",
        "logoff",
    ]
    .into_iter()
    .enumerate()
    {
        fx.online(0);
        fx.online(1);
        let capture = LogCapture::install();
        destroy_client_entities(
            &fx.connected,
            &entity_manager,
            fx.addr(0),
            &None,
            &fx.entity_to_addr,
            &fx.transport,
            &fx.db_pool,
            reason,
        )
        .await;
        fx.wait_for_packets(1, n + 1).await;
        let bundles = fx.bundles_to(1);
        assert_eq!(bundles[n], vec![(37, offline.clone())], "reason {reason}");
        // The spawned task logs after its sends; give it a moment.
        let mut row = None;
        for _ in 0..200 {
            row = capture
                .all()
                .into_iter()
                .find(|c| c.target == "org" && c.has_field("event", "member_offline"));
            if row.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let row = row.expect("member_offline row");
        assert!(row.has_field("disconnect_reason", reason), "{row:?}");
        assert!(row.has_field("recipients", "1"), "{row:?}");
        assert!(row.has_field("member_id", "0"), "{row:?}");
    }
    fx.teardown().await;
}

/// A session no longer listed online (a full-exit `logOff` already
/// unlisted it and announced) is not announced again when its disconnect
/// reaps it.
#[tokio::test]
async fn live_db_unlisted_session_is_not_announced_twice() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 2, 2, &["Org06 Unlisted"]).await;
    fx.org(OrgType::Team, "Org06 Unlisted", 0, &[1]).await;
    fx.online(0);
    fx.online(1);
    fx.connected
        .lock()
        .unwrap()
        .get_mut(&fx.addr(0))
        .unwrap()
        .listed_online = false;
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    destroy_client_entities(
        &fx.connected,
        &entity_manager,
        fx.addr(0),
        &None,
        &fx.entity_to_addr,
        &fx.transport,
        &fx.db_pool,
        "client_disconnect",
    )
    .await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(fx.typed.filter_to(fx.addr(1)).is_empty());
    fx.teardown().await;
}

/// A member whose session has no address mapping cannot be sent to: WARN
/// `org.send_failed` with the send's `reason`, and the presence row counts
/// only the members actually reached.
#[tokio::test]
async fn live_db_presence_send_failure_warns_with_reason() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 3, 2, &["Org06 Send Fail"]).await;
    fx.org(OrgType::Team, "Org06 Send Fail", 0, &[1]).await;
    fx.online(1);
    fx.entity_to_addr.lock().unwrap().remove(&fx.entity(1));
    let capture = LogCapture::install();
    announce_offline(&fx.ctx(), &fx.player(0), "client_disconnect").await;
    let warn = capture
        .find_event(
            Level::WARN,
            "could not be sent to a member",
            "entity_to_addr_miss",
        )
        .expect("org.send_failed WARN");
    assert!(warn.has_field("event", "org.send_failed"), "{warn:?}");
    assert!(
        warn.has_field("target_player_id", &fx.player_id(1).to_string()),
        "{warn:?}"
    );
    let row = capture
        .find_message(Level::INFO, "presence announced")
        .expect("presence row");
    assert!(row.has_field("recipients", "0"), "{row:?}");
    assert!(row.has_field("online_members", "1"), "{row:?}");
    fx.teardown().await;
}

/// Audit A-35: a character that drops without `logOff` (here an inactivity
/// timeout) is announced offline to its contact-list watchers too: CM 89
/// `LoggedInStatus` with the offline value. Before ORG-06 only `logOff`
/// told them.
#[tokio::test]
async fn live_db_contact_list_watchers_hear_a_teardown() {
    use crate::base::contact_list::persistence::{add_members, ensure_system_lists};
    use cimmeria_wire::base::contact_list::wire::{
        build_on_contact_list_event, DATA_OFFLINE, EVENT_LOGGED_IN_STATUS,
    };

    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 12, 2, &[]).await;
    let (friends, _) = ensure_system_lists(&pool, fx.player_id(1)).await.unwrap();
    add_members(&pool, fx.player_id(1), friends, &[fx.name(0)])
        .await
        .unwrap();
    fx.online(0);
    fx.online(1);
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    destroy_client_entities(
        &fx.connected,
        &entity_manager,
        fx.addr(0),
        &None,
        &fx.entity_to_addr,
        &fx.transport,
        &fx.db_pool,
        "inactivity_timeout",
    )
    .await;
    fx.wait_for_packets(1, 1).await;
    assert_eq!(
        fx.calls_to(1),
        vec![(
            89,
            build_on_contact_list_event(&fx.name(0), EVENT_LOGGED_IN_STATUS, DATA_OFFLINE)
        )]
    );
    fx.teardown().await;
}
