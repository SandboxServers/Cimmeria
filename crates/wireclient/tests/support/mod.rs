//! Shared helpers for the NA37 two-client Castle visibility integration
//! tests (`two_client_castle_visibility.rs` and
//! `two_client_castle_visibility_chaos.rs`). Not itself a test target --
//! `tests/support/` is a subdirectory, so cargo doesn't compile it as a
//! separate binary; each test file does `mod support;` to pull these in.
//!
//! Live-DB only: needs real seeded accounts (`test`, and sentinel non-GM
//! accounts each test inserts) and real `sgw_player` character rows each
//! test inserts directly into Castle (world 8). Skips (does not fail)
//! when `DATABASE_URL` is unset, matching
//! `crate::test_support::require_db_or_skip!`'s contract in
//! `cimmeria-services` (this crate can't reach that private macro, so the
//! same skip-vs-fail shape is reimplemented here).
//!
//! `mod support;` compiles this whole module into *each* including test
//! binary, but the lossless and chaos test files each use only a subset
//! of it (e.g. only the chaos file needs `bind_base_socket` /
//! `start_server_with_base_transport` / `wait_for_recording`) --
//! `#![allow(dead_code)]` avoids per-binary unused-item warnings for
//! items a *different* sibling binary uses. `cargo check -p
//! cimmeria-wireclient --all-targets` (which builds every test binary in
//! one pass) is what would otherwise flag these.

#![allow(dead_code)]

use std::net::TcpListener as StdTcpListener;
use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;

use cimmeria_common::ServerConfig;
use cimmeria_mercury::transport::BidirectionalTransport;
use cimmeria_services::orchestrator::Orchestrator;
use cimmeria_wireclient::auth::Credentials;
use cimmeria_wireclient::bundle::{decode_bundle, S2CMessage};
use cimmeria_wireclient::session::GameSession;

/// SGWPlayer -- the class byte every player-ghost introduction carries,
/// GM or not (see the doc comment on
/// `two_clients_in_castle_see_each_other_both_arrival_orders`).
pub const CLASS_SGWPLAYER: u8 = 0x02;

pub const SHARD: &str = "Test";
pub const CASTLE_WORLD_LOCATION: &str = "Castle";
pub const CASTLE_WORLD_ID: i32 = 8;
/// A real in-bounds Castle spawn point (`Castle` space is `0..1000` on X/Y
/// per `gate_travel` tests); reused from the live-DB position test's
/// baseline in `base/world_entry/cell_dispatch/position.rs`.
pub const CASTLE_BASE_POS: [f32; 3] = [411.349, 70.111, 987.685];
/// Universal seed/test password hash -- every seeded account
/// (`db/sgw/Accounts/Seed/account.sql`) and the sentinel accounts these
/// tests insert share SHA-1("test").
pub const TEST_PASSWORD_SHA1: &str = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3";

// ── Live-DB gate ─────────────────────────────────────────────────────────

