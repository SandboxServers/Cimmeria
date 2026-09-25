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
//! Live-DB only: needs real seeded accounts (`test`, and a sentinel
//! non-GM account this test inserts) and real `sgw_player` character rows
//! this test inserts directly into Castle (world 8). Skips (does not
//! fail) when `DATABASE_URL` is unset, matching
//! `crate::test_support::require_db_or_skip!`'s contract in
//! `cimmeria-services` (this crate can't reach that private macro, so the
//! same skip-vs-fail shape is reimplemented locally below).
//!
//! Run locally:
//! ```text
//! /c/Users/Steve/AppData/Local/Temp/cimmeria-castle/reload-db.sh
//! DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/<db> \
//!   cargo test -p cimmeria-wireclient --test two_client_castle_visibility -- --test-threads=1
//! ```
//! Not currently wired into `.github/workflows/test.yml`'s `ci-live-db`
//! job (which today runs `-p cimmeria-services --lib` only) -- see
//! `docs/architecture/wireclient.md` for the follow-up to add a
//! `wireclient-e2e` nextest profile. `--test-threads=1` (or nextest's
//! default per-file isolation) matters here because both tests in this
//! file each spawn their own `Orchestrator` against the *same* shared
//! database; sentinel id ranges are kept disjoint between tests so they
//! could in principle run concurrently, but serializing avoids surprising
//! interactions on the account/sgw_player tables.

use std::net::TcpListener as StdTcpListener;
use std::time::Duration;

use sqlx::PgPool;

use cimmeria_common::ServerConfig;
use cimmeria_services::orchestrator::Orchestrator;
use cimmeria_wireclient::auth::Credentials;
use cimmeria_wireclient::bundle::{decode_bundle, S2CMessage};
use cimmeria_wireclient::session::GameSession;

/// SGWPlayer -- the class byte every player-ghost introduction carries,
/// GM or not (see the doc comment on
/// `two_clients_in_castle_see_each_other_both_arrival_orders`).
const CLASS_SGWPLAYER: u8 = 0x02;

const SHARD: &str = "Test";
const CASTLE_WORLD_LOCATION: &str = "Castle";
const CASTLE_WORLD_ID: i32 = 8;
/// A real in-bounds Castle spawn point (`Castle` space is `0..1000` on X/Y
/// per `gate_travel` tests); reused from the live-DB position test's
/// baseline in `base/world_entry/cell_dispatch/position.rs`.
const CASTLE_BASE_POS: [f32; 3] = [411.349, 70.111, 987.685];
/// Universal seed/test password hash -- every seeded account
/// (`db/sgw/Accounts/Seed/account.sql`) and the sentinel account this test
/// inserts share SHA-1("test").
const TEST_PASSWORD_SHA1: &str = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";

// ── Live-DB gate (local reimplementation -- `test_support` is `pub(crate)`
//    in `cimmeria-services` and not reachable from here) ──────────────────

async fn live_db_pool_or_skip() -> Option<PgPool> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            eprintln!(
                "two_client_castle_visibility: DATABASE_URL not set -- skipping (see reload-db.sh)"
            );
            return None;
        }
    };
    match sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
    {
        Ok(pool) => Some(pool),
        Err(e) => panic!(
            "DATABASE_URL is set but unreachable ({e}) -- a configured DB is a request to \
             run the live-DB tier, so this is a failure, not a skip (see #615)"
        ),
    }
}

macro_rules! require_db_or_skip {
    () => {
        match live_db_pool_or_skip().await {
            Some(p) => p,
            None => return,
        }
    };
}

// ── Sentinel account / character fixtures ───────────────────────────────

/// Insert a sentinel `sgw_player` row in Castle at `pos`, owned by
/// `account_id` (which must already exist), named `name`. Mirrors the
/// column shape of `db/sgw/Players/Seed/sgw_player.sql`.
async fn insert_castle_character(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
    name: &str,
    pos: [f32; 3],
) {
    sqlx::query(
        "INSERT INTO sgw_player (
            account_id, player_id, level, alignment, archetype, gender,
            player_name, extra_name, world_location, bodyset, title,
            pos_x, pos_y, pos_z, heading, naquadah, exp, first_login,
            world_id, known_stargates, components, abilities, access_level,
            skin_color_id, bandolier_slot, interaction_maps, training_points,
            discipline_ids, racial_paradigm_levels, applied_science_points,
            blueprint_ids, known_respawners
        ) VALUES (
            $1, $2, 1, 2, 1, 1,
            $3, '', $4, 'BS_HumanMale.BS_HumanMale', 0,
            $5, $6, $7, 0, 0, 0, 0,
            $8, ARRAY[]::integer[], ARRAY[]::character varying[], ARRAY[]::integer[], 0,
            0, 0, NULL, 0,
            ARRAY[]::integer[], ARRAY[]::integer[], 0,
            ARRAY[]::integer[], ARRAY[]::integer[]
        )",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(name)
    .bind(CASTLE_WORLD_LOCATION)
    .bind(pos[0])
    .bind(pos[1])
    .bind(pos[2])
    .bind(CASTLE_WORLD_ID)
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("insert sentinel sgw_player {player_id}: {e}"));
}

