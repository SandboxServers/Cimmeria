//! SS-C1: `chatIgnore` (0xC5) through the dispatcher.
//!
//! The refusals that stop before the database run everywhere (type 12). The
//! list edits are live-DB (type 3, `require_db_or_skip!`, sentinels
//! `0x7300_C2xx`, cleanup by exact id): the row lands on the caller's own
//! flags-301 list, the contact-list window gets its CM 87 / 88 echo, and the
//! session and cell copies of the list are reloaded.

use std::collections::HashSet;

use sqlx::PgPool;

use super::super::ignore::{
    ignore_full_text, IGNORE_BAD_REQUEST_TEXT, IGNORE_NO_TARGET_TEXT, IGNORE_SELF_TEXT,
    IGNORE_UNAVAILABLE_TEXT,
};
use super::super::*;
use crate::base::contact_list::ignore::{load_ignore_names, MAX_IGNORE_LIST_MEMBERS};
use crate::mercury::method_idx;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

const OWNER_PORT: u16 = 54900;
const OWNER_EID: u32 = 9100;

struct Harness {
    addr: SocketAddr,
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
    db_pool: Option<Arc<PgPool>>,
}

impl Harness {
    fn new(player_id: i32, db_pool: Option<Arc<PgPool>>) -> Self {
        let addr = SocketAddr::from(([127, 0, 0, 1], OWNER_PORT));
        let mut s = test_default_connected_client_state();
        s.player_name = Some(format!("ssc1-owner-{player_id}"));
        s.active_player_id = Some(player_id);
        s.player_entity_id = Some(OWNER_EID);
        s.listed_online = true;
        let transport = Arc::new(TestTransport::default());
        let (tx, rx) = mpsc::channel(8);
        Self {
            addr,
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, s)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(OWNER_EID, addr)]))),
            cell_tx: Some(tx),
            cell_rx: rx,
            db_pool,
        }
    }

    async fn raw(&self, payload: &[u8]) {
        dispatch_sgw_player_base_method(
            sgw_player_base::CHAT_IGNORE,
            payload,
            &None,
            self.addr,
            &self.dyn_transport,
            [0u8; 32],
            &self.connected,
            &Arc::new(Mutex::new(EntityManager::new())),
            &self.cell_tx,
            &self.entity_to_addr,
            &self.db_pool,
        )
        .await
        .expect("chatIgnore never propagates Err");
    }

    async fn ignore(&self, name: &str, flag: u8) {
        let mut payload = Vec::new();
        crate::mercury::write_wstring(&mut payload, name);
        payload.push(flag);
        self.raw(&payload).await;
    }

    /// Every packet to the owner as `(method, args)`, oldest first; clears.
    fn take(&self) -> Vec<(u16, Vec<u8>)> {
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        let out = self
            .transport
            .filter_to(self.addr)
            .iter()
            .map(|p| {
                let pt = enc.decrypt(p).expect("decrypt");
                let body = &pt[1..pt.len() - 4];
                assert_eq!(
                    u32::from_le_bytes(body[3..7].try_into().unwrap()),
                    OWNER_EID
                );
                // Methods from index 61 use the extended encoding: marker
                // 0xBD, then the sub-index after the entity id.
                if body[0] == 0xBD {
                    (61 + u16::from(body[7]), body[8..].to_vec())
                } else {
                    (u16::from(body[0] & 0x7F), body[7..].to_vec())
                }
            })
            .collect();
        self.transport.clear();
        out
    }

    /// The text of the last feedback line (`onPlayerCommunication` from
    /// "SYSTEM").
    fn last_feedback(packets: &[(u16, Vec<u8>)]) -> String {
        let (_, args) = packets
            .iter()
            .rev()
            .find(|(m, _)| *m == method_idx::ON_PLAYER_COMMUNICATION)
            .expect("a feedback line");
        let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
        let mut o = 4 + speaker_len * 2 + 2;
        let n = u32::from_le_bytes(args[o..o + 4].try_into().unwrap()) as usize;
        o += 4;
        let units: Vec<u16> = (0..n)
            .map(|i| u16::from_le_bytes([args[o + i * 2], args[o + i * 2 + 1]]))
            .collect();
        String::from_utf16(&units).unwrap()
    }

    fn cell_sets(&mut self) -> Vec<HashSet<String>> {
        let mut out = Vec::new();
        while let Ok(msg) = self.cell_rx.try_recv() {
            if let BaseToCellMsg::UpdateIgnoreList {
                entity_id,
                ignore_names,
                ..
            } = msg
            {
                assert_eq!(entity_id, OWNER_EID);
                out.push(ignore_names);
            }
        }
        out
    }
}

