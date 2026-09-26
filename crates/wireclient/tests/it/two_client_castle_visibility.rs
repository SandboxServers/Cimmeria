//! Two real wire clients, one shared Castle world, end to end (NA37).
//!
//! The owner's report: players on the same shared map (Castle, world 8)
//! "can't reliably see each other -- maybe the first sees the second but
//! not the reverse." A prior investigation (NA34,
//! `two_player_visibility::both_arrival_directions_deliver_the_observee_identity`
//! in `crates/services`) drove both `EnteredAoI` directions at the
//! base-dispatch level (in-process, no sockets) and could not reproduce a
//! one-way failure there. This test goes one level further: two real
//! `cimmeria-wireclient` sessions, authenticated over real SOAP HTTP,
//! driven through the real Mercury UDP handshake and world-entry sequence
//! against a real spawned `Orchestrator` (auth + base + cell), so a bug
//! anywhere in the wire encode/decode path -- not just the in-process
//! dispatch -- would surface here.
//!
//! This is the **lossless-localhost** variant. See
//! `two_client_castle_visibility_chaos.rs` for the round-2 companion that
//! runs the same shape of scenario under injected packet loss, reorder,
//! jitter, and high latency -- the network conditions a real player on
//! the colo actually experiences, which this file's loopback run cannot
//! exercise.
//!
//! Shared setup/driver helpers live in `tests/it/support/mod.rs` (see
//! that file's doc comment for why it isn't a macro-based
//! `require_db_or_skip!`).
//!
//! Run locally (the trailing `::` keeps the chaos module out):
//! ```text
//! /c/Users/Steve/AppData/Local/Temp/cimmeria-castle/reload-db.sh
//! DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/<db> \
//!   cargo test -p cimmeria-wireclient --test it two_client_castle_visibility:: -- --test-threads=1
//! ```
//! Not currently wired into `.github/workflows/test.yml`'s `ci-live-db`
//! job (which today runs `-p cimmeria-services --lib` only) -- see
//! `docs/architecture/wireclient.md` for the follow-up to add a
//! `wireclient-e2e` nextest profile. `--test-threads=1` matters here
//! because every live-DB test in this binary (these two and the three
//! in `two_client_castle_visibility_chaos.rs`) spawns its own
//! `Orchestrator` against the *same* shared database; sentinel id
//! ranges are kept disjoint between tests so they could in principle run
//! concurrently, but serializing avoids surprising interactions on the
//! account/sgw_player tables. nextest gives each test its own process
//! but still runs them in parallel; `--profile ci-live-db` serialises.

use std::time::Duration;

use crate::support::{
    self, assert_never, credentials_for, insert_castle_character, insert_sentinel_account,
    live_db_pool_or_skip, start_server, wait_for, CASTLE_BASE_POS, CLASS_SGWPLAYER,
};

use cimmeria_wireclient::session::GameSession;

