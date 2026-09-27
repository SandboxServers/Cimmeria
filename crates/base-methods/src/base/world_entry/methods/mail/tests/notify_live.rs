//! Type 8 (fan-out) guards for the new-mail notification (SS-M4, D-SS11):
//! who is told, over which delivery path, with which bytes. Sentinels:
//! accounts, players and entities `0x7300_2300` up, items `0x7300_2380` up.

use std::time::Instant;

use super::super::expiry::sweep_mailbox;
use super::super::system::{send_system_mail, SystemItem, SystemMail};
use super::packets::{decode, plain_send, Received};
use super::*;
use crate::base::feedback::FeedbackCtx;
use crate::cell::messages::{MailGmActor, MailGmCellToBase, MailOp};
use crate::mercury::method_idx;

const BASE: i32 = 0x7300_2300;
const ITEMS: i32 = 0x7300_2380;

/// Several sessions on one transport and one session map, so a send from
/// one can be seen (or not) by the others.
struct Room {
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// One session in a [`Room`].
#[derive(Debug, Clone, Copy)]
struct Seat {
    addr: SocketAddr,
    entity_id: u32,
    player_id: i32,
}

impl Room {
    fn new() -> Self {
        let transport = Arc::new(TestTransport::default());
        Self {
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::new())),
            entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// A session in the world for `player_id`; `listed` is what world entry
    /// sets and `logOff` clears (the online index).
    fn seat(&self, entity_id: u32, player_id: i32, port: u16, name: &str, listed: bool) -> Seat {
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let mut s = crate::test_support::test_default_connected_client_state();
        s.player_entity_id = Some(entity_id);
        s.active_player_id = Some(player_id);
        s.player_name = Some(name.to_string());
        s.account_id = 0x7300_0001;
        s.listed_online = listed;
        self.connected.lock().unwrap().insert(addr, s);
        self.entity_to_addr.lock().unwrap().insert(entity_id, addr);
        Seat {
            addr,
            entity_id,
            player_id,
        }
    }

    async fn op(&self, who: Seat, op: MailOp, pool: &PgPool) {
        let caller = Caller {
            entity_id: who.entity_id,
            player_id: who.player_id,
            transport: &self.dyn_transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        };
        route(caller, op, Some(pool), Instant::now()).await;
    }

    fn fb(&self) -> FeedbackCtx<'_> {
        FeedbackCtx {
            transport: &self.dyn_transport,
            connected: &self.connected,
        }
    }

    /// Everything sent since the last call, decoded, per seat.
    fn take(&self, seats: &[Seat]) -> Vec<Vec<Received>> {
        let sent = self.transport.drain();
        seats
            .iter()
            .map(|seat| {
                sent.iter()
                    .filter(|(to, _)| *to == seat.addr)
                    .map(|(_, p)| decode(p, seat.entity_id))
                    .collect()
            })
            .collect()
    }
}

/// The notification a seat should get for `mail_id`: the feedback line,
/// then the one-row header upsert (`ResetCategory` 0, inbox).
fn assert_notified(got: &[Received], mail_id: i32, line: &str) {
    match got {
        [Received::Feedback(text), Received::HeaderInfo {
            reset,
            b_archive,
            headers,
            ..
        }] => {
            assert_eq!(text, line);
            assert_eq!((*reset, *b_archive), (0, 0), "an upsert, never a reset");
            assert_eq!(headers.len(), 1);
            assert_eq!(headers[0].0, mail_id);
        }
        other => panic!("expected the notification for {mail_id}, got {other:?}"),
    }
}