fn refused(capture: &crate::test_support::LogCaptureGuard, reason: &str) -> bool {
    capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "chat.ignore_refused") && c.has_field("reason", reason))
}

// ── no database needed ─────────────────────────────────────────────────────

/// Malformed payloads, an unknown flag, an empty name and a missing pool
/// each answer with one feedback line and a `reason`, and touch nothing.
#[tokio::test]
async fn chat_ignore_refusals_before_the_database() {
    let capture = LogCapture::install();
    let mut h = Harness::new(1, None);

    h.raw(&[0xFF, 0xFF]).await;
    assert_eq!(Harness::last_feedback(&h.take()), IGNORE_BAD_REQUEST_TEXT);
    assert!(refused(&capture, "decode_failed"));

    let mut no_flag = Vec::new();
    crate::mercury::write_wstring(&mut no_flag, "Bob");
    h.raw(&no_flag).await;
    assert_eq!(Harness::last_feedback(&h.take()), IGNORE_BAD_REQUEST_TEXT);

    h.ignore("Bob", 7).await;
    assert_eq!(Harness::last_feedback(&h.take()), IGNORE_BAD_REQUEST_TEXT);
    assert!(refused(&capture, "bad_flag"));

    h.ignore("", 1).await;
    assert_eq!(Harness::last_feedback(&h.take()), IGNORE_NO_TARGET_TEXT);
    assert!(refused(&capture, "no_target"));

    h.ignore("Bob", 1).await;
    assert_eq!(Harness::last_feedback(&h.take()), IGNORE_UNAVAILABLE_TEXT);
    assert!(refused(&capture, "no_db_pool"));

    assert!(h.cell_sets().is_empty(), "no refusal pushes an Ignore set");
}

// ── live DB ────────────────────────────────────────────────────────────────

const TEST_BASE: i32 = 0x7300_C200;

async fn cleanup(pool: &PgPool, ids: &[(i32, i32)]) {
    for &(account_id, player_id) in ids {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }
}

async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32, name: &str) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("ss-c1-chat-ignore-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert player");
}

/// CAT-L-07: `chatIgnore(name, 1)` puts the canonical name on the CALLER's
/// flags-301 list (not the other character's), echoes CM 87, confirms with a
/// feedback line, and reloads the session and cell copies. `chatIgnore(name,
/// 0)` takes it off again with CM 88.
#[tokio::test]
async fn chat_ignore_adds_to_own_ignore_list() {
    let pool = require_db_or_skip!();
    let owner = (TEST_BASE, TEST_BASE + 1);
    let target = (TEST_BASE + 2, TEST_BASE + 3);
    cleanup(&pool, &[owner, target]).await;
    insert_player(&pool, owner.0, owner.1, "ssc1-owner-a").await;
    insert_player(&pool, target.0, target.1, "SsC1Pest").await;
    let mut h = Harness::new(owner.1, Some(Arc::new(pool.clone())));

    h.ignore("ssc1pest", 1).await;
    let pkts = h.take();
    assert!(
        pkts.iter()
            .any(|(m, _)| *m == method_idx::ON_CONTACT_LIST_ADD_MEMBERS),
        "the contact-list window gets its CM 87 echo"
    );
    assert_eq!(
        Harness::last_feedback(&pkts),
        "You are now ignoring SsC1Pest."
    );
    let pest: HashSet<String> = ["SsC1Pest".to_string()].into();
    assert_eq!(
        load_ignore_names(&pool, owner.1).await.unwrap(),
        pest,
        "the canonical name is on the caller's own Ignore list"
    );
    assert!(
        load_ignore_names(&pool, target.1).await.unwrap().is_empty(),
        "the target's lists are untouched"
    );
    assert!(h.connected.lock().unwrap()[&h.addr]
        .ignore
        .ignores("SsC1Pest"));
    assert_eq!(h.cell_sets(), vec![pest]);

    h.ignore("SsC1Pest", 0).await;
    let pkts = h.take();
    assert!(pkts
        .iter()
        .any(|(m, _)| *m == method_idx::ON_CONTACT_LIST_REMOVE_MEMBERS));
    assert_eq!(
        Harness::last_feedback(&pkts),
        "You are no longer ignoring SsC1Pest."
    );
    assert!(load_ignore_names(&pool, owner.1).await.unwrap().is_empty());
    assert!(h.connected.lock().unwrap()[&h.addr].ignore.is_empty());
    assert_eq!(h.cell_sets(), vec![HashSet::new()]);

    cleanup(&pool, &[owner, target]).await;
}