pub async fn live_db_pool_or_skip() -> Option<PgPool> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => {
            eprintln!("two_client_castle_visibility: DATABASE_URL not set -- skipping (see reload-db.sh)");
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

// No macro here: `require_db_or_skip!`'s defining feature (an early
// `return` out of the *caller's* function) doesn't survive being called
// through a plain function, and `#[macro_export]` + `$crate` gets murky
// across the "shared module included into several independent test
// binaries" shape this file lives in. Each test just inlines:
// `let pool = match support::live_db_pool_or_skip().await { Some(p) => p, None => return };`

// ── Sentinel account / character fixtures ───────────────────────────────

/// Insert a sentinel `sgw_player` row in Castle at `pos`, owned by
/// `account_id` (which must already exist), named `name`. Mirrors the
/// column shape of `db/sgw/Players/Seed/sgw_player.sql`.
pub async fn insert_castle_character(
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

/// Insert a sentinel non-GM (`accesslevel = 0`) account, so a two-client
/// test can cover both a GM (existing seed account "test") and a non-GM
/// session, per the owner's report that every *current* real account is a
/// GM -- this is the harness's negative control for the GM-only blind
/// spot.
pub async fn insert_sentinel_account(pool: &PgPool, account_id: i32, account_name: &str) {
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

pub fn credentials_for(username: &str) -> Credentials {
    Credentials {
        username: username.to_string(),
        password_sha1_hex: TEST_PASSWORD_SHA1.to_uppercase(),
        protocol_digest: "58AFA196AD3AC4F65CADD99BFF23B799".to_string(),
        sku: "SGW_BETA".to_string(),
    }
}

// ── Orchestrator bring-up ────────────────────────────────────────────────

pub fn ephemeral_port() -> u16 {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind ephemeral TCP listener");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    port
}

pub struct RunningServer {
    pub orchestrator: Orchestrator,
    pub auth_url: String,
}

/// `CellService::start()` loads `entities/spaces.xml` /
/// `entities/cell_spaces.xml` (the "Castle" space definition these tests
/// need) from the **process CWD**-relative path `"entities"`
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
/// safe because both test files require `--test-threads=1` for their
/// live-DB isolation anyway.
pub fn chdir_to_repo_root_for_entities_xml() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // repo root
        .expect("crates/wireclient/../.. must resolve to the repo root");
    std::env::set_current_dir(repo_root)
        .unwrap_or_else(|e| panic!("chdir to repo root {repo_root:?} failed: {e}"));
}

fn base_config(db_url: &str, base_port: u16) -> ServerConfig {
    ServerConfig {
        auth_host: "127.0.0.1".to_string(),
        logon_port: ephemeral_port(),
        base_host: "127.0.0.1".to_string(),
        base_external_host: "127.0.0.1".to_string(),
        base_port,
        cell_host: "127.0.0.1".to_string(),
        cell_port: ephemeral_port(),
        admin_port: ephemeral_port(),
        minigame_port: ephemeral_port(),
        db_connection_string: db_url.to_string(),
        developer_mode: false,
        ..ServerConfig::default()
    }
}

/// Start a full `Orchestrator` (auth + base + cell + minigame) against
/// `db_url` on ephemeral ports, `developer_mode = false` so the real
/// DB-backed credential path (and real per-account `accesslevel`) is
/// exercised -- the developer-mode short-circuit would collapse every
/// account onto the same fabricated identity and defeat the GM/non-GM
/// distinction these tests need.
pub async fn start_server(db_url: &str) -> RunningServer {
    chdir_to_repo_root_for_entities_xml();
    const MAX_ATTEMPTS: usize = 5;
    let mut last_err = None;
    for _ in 0..MAX_ATTEMPTS {
        let config = base_config(db_url, ephemeral_port());
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

/// Like [`start_server`], but the BaseApp's UDP recv loop runs on
/// `transport` (a caller-supplied `Arc<dyn BidirectionalTransport>`,
/// typically a `LossyTransport` wrapping a real socket the caller bound
/// itself) instead of a plain socket `start()` binds internally.
/// `base_port` must be the port `transport`'s underlying socket is
/// actually bound to -- callers get this from `bind_base_socket` or their
/// own `UdpSocket::bind` + `local_addr()`. Single-attempt (no ephemeral-
/// port retry loop): the base port is already locked in by a live socket,
/// so only the *other* ports (auth/cell/admin/minigame) could in
/// principle race, which is rare enough on a dev box not to warrant the
/// retry machinery `start_server` needs for its fully-ephemeral case.
///
/// Requires the `chaos-testing` feature on the `cimmeria-services`
/// dev-dependency (already enabled in `Cargo.toml`) for
/// `BaseService::set_transport_override`. See
/// `docs/architecture/network-chaos-testing.md`.
pub async fn start_server_with_base_transport(
    db_url: &str,
    base_port: u16,
    transport: Arc<dyn BidirectionalTransport>,
) -> RunningServer {
    chdir_to_repo_root_for_entities_xml();
    let config = base_config(db_url, base_port);
    let auth_url = format!("http://127.0.0.1:{}", config.logon_port);
    let orchestrator = Orchestrator::new(config);
    {
        let state_arc = orchestrator.state();
        let mut state = state_arc.write().await;
        state.base.set_transport_override(transport);
    }
    orchestrator
        .start_all()
        .await
        .expect("start_all with chaos transport override");
    RunningServer {
        orchestrator,
        auth_url,
    }
}

/// Bind a real UDP socket on an ephemeral port and wrap it as a plain
/// (lossless) `BidirectionalTransport`, returning the transport and the
/// port it's bound to. Callers layer a `LossyTransport` (or other chaos
/// wrapper) around the returned transport before handing it to
/// [`start_server_with_base_transport`].
pub async fn bind_base_socket() -> (Arc<dyn BidirectionalTransport>, u16) {
    let socket = tokio::net::UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("bind chaos base socket");
    let port = socket.local_addr().expect("local_addr").port();
    let transport: Arc<dyn BidirectionalTransport> =
        Arc::new(cimmeria_mercury::transport::UdpTransport::new(Arc::new(
            socket,
        )));
    (transport, port)
}

// ── World-entry driver ───────────────────────────────────────────────────

/// Drive a `GameSession` through auth, character select, and world entry
/// exactly as the real client sends it: `AUTHENTICATE` + `ENABLE_ENTITIES`
/// (char list) -> `playCharacter` -> `ENABLE_ENTITIES` (create player) ->
/// `mapLoaded` -> `onClientReady`. Returns the session with
/// `player_entity_id` populated from the server's `CREATE_BASE_PLAYER`
/// reply. `recv_timeout` is the per-step wait bound -- callers running
/// under injected loss/latency should pass something much larger than the
/// lossless-network default (5s) to give retransmit + jitter room to
/// still land within the test's patience.
///
/// Waits for `count` **meaningful** bundles, skipping any bundle that
/// decodes to nothing but `tickSync` (msg_id `0x0D`) heartbeats. Under
/// injected latency/jitter, a periodic tickSync can complete its
/// (randomized) delay and land in the inbox ahead of a delayed reply that
/// was sent earlier, which breaks a naive "the next bundle is always the
/// expected reply" assumption -- `enter_castle_with_timeout`'s own
/// sequencing (send one client message, expect exactly one specific kind
/// of reply next) depends on this filter to stay correct once packets can
/// arrive out of their send order.
async fn recv_meaningful_bundles(
    session: &GameSession,
    count: usize,
    timeout: Duration,
) -> Vec<bytes::Bytes> {
    let start = tokio::time::Instant::now();
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            break;
        }
        let bundles = session.recv_bundles(1, timeout - elapsed).await;
        if bundles.is_empty() {
            break;
        }
        for b in bundles {
            let msgs = decode_bundle(&b);
            let is_trivial_tick_sync = !msgs.is_empty() && msgs.iter().all(|m| m.msg_id == 0x0D);
            if !is_trivial_tick_sync {
                out.push(b);
                if out.len() >= count {
                    break;
                }
            }
        }
    }
    out
}

pub async fn enter_castle_with_timeout(
    auth_url: &str,
    creds: &Credentials,
    player_id: i32,
    request_id: u32,
    recv_timeout: Duration,
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
    let char_list = recv_meaningful_bundles(&session, 1, recv_timeout).await;
    assert_eq!(
        char_list.len(),
        1,
        "expected the character-list reply bundle"
    );

    session
        .send_bundle(&GameSession::play_character(player_id), true)
        .await
        .expect("send playCharacter");
    let reset = recv_meaningful_bundles(&session, 1, recv_timeout).await;
    assert_eq!(reset.len(), 1, "expected RESET_ENTITIES");

    session
        .send_bundle(&GameSession::enable_entities(), true)
        .await
        .expect("send ENABLE_ENTITIES (create player)");
    let create_player = recv_meaningful_bundles(&session, 1, recv_timeout).await;
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
    let enter_world = recv_meaningful_bundles(&session, 2, recv_timeout).await;
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

/// [`enter_castle_with_timeout`] with the lossless-network default (5s).
pub async fn enter_castle(
    auth_url: &str,
    creds: &Credentials,
    player_id: i32,
    request_id: u32,
) -> GameSession {
    enter_castle_with_timeout(auth_url, creds, player_id, request_id, Duration::from_secs(5)).await
}

/// Poll `session`'s inbox (decoding every bundle that arrives) until
/// `pred` matches a message or `deadline` elapses. Every decoded message
/// that doesn't match is discarded (not re-queued) -- callers that need to
/// check more than one predicate against the same traffic window should
/// call this with a broader predicate and inspect the returned message's
/// fields instead of calling it twice.
pub async fn wait_for(
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

/// Like [`wait_for`] but records **every** decoded message seen (matching
/// or not) into the returned `Vec`, in arrival order, alongside the
/// matching message (if any) once `pred` fires or `deadline` elapses.
/// Used by the ordering-hazard scenario, which needs to inspect the whole
/// arrival sequence, not just the first hit.
pub async fn wait_for_recording(
    session: &GameSession,
    deadline: Duration,
    mut pred: impl FnMut(&S2CMessage) -> bool,
) -> (Option<S2CMessage>, Vec<S2CMessage>) {
    let start = tokio::time::Instant::now();
    let mut seen = Vec::new();
    loop {
        let elapsed = start.elapsed();
        if elapsed >= deadline {
            return (None, seen);
        }
        let slice = Duration::from_millis(300).min(deadline - elapsed);
        let bundles = session.recv_bundles(1, slice).await;
        for b in &bundles {
            for msg in decode_bundle(b) {
                if std::env::var("NA37_DEBUG_DECODE").is_ok() {
                    eprintln!(
                        "[decode] t={:?} msg_id={:#04x} entity_id={:?} class_id={:?} method_index={:?} payload_len={}",
                        start.elapsed(),
                        msg.msg_id,
                        msg.entity_id,
                        msg.class_id,
                        msg.method_index,
                        msg.payload.len()
                    );
                }
                let hit = pred(&msg);
                seen.push(msg.clone());
                if hit {
                    return (seen.last().cloned(), seen);
                }
            }
        }
    }
}

/// Like [`wait_for`] but asserts nothing matching `pred` arrives before
/// `deadline` -- the negative-control half of a visibility assertion.
pub async fn assert_never(
    session: &GameSession,
    deadline: Duration,
    pred: impl FnMut(&S2CMessage) -> bool,
    msg: &str,
) {
    if let Some(m) = wait_for(session, deadline, pred).await {
        panic!("{msg}: unexpectedly observed {m:?}");
    }
}