/// Core NA37 scenario: two real clients, one GM (seeded "test" account)
/// and one non-GM (sentinel account this test inserts), both entering the
/// shared Castle world ~50m apart (well inside the 100m AoI radius --
/// `CellEntity::aoi_radius` default, `aoi_churn_smoke.rs`). Runs the
/// introduction in **both** arrival orders against the same live server,
/// since NA34's investigation flagged exactly this as the untested case
/// most likely to regress. For each order this asserts:
///
/// 1. Each client receives a `CREATE_ENTITY` (or `CREATE_BASE_PLAYER`, for
///    whichever avatar is already-mid-load) for the *other* player's
///    entity. The class byte on that introduction is asserted to be
///    `SGWPlayer` (0x02) for **both** the GM and non-GM observee: this is
///    documented, intentional behavior, not a bug this test flags --
///    `connect_entity` stamps `class_id = 0x02` for every player's cell
///    identity regardless of GM status ("GMs are introduced as plain
///    players" in `player-ghost-aoi-cascade.md`'s Known Gaps). Only the
///    *owning* client's own `CREATE_BASE_PLAYER` at world entry carries the
///    real GM class byte (0x03) -- this test's `enter_castle` doesn't
///    assert on that today.
/// 2. Each client also receives a `BEING_APPEARANCE` (method 26) cascade
///    entry for the other's entity -- proof the full player-ghost cascade
///    landed, not just the bare phase-1 create.
/// 3. A movement send from one relays to the other as an `UPDATE_AVATAR`
///    (0x10-0x2F family) referencing the mover's entity id.
/// 4. A `DISCONNECT` from one is followed by the other observing a
///    `leaveAoI` (0x0C) / `entityInvisible` (0x0B) for that entity.
#[tokio::test]
async fn two_clients_in_castle_see_each_other_both_arrival_orders() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "cimmeria_services=info".to_string()),
        )
        .with_test_writer()
        .try_init();
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;

    // Sentinel non-GM account + two sentinel characters per order, so the
    // two orders don't reuse an entity id that the first order's
    // disconnect may not have fully torn down yet.
    const SENTINEL_ACCOUNT: i32 = 900_101;
    insert_sentinel_account(&pool, SENTINEL_ACCOUNT, "na37_nongm").await;

    let gm_creds = credentials_for("test"); // seeded GM account_id 2
    let nongm_creds = credentials_for("na37_nongm");

    let pos_a = CASTLE_BASE_POS;
    let pos_b = [
        CASTLE_BASE_POS[0] + 50.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];

    for (order, gm_player_id, nongm_player_id) in [
        ("A-then-B", 900_102, 900_103),
        ("B-then-A", 900_104, 900_105),
    ] {
        insert_castle_character(&pool, 2, gm_player_id, "NA37GM", pos_a).await;
        insert_castle_character(&pool, SENTINEL_ACCOUNT, nongm_player_id, "NA37NonGM", pos_b).await;

        let (first, second) = if order == "A-then-B" {
            let gm = support::enter_castle(&server.auth_url, &gm_creds, gm_player_id, 1).await;
            let nongm =
                support::enter_castle(&server.auth_url, &nongm_creds, nongm_player_id, 2).await;
            (gm, nongm)
        } else {
            let nongm =
                support::enter_castle(&server.auth_url, &nongm_creds, nongm_player_id, 3).await;
            let gm = support::enter_castle(&server.auth_url, &gm_creds, gm_player_id, 4).await;
            (nongm, gm)
        };
        let first_id = first.player_entity_id.unwrap();
        let second_id = second.player_entity_id.unwrap();

        // Direction 1: `first` sees `second`.
        let create_of_second = wait_for(&first, Duration::from_secs(10), |m| {
            m.is_create() && m.entity_id == Some(second_id)
        })
        .await
        .unwrap_or_else(|| {
            panic!("[{order}] `first` never saw a create for `second` (id {second_id})")
        });
        assert_eq!(
            create_of_second.class_id,
            Some(CLASS_SGWPLAYER),
            "[{order}] `second`'s class byte on `first`'s wire feed"
        );
        wait_for(&first, Duration::from_secs(5), |m| {
            m.method_index == Some(26) && m.entity_id == Some(second_id)
        })
        .await
        .unwrap_or_else(|| {
            panic!("[{order}] `first` never received BEING_APPEARANCE (26) for `second`")
        });

        // Direction 2: `second` sees `first`.
        let create_of_first = wait_for(&second, Duration::from_secs(10), |m| {
            m.is_create() && m.entity_id == Some(first_id)
        })
        .await
        .unwrap_or_else(|| {
            panic!("[{order}] `second` never saw a create for `first` (id {first_id})")
        });
        assert_eq!(
            create_of_first.class_id,
            Some(CLASS_SGWPLAYER),
            "[{order}] `first`'s class byte on `second`'s wire feed"
        );
        wait_for(&second, Duration::from_secs(5), |m| {
            m.method_index == Some(26) && m.entity_id == Some(first_id)
        })
        .await
        .unwrap_or_else(|| {
            panic!("[{order}] `second` never received BEING_APPEARANCE (26) for `first`")
        });

        // Movement relay: `second` moves, `first` must see an
        // UPDATE_AVATAR-family message for `second`'s id.
        second
            .send_bundle(
                &GameSession::avatar_update_explicit(0, pos_b, [0.0; 3], [0, 0, 0], 1),
                false,
            )
            .await
            .expect("send AVATAR_UPDATE_EXPLICIT");
        wait_for(&first, Duration::from_secs(5), |m| {
            m.is_position_update() && m.entity_id == Some(second_id)
        })
        .await
        .unwrap_or_else(|| panic!("[{order}] `first` never saw a position update for `second`"));

        // Leave-on-logout: `second` disconnects, `first` must see
        // leaveAoI/entityInvisible for `second`'s id.
        second
            .send_bundle(&GameSession::disconnect(0), true)
            .await
            .expect("send DISCONNECT");
        wait_for(&first, Duration::from_secs(5), |m| {
            m.is_leave_or_hide() && m.entity_id == Some(second_id)
        })
        .await
        .unwrap_or_else(|| {
            panic!("[{order}] `first` never saw `second` leave AoI after disconnect")
        });

        first
            .send_bundle(&GameSession::disconnect(0), true)
            .await
            .expect("send DISCONNECT");

        for pid in [gm_player_id, nongm_player_id] {
            let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
                .bind(pid)
                .execute(&pool)
                .await;
        }
    }

    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(SENTINEL_ACCOUNT)
        .execute(&pool)
        .await;
    server.orchestrator.stop_all().await;
}

