//! Two real wire clients: a Command leader invites a player, who accepts
//! (ORG-07).
//!
//! Alpha leads a Command (seeded rows). Alpha sends base method 0xD0
//! `organizationInviteByType(2, name)`. Bravo gets `onOrganizationInvite`
//! [34] with type 2 and a base-issued request id (`BASE_INVITE_REQUEST_FLAG`,
//! bit 29), and accepts with CM 8, which the cell forwards to the base.
//! Bravo then receives the Command's state, opening with
//! `onOrganizationJoined` [35] (rank Initiate, new member), and Alpha gets
//! `onMemberJoinedOrganization` [37] naming Bravo as a new member. The
//! database holds Bravo at rank 1.
//!
//! Against a spawned `Orchestrator`, over real SOAP auth and Mercury UDP.
//! Not in CI's live-DB job (see `docs/architecture/wireclient.md`). Run:
//! ```text
//! bash tools/build-lane/reload-db.sh
//! DATABASE_URL=<printed url> bash tools/build-lane/lane.sh \
//!   cargo test -p cimmeria-wireclient --test it two_client_command_invite -- --test-threads=1
//! ```

use std::time::Duration;

use cimmeria_wireclient::session::GameSession;

use crate::support::{
    self, credentials_for, insert_castle_character, insert_sentinel_account, live_db_pool_or_skip,
    start_server, wait_for, wait_for_recording, CASTLE_BASE_POS,
};

// ORG-07's wireclient sentinels (`0x7000_5300..=0x7000_530F`).
const ALPHA_ACCOUNT: i32 = 0x7000_5301;
const BRAVO_ACCOUNT: i32 = 0x7000_5302;
const ALPHA_PLAYER: i32 = 0x7000_5311;
const BRAVO_PLAYER: i32 = 0x7000_5312;
const ALPHA_NAME: &str = "ORG07Alpha";
const BRAVO_NAME: &str = "ORG07Bravo";
const COMMAND_NAME: &str = "Org07 Wire Command";
const COMMAND_KEY: &str = "org07 wire command";
/// `BASE_INVITE_REQUEST_FLAG` (D-ORG06).
const BASE_REQUEST_FLAG: i32 = 1 << 29;

/// `WSTRING`: a u32 UTF-16 unit count, then the units LE.
fn wstring(s: &str) -> Vec<u8> {
    let units: Vec<u16> = s.encode_utf16().collect();
    let mut out = (units.len() as u32).to_le_bytes().to_vec();
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out
}

/// A direct entity-method payload is the 4-byte entity id, then the args.
fn args(payload: &[u8]) -> &[u8] {
    &payload[4..]
}

fn has_wstring(args: &[u8], needle: &str) -> bool {
    let w = wstring(needle);
    args.windows(w.len()).any(|win| win == w.as_slice())
}

async fn cleanup(pool: &sqlx::PgPool) {
    // The organization first: its cascade removes the member rows, so the
    // character deletes below fire no leader-promotion audit rows.
    let _ = sqlx::query("DELETE FROM sgw_organizations WHERE name_key = $1")
        .bind(COMMAND_KEY)
        .execute(pool)
        .await;
    for aid in [ALPHA_ACCOUNT, BRAVO_ACCOUNT] {
        let _ = sqlx::query("DELETE FROM sgw_organization_events WHERE from_account_id = $1")
            .bind(aid)
            .execute(pool)
            .await;
    }
    for pid in [ALPHA_PLAYER, BRAVO_PLAYER] {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
    }
    for aid in [ALPHA_ACCOUNT, BRAVO_ACCOUNT] {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(aid)
            .execute(pool)
            .await;
    }
}

/// A Command led by Alpha: the organization row, one rank row per Command
/// rank (the Leader row holds every bit, as the schema pins; the others
/// none, which this test never needs), and Alpha at rank 8.
async fn seed_command(pool: &sqlx::PgPool) -> i32 {
    let org_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_organizations (org_type, name, name_key) VALUES (2, $1, $2) \
         RETURNING org_id",
    )
    .bind(COMMAND_NAME)
    .bind(COMMAND_KEY)
    .fetch_one(pool)
    .await
    .expect("insert command");
    sqlx::query(
        "INSERT INTO sgw_organization_ranks (org_id, org_type, rank, permissions) \
         SELECT $1, 2, r, CASE WHEN r = 8 THEN 67108863 ELSE 0 END \
         FROM generate_series(1, 8) AS r",
    )
    .bind(org_id)
    .execute(pool)
    .await
    .expect("insert ranks");
    sqlx::query(
        "INSERT INTO sgw_organization_members (org_id, player_id, account_id, org_type, rank) \
         VALUES ($1, $2, $3, 2, 8)",
    )
    .bind(org_id)
    .bind(ALPHA_PLAYER)
    .bind(ALPHA_ACCOUNT)
    .execute(pool)
    .await
    .expect("insert leader");
    org_id
}