/// D-SS11 fan-out: a send to three names reaches, as a notification, only
/// the recipient listed online. The offline one (no session) and the
/// logged-off one (a session, but `listed_online` cleared by `logOff`) get
/// nothing, and the sender gets only its own `sendMailResult`. Fails if the
/// send path stops notifying, or notifies any session holding the player id
/// rather than a listed one.
#[tokio::test]
async fn notification_reaches_only_the_online_recipient() {
    let pool = require_db_or_skip!();
    cleanup(&pool, BASE).await;
    let (sender, online, offline, logged_off) = (BASE + 1, BASE + 2, BASE + 3, BASE + 4);
    insert_players(
        &pool,
        BASE,
        &[
            (sender, "SsmFourNtfS"),
            (online, "SsmFourNtfOn"),
            (offline, "SsmFourNtfOff"),
            (logged_off, "SsmFourNtfGone"),
        ],
    )
    .await;
    let room = Room::new();
    let s = room.seat(BASE as u32 + 0x10, sender, 55_230, "SsmFourNtfS", true);
    let on = room.seat(BASE as u32 + 0x11, online, 55_231, "SsmFourNtfOn", true);
    let gone = room.seat(
        BASE as u32 + 0x12,
        logged_off,
        55_232,
        "SsmFourNtfGone",
        false,
    );

    room.op(
        s,
        MailOp::Send(plain_send(&[
            "SsmFourNtfOn",
            "SsmFourNtfOff",
            "SsmFourNtfGone",
        ])),
        &pool,
    )
    .await;

    let mail_id: i32 =
        sqlx::query_scalar("SELECT mail_id FROM sgw_gate_mail WHERE character_id = $1")
            .bind(online)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        mail_count(&pool, offline).await,
        1,
        "delivered, just not told"
    );
    let got = room.take(&[s, on, gone]);
    assert!(
        matches!(
            got[0].as_slice(),
            [Received::SendMailResult { result: 0, .. }]
        ),
        "the sender: {:?}",
        got[0]
    );
    assert_notified(&got[1], mail_id, "You have new gate-mail from SsmFourNtfS.");
    assert!(got[2].is_empty(), "logged off: {:?}", got[2]);

    cleanup(&pool, BASE).await;
}