/// Insert a sentinel non-GM (`accesslevel = 0`) account, so the two-client
/// test covers both a GM (existing seed account "test") and a non-GM
/// session, per the owner's report that every *current* real account is a
/// GM -- this is the harness's negative control for the GM-only blind
/// spot.
async fn insert_sentinel_account(pool: &PgPool, account_id: i32, account_name: &str) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled) \
         VALUES ($1, $2, $3, 1, 0, true)",
    )
    .bind(account_id)
    .bind(account_name)
    .bind(TEST_PASSWORD_SHA1)
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("insert sentinel account {account_id}: {e}"));
}

fn credentials_for(username: &str) -> Credentials {
    Credentials {
        username: username.to_string(),
        password_sha1_hex: TEST_PASSWORD_SHA1.to_uppercase(),
        protocol_digest: "58AFA196AD3AC4F65CADD99BFF23B799".to_string(),
        sku: "SGW_BETA".to_string(),
    }
}

// ── Orchestrator bring-up ────────────────────────────────────────────────

fn ephemeral_port() -> u16 {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind ephemeral TCP listener");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    port
}

struct RunningServer {
    orchestrator: Orchestrator,
    auth_url: String,
}

/// `CellService::start()` loads `entities/spaces.xml` /
/// `entities/cell_spaces.xml` (the "Castle" space definition this test
/// needs) from the **process CWD**-relative path `"entities"`
/// (`crates/services/src/cell/service/mod.rs`'s `entities_dir` default) --
/// there is no `ServerConfig` field or setter to override it from outside
/// `cimmeria-services`. `cargo test` runs an integration test binary with
/// its CWD set to the *package* directory (`crates/wireclient/`), not the
/// workspace root, so the relative path resolves to a directory that
/// doesn't exist and `CreateEntity` fails with `"Unknown world: Castle"`
/// (silently falling back to a hardcoded space id that happens to be
/// shared by every unknown world, so two players in different *nominal*
/// worlds could wrongly appear to share a space -- and, worse here, an
/// entity that fails `CreateEntity` is never actually registered in any
/// `SpaceManager` space, so it can never appear in anyone's AoI). `chdir`
/// process-wide to the repo root once before starting the server, which is
/// safe because this file requires `--test-threads=1` (documented in the
/// header) for its live-DB isolation anyway.
fn chdir_to_repo_root_for_entities_xml() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // repo root
        .expect("crates/wireclient/../.. must resolve to the repo root");
    std::env::set_current_dir(repo_root)
        .unwrap_or_else(|e| panic!("chdir to repo root {repo_root:?} failed: {e}"));
}

/// Start a full `Orchestrator` (auth + base + cell + minigame) against
/// `db_url` on ephemeral ports, `developer_mode = false` so the real
/// DB-backed credential path (and real per-account `accesslevel`) is
/// exercised -- the developer-mode short-circuit would collapse every
/// account onto the same fabricated identity and defeat the GM/non-GM
/// distinction this test needs.
async fn start_server(db_url: &str) -> RunningServer {
    chdir_to_repo_root_for_entities_xml();
    const MAX_ATTEMPTS: usize = 5;
    let mut last_err = None;
    for _ in 0..MAX_ATTEMPTS {
        let config = ServerConfig {
            auth_host: "127.0.0.1".to_string(),
            logon_port: ephemeral_port(),
            base_host: "127.0.0.1".to_string(),
            base_external_host: "127.0.0.1".to_string(),
            base_port: ephemeral_port(),
            cell_host: "127.0.0.1".to_string(),
            cell_port: ephemeral_port(),
            admin_port: ephemeral_port(),
            minigame_port: ephemeral_port(),
            db_connection_string: db_url.to_string(),
            developer_mode: false,
            ..ServerConfig::default()
        };
        let auth_url = format!("http://127.0.0.1:{}", config.logon_port);
        let orchestrator = Orchestrator::new(config);
        match orchestrator.start_all().await {
            Ok(()) => {
                return RunningServer {
                    orchestrator,
                    auth_url,
                }
            }
            Err(e) => last_err = Some(e),
        }
    }
    panic!("failed to start Orchestrator after {MAX_ATTEMPTS} attempts (last error: {last_err:?})");
}

// ── World-entry driver ───────────────────────────────────────────────────