#[tokio::test]
async fn two_clients_invite_into_a_command() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;
    cleanup(&pool).await;
    insert_sentinel_account(&pool, ALPHA_ACCOUNT, "org07_alpha").await;
    insert_sentinel_account(&pool, BRAVO_ACCOUNT, "org07_bravo").await;
    let pos_b = [
        CASTLE_BASE_POS[0] + 20.0,
        CASTLE_BASE_POS[1],
        CASTLE_BASE_POS[2],
    ];
    insert_castle_character(
        &pool,
        ALPHA_ACCOUNT,
        ALPHA_PLAYER,
        ALPHA_NAME,
        CASTLE_BASE_POS,
    )
    .await;
    insert_castle_character(&pool, BRAVO_ACCOUNT, BRAVO_PLAYER, BRAVO_NAME, pos_b).await;
    let org_id = seed_command(&pool).await;

    let alpha = support::enter_castle(
        &server.auth_url,
        &credentials_for("org07_alpha"),
        ALPHA_PLAYER,
        41,
    )
    .await;
    let bravo = support::enter_castle(
        &server.auth_url,
        &credentials_for("org07_bravo"),
        BRAVO_PLAYER,
        42,
    )
    .await;
    let alpha_id = alpha.player_entity_id.unwrap();
    let bravo_id = bravo.player_entity_id.unwrap();

    // 1. Invite by type 2 (Command). The base lists a character online at
    // `onClientReady`, which may land after this send; a refused invite
    // ("No player by that name is online.") is simply sent again.
    let invite = GameSession::base_method(0xD0, &[&[2u8][..], &wstring(BRAVO_NAME)].concat());
    let mut offer = None;
    for _ in 0..5 {
        alpha.send_bundle(&invite, true).await.expect("send 0xD0");
        offer = wait_for(&bravo, Duration::from_secs(2), |m| {
            m.method_index == Some(34) && m.entity_id == Some(bravo_id)
        })
        .await;
        if offer.is_some() {
            break;
        }
    }
    let offer = offer.expect("Bravo never received onOrganizationInvite [34]");
    let a = args(&offer.payload);
    assert!(has_wstring(a, ALPHA_NAME), "[34] names the inviter");
    assert!(has_wstring(a, COMMAND_NAME), "[34] names the Command");
    // WSTRING inviter, UINT8 type, INT32 request id.
    let at = 4 + ALPHA_NAME.len() * 2;
    assert_eq!(a[at], 2, "organization type Command");
    let request_id = &a[at + 1..at + 5];
    let id = i32::from_le_bytes(request_id.try_into().unwrap());
    assert_ne!(id & BASE_REQUEST_FLAG, 0, "a base-issued request id");

    // 2. Bravo accepts: CM 8, which the cell forwards to the base.
    bravo
        .send_bundle(
            &GameSession::cell_method(8, bravo_id, &[request_id, &[1]].concat()),
            true,
        )
        .await
        .expect("send CM 8");
    let joined = wait_for(&bravo, Duration::from_secs(5), |m| {
        m.method_index == Some(35) && m.entity_id == Some(bravo_id)
    })
    .await
    .expect("Bravo never received onOrganizationJoined [35]");
    // INT32 org id, UINT8 type, UINT8 rank, UINT8 new member.
    let j = args(&joined.payload);
    assert_eq!(j[0..4], org_id.to_le_bytes(), "Bravo joined the Command");
    assert_eq!((j[4], j[5], j[6]), (2, 1, 1), "Command, Initiate, new");

    let (hit, _) = wait_for_recording(&alpha, Duration::from_secs(5), |m| {
        m.method_index == Some(37)
            && m.entity_id == Some(alpha_id)
            && has_wstring(args(&m.payload), BRAVO_NAME)
    })
    .await;
    let member = hit.expect("Alpha never received [37] for Bravo");
    let m = args(&member.payload);
    // ... INT32 org id, UINT8 rank, UINT8 new member at the end.
    assert_eq!(&m[m.len() - 2..], &[1, 1], "Initiate, new member");

    let rank: Option<i16> = sqlx::query_scalar(
        "SELECT rank FROM sgw_organization_members WHERE org_id = $1 AND player_id = $2",
    )
    .bind(org_id)
    .bind(BRAVO_PLAYER)
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(rank, Some(1), "Bravo is an Initiate of the Command");

    for s in [&alpha, &bravo] {
        let _ = s.send_bundle(&GameSession::disconnect(0), true).await;
    }
    cleanup(&pool).await;
    server.orchestrator.stop_all().await;
}