/// D-SS11 on the other delivery paths, each told after its commit to the
/// online player the mail reached: a return (to the sender), a COD payment
/// (to the seller), an expiry return (to the sender, while the mailbox it
/// left loses the header), server mail and a GM `.mail` (to the recipient).
#[tokio::test]
async fn every_delivery_path_notifies_the_online_recipient() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x20;
    cleanup(&pool, base).await;
    let (a, b) = (base + 1, base + 2);
    insert_players(&pool, base, &[(a, "SsmFourNtfA"), (b, "SsmFourNtfB")]).await;
    set_naquadah(&pool, b, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let room = Room::new();
    let sa = room.seat(base as u32 + 0x10, a, 55_240, "SsmFourNtfA", true);
    let sb = room.seat(base as u32 + 0x11, b, 55_241, "SsmFourNtfB", true);

    // B returns A's mail: A is told.
    let gift = AttachedMail::from(b, a, "SsmFourNtfA")
        .cash(5)
        .insert(&pool)
        .await;
    room.op(sb, MailOp::Return { mail_id: gift }, &pool).await;
    let got = room.take(&[sa]);
    assert_notified(
        &got[0],
        gift,
        "SsmFourNtfB returned your gate-mail. It is back in your mailbox.",
    );

    // B pays A's COD: A is told of the payment mail.
    let cod = AttachedMail::from(b, a, "SsmFourNtfA")
        .cod(40)
        .item(ITEMS, type_id, 1)
        .insert(&pool)
        .await;
    room.op(sb, MailOp::PayCod { mail_id: cod }, &pool).await;
    let payment: i32 = sqlx::query_scalar(
        "SELECT MAX(mail_id) FROM sgw_gate_mail WHERE character_id = $1 AND sender_id IS NULL",
    )
    .bind(a)
    .fetch_one(&pool)
    .await
    .unwrap();
    let got = room.take(&[sa]);
    assert_notified(
        &got[0],
        payment,
        "SsmFourNtfB paid for your COD delivery. The payment is in your gate-mail.",
    );

    // A's mail expires in B's box: B loses the header, A is told.
    let stale = AttachedMail::from(b, a, "SsmFourNtfA")
        .cash(9)
        .insert(&pool)
        .await;
    set_expiry_state(&pool, stale, Some(1_000), false, false).await;
    let summary = sweep_mailbox(&pool, b, 1_000, Some(&room.fb())).await;
    assert_eq!(summary.returned, 1, "{summary:?}");
    let got = room.take(&[sa, sb]);
    assert_notified(
        &got[0],
        stale,
        "Your gate-mail to SsmFourNtfB expired unclaimed and was returned to your mailbox.",
    );
    assert_eq!(
        got[1],
        vec![Received::Other(method_idx::ON_MAIL_HEADER_REMOVE)],
        "the mailbox it left drops the header"
    );

    // Server mail: B is told, by the caller after its commit.
    let sent = send_system_mail(
        &pool,
        &SystemMail {
            sender_name: "Black Market".into(),
            recipient_player_id: b,
            subject: "Auction sold".into(),
            body: "Your item sold.".into(),
            cash: 12,
            item: SystemItem::None,
        },
    )
    .await
    .unwrap();
    sent.notify(&pool, &room.fb()).await;
    let got = room.take(&[sb]);
    assert_notified(
        &got[0],
        sent.mail_id,
        "You have new gate-mail from Black Market.",
    );

    // A GM `.mail` to B: B is told; the GM gets their confirmation only.
    super::super::handle_mail_gm(
        MailGmCellToBase::Send {
            actor: MailGmActor {
                entity_id: sa.entity_id,
                player_id: a,
                account_id: Some(0x7300_0001),
            },
            to: Some("SsmFourNtfB".into()),
            cash: 3,
            item: None,
            cod: None,
            subject: "GM test mail".into(),
        },
        &room.dyn_transport,
        &room.connected,
        &room.entity_to_addr,
        &Some(Arc::new(pool.clone())),
    )
    .await;
    let gm_mail: i32 =
        sqlx::query_scalar("SELECT MAX(mail_id) FROM sgw_gate_mail WHERE character_id = $1")
            .bind(b)
            .fetch_one(&pool)
            .await
            .unwrap();
    let got = room.take(&[sa, sb]);
    assert!(
        matches!(got[0].as_slice(), [Received::Feedback(t)] if t.starts_with("Mail ")),
        "the GM's confirmation: {:?}",
        got[0]
    );
    assert_notified(&got[1], gm_mail, "You have new gate-mail from SsmFourNtfA.");

    cleanup(&pool, base).await;
}

/// A quarantined mail leaves an online owner's list with a line that says
/// why (security review of SS-M4, LOW): without it an item they could see
/// vanishes unexplained.
#[tokio::test]
async fn quarantine_tells_the_online_owner() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x40;
    cleanup(&pool, base).await;
    let owner = base + 1;
    insert_players(&pool, base, &[(owner, "SsmFourNtfQ")]).await;
    let room = Room::new();
    let seat = room.seat(base as u32 + 0x10, owner, 55_260, "SsmFourNtfQ", true);
    let mail_id = AttachedMail {
        owner,
        sender_id: None,
        sender_name: "Black Market",
        cash: 15,
        flags: 0,
        item: None,
    }
    .insert(&pool)
    .await;
    set_expiry_state(&pool, mail_id, Some(1_000), false, false).await;

    sweep_mailbox(&pool, owner, 1_000, Some(&room.fb())).await;

    let got = room.take(&[seat]);
    assert_eq!(
        got[0],
        vec![
            Received::Other(method_idx::ON_MAIL_HEADER_REMOVE),
            Received::Feedback(
                "Your gate-mail \"Attached\" expired and could not be returned. It is held, \
                 with its attachments, for a GM to recover."
                    .into()
            ),
        ]
    );

    cleanup(&pool, base).await;
}
