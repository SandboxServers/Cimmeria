//! Two real wire clients form a squad and one leaves (ORG-03).
//!
//! Alpha types `/squadinvite ORG03Bravo`, which the client sends as base
//! method 0xD0 `organizationInviteByType(0, name)` (ORG-E1 Q2). Bravo gets
//! `onOrganizationInvite` [34], accepts with CM 8, and both clients receive
//! `onOrganizationJoined` [35] and an `onMemberJoinedOrganization` [37]
//! naming the other. Bravo then leaves with CM 9: Bravo gets
//! `onOrganizationLeft` [36] `Requested`, and Alpha, left alone, gets
//! `onMemberLeftOrganization` [39] then [36] `Disbanded`.
//!
//! Against a spawned `Orchestrator`, over real SOAP auth and Mercury UDP.
//! Not in CI's live-DB job (see `docs/architecture/wireclient.md`). Run:
//! ```text
//! bash tools/build-lane/reload-db.sh
//! DATABASE_URL=<printed url> bash tools/build-lane/lane.sh \
//!   cargo test -p cimmeria-wireclient --test it two_client_squad -- --test-threads=1
//! ```

use std::time::Duration;

use cimmeria_wireclient::session::GameSession;

use crate::support::{
    self, credentials_for, insert_castle_character, insert_sentinel_account, live_db_pool_or_skip,
    start_server, wait_for, wait_for_recording, CASTLE_BASE_POS,
};

// Own block since #800: the old `0x7000_0301`/`_0302` players were
// `inventory/grant/tests.rs`'s `sgw_player` ids.
const ALPHA_ACCOUNT: i32 = 0x7000_8E11;
const BRAVO_ACCOUNT: i32 = 0x7000_8E12;
const ALPHA_PLAYER: i32 = 0x7000_8E01;
const BRAVO_PLAYER: i32 = 0x7000_8E02;
const BRAVO_NAME: &str = "ORG03Bravo";
const ALPHA_NAME: &str = "ORG03Alpha";

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

/// `true` when `args` contains `needle` as a `WSTRING`.
fn has_wstring(args: &[u8], needle: &str) -> bool {
    let w = wstring(needle);
    args.windows(w.len()).any(|win| win == w.as_slice())
}

async fn cleanup(pool: &sqlx::PgPool) {
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

#[tokio::test]
async fn two_clients_form_a_squad_and_leave() {
    let pool = match live_db_pool_or_skip().await {
        Some(p) => p,
        None => return,
    };
    let server = start_server(&std::env::var("DATABASE_URL").unwrap()).await;
    cleanup(&pool).await;
    insert_sentinel_account(&pool, ALPHA_ACCOUNT, "org03_alpha").await;
    insert_sentinel_account(&pool, BRAVO_ACCOUNT, "org03_bravo").await;
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

    let alpha = support::enter_castle(
        &server.auth_url,
        &credentials_for("org03_alpha"),
        ALPHA_PLAYER,
        31,
    )
    .await;
    let bravo = support::enter_castle(
        &server.auth_url,
        &credentials_for("org03_bravo"),
        BRAVO_PLAYER,
        32,
    )
    .await;
    let alpha_id = alpha.player_entity_id.unwrap();
    let bravo_id = bravo.player_entity_id.unwrap();

    // 1. /squadinvite. The cell learns each player's name from
    // `InitPlayerState`, which follows `onClientReady` asynchronously, so
    // an invite sent before Bravo's lands is refused ("No player named ...
    // online") and is simply sent again.
    let invite = GameSession::base_method(0xD0, &[&[0u8][..], &wstring(BRAVO_NAME)].concat());
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
    assert!(has_wstring(a, ALPHA_NAME), "[34] names the inviter: {a:?}");
    // WSTRING inviter, UINT8 type (0 = squad), INT32 request id.
    let at = 4 + ALPHA_NAME.len() * 2;
    assert_eq!(a[at], 0, "organization type Squad");
    let request_id = &a[at + 1..at + 5];

    // 2. Bravo accepts: CM 8 `organizationInviteResponse(request_id, 1)`.
    bravo
        .send_bundle(
            &GameSession::cell_method(8, bravo_id, &[request_id, &[1]].concat()),
            true,
        )
        .await
        .expect("send CM 8");
    let mut squad_id = Vec::new();
    for (who, session, own_id, other) in [
        ("Alpha", &alpha, alpha_id, BRAVO_NAME),
        ("Bravo", &bravo, bravo_id, ALPHA_NAME),
    ] {
        let (hit, seen) = wait_for_recording(session, Duration::from_secs(5), |m| {
            m.method_index == Some(37) && m.entity_id == Some(own_id)
        })
        .await;
        let joined = hit.unwrap_or_else(|| panic!("{who} never received [37]"));
        assert!(
            has_wstring(args(&joined.payload), other),
            "{who}'s [37] names {other}"
        );
        let joined_org = seen
            .iter()
            .find(|m| m.method_index == Some(35) && m.entity_id == Some(own_id))
            .unwrap_or_else(|| panic!("{who} received no [35] before [37]"));
        // INT32 org id, UINT8 type (0 = squad), UINT8 rank, UINT8 new.
        let j = args(&joined_org.payload);
        assert_eq!((j[4], j[6]), (0, 1), "{who}: a squad, newly joined");
        if squad_id.is_empty() {
            squad_id = j[0..4].to_vec();
        }
        assert_eq!(j[0..4], squad_id[..], "{who}: the same squad");
    }

    // 3. Bravo leaves: CM 9 with the squad id [35] carried.
    let squad_id = squad_id.as_slice();
    assert!(i32::from_le_bytes(squad_id.try_into().unwrap()) >= 0x4000_0000);
    bravo
        .send_bundle(&GameSession::cell_method(9, bravo_id, squad_id), true)
        .await
        .expect("send CM 9");
    let left = wait_for(&bravo, Duration::from_secs(5), |m| {
        m.method_index == Some(36) && m.entity_id == Some(bravo_id)
    })
    .await
    .expect("Bravo never received onOrganizationLeft [36]");
    assert_eq!(
        args(&left.payload),
        [&[0u8][..], squad_id].concat(),
        "Requested"
    );
    let (disband, seen) = wait_for_recording(&alpha, Duration::from_secs(5), |m| {
        m.method_index == Some(36) && m.entity_id == Some(alpha_id)
    })
    .await;
    let disband = disband.expect("Alpha never received [36] Disbanded");
    assert_eq!(args(&disband.payload), [&[2u8][..], squad_id].concat());
    assert!(
        seen.iter().any(|m| m.method_index == Some(39)
            && m.entity_id == Some(alpha_id)
            && has_wstring(args(&m.payload), BRAVO_NAME)),
        "Alpha received [39] for Bravo before the disband"
    );

    for s in [&alpha, &bravo] {
        let _ = s.send_bundle(&GameSession::disconnect(0), true).await;
    }
    cleanup(&pool).await;
    server.orchestrator.stop_all().await;
}