/// CAT-L-07: a player cannot ignore their own character. Fails when the
/// self check in `dispatch::ignore` is removed (the name then lands on the
/// list).
#[tokio::test]
async fn chat_ignore_rejects_self() {
    let capture = LogCapture::install();
    let pool = require_db_or_skip!();
    let owner = (TEST_BASE + 10, TEST_BASE + 11);
    cleanup(&pool, &[owner]).await;
    insert_player(&pool, owner.0, owner.1, "SsC1Selfie").await;
    let mut h = Harness::new(owner.1, Some(Arc::new(pool.clone())));

    h.ignore("ssc1selfie", 1).await;
    assert_eq!(Harness::last_feedback(&h.take()), IGNORE_SELF_TEXT);
    assert!(refused(&capture, "self"));
    assert!(load_ignore_names(&pool, owner.1).await.unwrap().is_empty());
    assert!(h.cell_sets().is_empty());

    cleanup(&pool, &[owner]).await;
}

/// Unknown names, repeat adds and removes of absent names are refused with
/// their own line; a full list refuses the next add (CAT-L-04's cap).
#[tokio::test]
async fn chat_ignore_refuses_unknown_duplicate_absent_and_full() {
    let capture = LogCapture::install();
    let pool = require_db_or_skip!();
    let owner = (TEST_BASE + 20, TEST_BASE + 21);
    let target = (TEST_BASE + 22, TEST_BASE + 23);
    cleanup(&pool, &[owner, target]).await;
    insert_player(&pool, owner.0, owner.1, "ssc1-owner-c").await;
    insert_player(&pool, target.0, target.1, "SsC1Twice").await;
    let h = Harness::new(owner.1, Some(Arc::new(pool.clone())));

    h.ignore("ssc1-nobody-at-all", 1).await;
    assert_eq!(
        Harness::last_feedback(&h.take()),
        "No character named ssc1-nobody-at-all exists."
    );
    assert!(refused(&capture, "unknown_character"));

    h.ignore("SsC1Twice", 0).await;
    assert_eq!(
        Harness::last_feedback(&h.take()),
        "SsC1Twice is not on your Ignore list."
    );
    assert!(refused(&capture, "not_ignored"));

    h.ignore("SsC1Twice", 1).await;
    h.take();
    h.ignore("SsC1Twice", 1).await;
    assert_eq!(
        Harness::last_feedback(&h.take()),
        "SsC1Twice is already on your Ignore list."
    );
    assert!(refused(&capture, "already_ignored"));

    // Fill the list to the cap with names the UI could have added, then
    // try one more real character.
    h.ignore("SsC1Twice", 0).await;
    h.take();
    let list_id: i32 = sqlx::query_scalar(
        "SELECT list_id FROM sgw_contact_list WHERE player_id = $1 AND flags = 301",
    )
    .bind(owner.1)
    .fetch_one(&pool)
    .await
    .unwrap();
    let filler: Vec<String> = (0..MAX_IGNORE_LIST_MEMBERS)
        .map(|i| format!("filler-{i}"))
        .collect();
    sqlx::query(
        "INSERT INTO sgw_contact_list_member (list_id, player_name) \
         SELECT $1, n FROM UNNEST($2::text[]) AS t(n)",
    )
    .bind(list_id)
    .bind(&filler)
    .execute(&pool)
    .await
    .unwrap();
    h.ignore("SsC1Twice", 1).await;
    assert_eq!(Harness::last_feedback(&h.take()), ignore_full_text());
    assert!(refused(&capture, "list_full"));
    assert!(!load_ignore_names(&pool, owner.1)
        .await
        .unwrap()
        .contains("SsC1Twice"));

    cleanup(&pool, &[owner, target]).await;
}