/// Drive a `GameSession` through auth, character select, and world entry
/// exactly as the real client sends it: `AUTHENTICATE` + `ENABLE_ENTITIES`
/// (char list) -> `playCharacter` -> `ENABLE_ENTITIES` (create player) ->
/// `mapLoaded` -> `onClientReady`. Returns the session with
/// `player_entity_id` populated from the server's `CREATE_BASE_PLAYER`
/// reply.
async fn enter_castle(
    auth_url: &str,
    creds: &Credentials,
    player_id: i32,
    request_id: u32,
) -> GameSession {
    let mut session = GameSession::connect(auth_url, creds, SHARD, request_id)
        .await
        .expect("GameSession::connect (auth + Mercury handshake)");

    let mut post_handshake = GameSession::authenticate();
    post_handshake.extend_from_slice(&GameSession::enable_entities());
    session
        .send_bundle(&post_handshake, true)
        .await
        .expect("send AUTHENTICATE + ENABLE_ENTITIES (char list)");
    let char_list = session.recv_bundles(1, Duration::from_secs(5)).await;
    assert_eq!(
        char_list.len(),
        1,
        "expected the character-list reply bundle"
    );

    session
        .send_bundle(&GameSession::play_character(player_id), true)
        .await
        .expect("send playCharacter");
    let reset = session.recv_bundles(1, Duration::from_secs(5)).await;
    assert_eq!(reset.len(), 1, "expected RESET_ENTITIES");

    session
        .send_bundle(&GameSession::enable_entities(), true)
        .await
        .expect("send ENABLE_ENTITIES (create player)");
    let create_player = session.recv_bundles(1, Duration::from_secs(5)).await;
    assert_eq!(create_player.len(), 1, "expected CREATE_BASE_PLAYER bundle");
    let own_id = decode_bundle(&create_player[0])
        .into_iter()
        .find(|m| m.msg_id == 0x05)
        .and_then(|m| m.entity_id)
        .unwrap_or_else(|| {
            panic!("CREATE_BASE_PLAYER bundle carried no entity_id: {create_player:?}")
        });
    session.player_entity_id = Some(own_id);

    session
        .send_bundle(&GameSession::map_loaded(own_id), true)
        .await
        .expect("send mapLoaded");
    let enter_world = session.recv_bundles(2, Duration::from_secs(5)).await;
    assert_eq!(
        enter_world.len(),
        2,
        "expected VIEWPORT+CELL+FORCED_POSITION plus the entity-data bundle"
    );

    session
        .send_bundle(&GameSession::on_client_ready(), true)
        .await
        .expect("send onClientReady");

    session
}

/// Poll `session`'s inbox (decoding every bundle that arrives) until
/// `pred` matches a message or `deadline` elapses. Every decoded message
/// that doesn't match is discarded (not re-queued) -- callers that need to
/// check more than one predicate against the same traffic window should
/// call this with a broader predicate and inspect the returned message's
/// fields instead of calling it twice.
async fn wait_for(
    session: &GameSession,
    deadline: Duration,
    mut pred: impl FnMut(&S2CMessage) -> bool,
) -> Option<S2CMessage> {
    let start = tokio::time::Instant::now();
    loop {
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            return None;
        }
        let slice = Duration::from_millis(300).min(deadline - elapsed);
        let bundles = session.recv_bundles(1, slice).await;
        for b in &bundles {
            for msg in decode_bundle(b) {
                if std::env::var("NA37_DEBUG_DECODE").is_ok() {
                    eprintln!(
                        "[decode] msg_id={:#04x} entity_id={:?} class_id={:?} method_index={:?} payload_len={}",
                        msg.msg_id,
                        msg.entity_id,
                        msg.class_id,
                        msg.method_index,
                        msg.payload.len()
                    );
                }
                if pred(&msg) {
                    return Some(msg);
                }
            }
        }
    }
}

/// Like [`wait_for`] but asserts nothing matching `pred` arrives before
/// `deadline` -- the negative-control half of a visibility assertion.
async fn assert_never(
    session: &GameSession,
    deadline: Duration,
    pred: impl FnMut(&S2CMessage) -> bool,
    msg: &str,
) {
    if let Some(m) = wait_for(session, deadline, pred).await {
        panic!("{msg}: unexpectedly observed {m:?}");
    }
}

// ── Scenarios ─────────────────────────────────────────────────────────────

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
    let pool = require_db_or_skip!();
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
            let gm = enter_castle(&server.auth_url, &gm_creds, gm_player_id, 1).await;
            let nongm = enter_castle(&server.auth_url, &nongm_creds, nongm_player_id, 2).await;
            (gm, nongm)
        } else {
            let nongm = enter_castle(&server.auth_url, &nongm_creds, nongm_player_id, 3).await;
            let gm = enter_castle(&server.auth_url, &gm_creds, gm_player_id, 4).await;
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
    let pool = require_db_or_skip!();
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

    let anchor = enter_castle(
        &server.auth_url,
        &credentials_for("na37_anchor"),
        ANCHOR_PLAYER_ID,
        10,
    )
    .await;
    let near = enter_castle(
        &server.auth_url,
        &credentials_for("na37_near"),
        NEAR_PLAYER_ID,
        11,
    )
    .await;
    let far = enter_castle(
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
