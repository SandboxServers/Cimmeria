//! A client session for the mail tests, and a decoder for what it was sent.

use std::time::Instant;

use super::*;
use crate::mercury::method_idx;

/// One player's session, with the maps the mail handler reads.
pub(super) struct Client {
    pub(super) addr: SocketAddr,
    pub(super) entity_id: u32,
    pub(super) player_id: i32,
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl Client {
    /// A logged-in session for `player_id` on `entity_id`. `session_name`
    /// is the name the session holds, which the handler must not trust for
    /// anything stored or shown.
    pub(super) fn new(entity_id: u32, player_id: i32, port: u16, session_name: &str) -> Self {
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let mut state = crate::test_support::test_default_connected_client_state();
        state.player_entity_id = Some(entity_id);
        state.active_player_id = Some(player_id);
        state.player_name = Some(session_name.to_string());
        state.account_id = 0x7300_0001;
        let transport = Arc::new(TestTransport::default());
        Self {
            addr,
            entity_id,
            player_id,
            dyn_transport: transport.clone(),
            transport,
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
        }
    }

    /// Run one mail op through the router at `now`.
    pub(super) async fn op(&self, op: MailOp, pool: Option<&PgPool>, now: Instant) {
        let caller = Caller {
            entity_id: self.entity_id,
            player_id: self.player_id,
            transport: &self.dyn_transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        };
        route(caller, op, pool, now).await;
    }

    /// Everything sent to this client since the last call, decoded.
    pub(super) fn take(&self) -> Vec<Received> {
        self.transport
            .drain()
            .into_iter()
            .filter(|(to, _)| *to == self.addr)
            .map(|(_, p)| decode(&p, self.entity_id))
            .collect()
    }
}

/// One decoded client method.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Received {
    SendMailResult {
        result: u8,
        failed: Vec<String>,
        failed_flags: i32,
    },
    /// An `onPlayerCommunication` feedback line.
    Feedback(String),
    /// `onMailHeaderInfo`: `bArchive`, each header's `(id, flags)`, each
    /// header's `cash` in the same order, and every `MessageAttachment` as
    /// `[id, itemId, stackSize, durability, charges]`.
    HeaderInfo {
        b_archive: u8,
        headers: Vec<(i32, i32)>,
        cash: Vec<i32>,
        attachments: Vec<[i32; 5]>,
    },
    /// `onCashChanged(total)`.
    CashChanged(i32),
    /// `onRemoveItem(ItemIdList)`.
    RemoveItem(Vec<i32>),
    /// `onUpdateItem`: each listed item's `(id, stackSize)`.
    UpdateItem(Vec<(i32, i32)>),
    /// `onMailRead`: the mail id and `ToText`.
    MailRead {
        mail_id: i32,
        to_text: String,
    },
    Other(u16),
}

/// Decrypt one packet (all-zero test key) and decode its single method.
fn decode(packet: &[u8], entity_id: u32) -> Received {
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let pt = enc.decrypt(packet).expect("decrypt test packet");
    let body = &pt[1..pt.len() - 4];
    assert_eq!(
        u32::from_le_bytes(body[3..7].try_into().unwrap()),
        entity_id,
        "every mail reply is addressed to the caller's own entity"
    );
    // Direct encoding below the SGWPlayer threshold (61), extended above.
    let (method, args) = if body[0] == 0xBD {
        (61 + u16::from(body[7]), &body[8..])
    } else {
        (u16::from(body[0] & 0x7F), &body[7..])
    };
    let mut r = Reader { buf: args, off: 0 };
    match method {
        method_idx::SEND_MAIL_RESULT => {
            let result = r.u8();
            let n = r.u32();
            let failed = (0..n).map(|_| r.wstring()).collect();
            let failed_flags = r.i32();
            Received::SendMailResult {
                result,
                failed,
                failed_flags,
            }
        }
        method_idx::ON_PLAYER_COMMUNICATION => {
            let _speaker = r.wstring();
            let _flags = r.u8();
            let channel = r.u8();
            assert_eq!(channel, 9, "feedback rides the tell channel");
            Received::Feedback(r.wstring())
        }
        method_idx::ON_MAIL_HEADER_INFO => {
            let _reset = r.u8();
            let b_archive = r.u8();
            let n = r.u32();
            let mut cash = Vec::new();
            let headers = (0..n)
                .map(|_| {
                    let id = r.i32();
                    let _from = r.wstring();
                    let _from_id = r.i32();
                    let _subject = r.wstring();
                    let _subject_id = r.i32();
                    cash.push(r.i32());
                    let _sent = r.i32();
                    let _read = r.i32();
                    (id, r.i32())
                })
                .collect();
            let n = r.u32();
            let attachments = (0..n)
                .map(|_| [r.i32(), r.i32(), r.i32(), r.i32(), r.i32()])
                .collect();
            assert_eq!(r.off, r.buf.len(), "onMailHeaderInfo has no trailing bytes");
            Received::HeaderInfo {
                b_archive,
                headers,
                cash,
                attachments,
            }
        }
        method_idx::ON_CASH_CHANGED => Received::CashChanged(r.i32()),
        method_idx::ON_REMOVE_ITEM => {
            let n = r.u32();
            Received::RemoveItem((0..n).map(|_| r.i32()).collect())
        }
        method_idx::ON_UPDATE_ITEM => {
            // InvItem (`cimmeria_entity::inventory::InvItem::serialize`):
            // id, dbid, stackSize, slotId, containerId, bound(u8),
            // durability, ammoTypes(ARRAY<INT32>), curAmmoType, charges.
            let n = r.u32();
            let items = (0..n)
                .map(|_| {
                    let id = r.i32();
                    let _dbid = r.i32();
                    let stack = r.i32();
                    let _slot = r.i32();
                    let _container = r.i32();
                    let _bound = r.u8();
                    let _durability = r.i32();
                    for _ in 0..r.u32() {
                        r.i32();
                    }
                    let _cur_ammo = r.i32();
                    let _charges = r.i32();
                    (id, stack)
                })
                .collect();
            assert_eq!(r.off, r.buf.len(), "onUpdateItem has no trailing bytes");
            Received::UpdateItem(items)
        }
        method_idx::ON_MAIL_READ => {
            let mail_id = r.i32();
            let _body = r.wstring();
            let _body_id = r.i32();
            Received::MailRead {
                mail_id,
                to_text: r.wstring(),
            }
        }
        other => Received::Other(other),
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    off: usize,
}

impl Reader<'_> {
    fn bytes<const N: usize>(&mut self) -> [u8; N] {
        let out = self.buf[self.off..self.off + N].try_into().unwrap();
        self.off += N;
        out
    }
    fn u8(&mut self) -> u8 {
        self.bytes::<1>()[0]
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.bytes())
    }
    fn i32(&mut self) -> i32 {
        i32::from_le_bytes(self.bytes())
    }
    fn wstring(&mut self) -> String {
        let n = self.u32() as usize;
        let units: Vec<u16> = (0..n).map(|_| u16::from_le_bytes(self.bytes())).collect();
        String::from_utf16(&units).unwrap()
    }
}

/// A text-only [`MailSend`](crate::cell::messages::MailSend) to `names`.
pub(super) fn plain_send(names: &[&str]) -> crate::cell::messages::MailSend {
    crate::cell::messages::MailSend {
        recipient_flags: 0,
        recipients: names.iter().map(|n| n.to_string()).collect(),
        subject: "Subject".to_string(),
        body: "Body".to_string(),
        cash: 0,
        cod: false,
        item_id: 0,
        item_quantity: 0,
    }
}