/// Negative control: two GM characters more than the 100m AoI radius
/// apart must **not** introduce each other, while a third character
/// placed close to one of them must. Without this control, a harness bug
/// that made [`wait_for`] always return `Some(..)` (a stuck-open
/// predicate, or scanning stale traffic) would make the positive
/// assertions above pass trivially.
#[tokio::test]
async fn characters_over_aoi_radius_apart_are_not_introduced() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;

    const ANCHOR_ACCOUNT: i32 = 900_211;
    const NEAR_ACCOUNT: i32 = 900_212;
    const FAR_ACCOUNT: i32 = 900_213;
    const FAR_PLAYER_ID: i32 = 900_201;
    const NEAR_PLAYER_ID: i32 = 900_202;
    const ANCHOR_PLAYER_ID: i32 = 900_203;

    let anchor_pos = CASTLE_BASE_POS;
    let near_pos = [
        CASTLE_BASE_POS[0] + 40.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];
    let far_pos = [
        CASTLE_BASE_POS[0] + 300.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];

    // Separate sentinel accounts per character (rather than three
    // characters on one account) -- keeps each `GameSession` on its own
    // account identity, matching how three distinct real players would
    // actually connect.
    insert_sentinel_account(&pool, ANCHOR_ACCOUNT, "na37_anchor").await;
    insert_sentinel_account(&pool, NEAR_ACCOUNT, "na37_near").await;
    insert_sentinel_account(&pool, FAR_ACCOUNT, "na37_far").await;
    insert_castle_character(
        &pool,
        ANCHOR_ACCOUNT,
        ANCHOR_PLAYER_ID,
        "NA37Anchor",
        anchor_pos,
    )
    .await;
    insert_castle_character(&pool, NEAR_ACCOUNT, NEAR_PLAYER_ID, "NA37Near", near_pos).await;
    insert_castle_character(&pool, FAR_ACCOUNT, FAR_PLAYER_ID, "NA37Far", far_pos).await;

    let anchor = support::enter_castle(
        &server.auth_url,
        &credentials_for("na37_anchor"),
        ANCHOR_PLAYER_ID,
        10,
    )
    .await;
    let near = support::enter_castle(
        &server.auth_url,
        &credentials_for("na37_near"),
        NEAR_PLAYER_ID,
        11,
    )
    .await;
    let far = support::enter_castle(
        &server.auth_url,
        &credentials_for("na37_far"),
        FAR_PLAYER_ID,
        12,
    )
    .await;

    let near_id = near.player_entity_id.unwrap();
    let far_id = far.player_entity_id.unwrap();

    // Positive: anchor sees near (40m apart).
    wait_for(&anchor, Duration::from_secs(10), |m| {
        m.is_create() && m.entity_id == Some(near_id)
    })
    .await
    .expect("anchor must see `near` (40m apart, inside the 100m AoI radius)");

    // Negative: anchor must not see far (300m apart) within a generous
    // window -- the periodic AoI tick runs every ~100ms, so a few
    // seconds is ample if it were (incorrectly) going to fire.
    assert_never(
        &anchor,
        Duration::from_secs(3),
        |m| m.is_create() && m.entity_id == Some(far_id),
        "anchor must not see `far` (300m apart, outside the 100m AoI radius)",
    )
    .await;

    for pid in [ANCHOR_PLAYER_ID, NEAR_PLAYER_ID, FAR_PLAYER_ID] {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(pid)
            .execute(&pool)
            .await;
    }
    for aid in [ANCHOR_ACCOUNT, NEAR_ACCOUNT, FAR_ACCOUNT] {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(aid)
            .execute(&pool)
            .await;
    }
    server.orchestrator.stop_all().await;
}
