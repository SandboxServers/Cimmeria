//! Vendor telemetry (`target: "vendor"`): one row per store open, one per
//! completed transaction, one per refusal.
//!
//! The 2026-09-29 colo playtest had zero vendor rows a player's
//! `account_id` could find: the handlers logged under their module path with
//! no account, and most refusals said only `"… insufficient naquadah"`. Every
//! row here carries `account_id` (from the session; absent when the session
//! is gone), `player_id`, `entity_id`, `vendor_entity_id`,
//! `vendor_template_id` and `action` (`open` \| `buy` \| `sell` \| `buyback`
//! \| `repair` \| `repair_all` \| `recharge` \| `recharge_all`), plus the item
//! fields the action has: `design_id` (the item type), `item_id` (the
//! inventory row), `quantity`, `price`, `cash_before` / `cash_after`.
//!
//! | `event` | Level | When |
//! |---|---|---|
//! | `store_opened` | INFO | the store window was sent; `buy_count`, `sell_count`, `buyback_count`, `repair_count`, `recharge_count` |
//! | `transaction` | INFO | a buy/sell/buyback/repair/recharge committed; `lines`, `price` (total cash), `cash_before`, `cash_after` |
//! | `refused` | INFO | the player's request was refused and nothing was written (`reason`, see each handler) |
//! | `failed` | WARN | the server could not complete it (a database error, a lost commit); `reason` and `error` |
//!
//! Levels follow `negative-logging-convention.md`: a refusal the player
//! caused (not enough cash, bags full, item not repairable) is expected and
//! is INFO so it is still exported (`OTEL_FILTER` has `vendor=info`); a
//! server-side failure is WARN. `vendor.*` spans stay as they were.
//!
//! Every `refused` and `failed` row also sends the player one
//! `CHAN_FEEDBACK` line ([`refusal_text`], [`FAILED_TEXT`]): owner rule,
//! every button press gets visible feedback on the first press. No stock
//! `ErrorStrings` id says "not enough naquadah" or any other vendor reason,
//! so the line is chat text, not `onErrorCode`. The exceptions are the
//! reasons only a forged packet or a server-side state produces, which stay
//! silent (see [`refusal_text`]). A line that cannot be sent is a WARN
//! (`event = "feedback_send_failed"`).

use cimmeria_entity::known_names;
use cimmeria_mercury::transport::Transport;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use super::super::super::super::ConnectedClientState;
use crate::base::feedback::{send_feedback_line, FeedbackCtx, FeedbackOutcome};

/// The line for a request the server could not complete (a database error,
/// a lost commit).
pub(crate) const FAILED_TEXT: &str = "The vendor could not complete that request.";

/// The player's line for a `refused` reason, or `None` for the reasons a
/// stock client cannot cause:
///
/// - `empty_request`: the cell refuses an empty or malformed payload before
///   it reaches the base, so only a forged packet gets here; answering would
///   hand a forging client a free message per packet.
/// - `no_vendor_template`: the store window opens empty, which is the
///   feedback.
///
/// Any other reason, including one added later, gets a line.
pub(crate) fn refusal_text(reason: &str) -> Option<&'static str> {
    Some(match reason {
        "empty_request" | "no_vendor_template" => return None,
        "insufficient_cash" => "You do not have enough naquadah.",
        "missing_item_cost" => "You do not have the items this purchase requires.",
        "bags_full" => "You do not have room in your bags.",
        "not_on_vendor_list" => "This vendor does not sell that item.",
        "vendor_buys_nothing" => "This vendor does not buy items.",
        "not_sellable" => "This vendor will not buy that item.",
        "buyback_full" => "This vendor cannot take any more items right now.",
        "quantity_exceeds_stack" => "You do not have that many.",
        "not_in_buyback" => "That item is no longer available to buy back.",
        "vendor_does_not_repair" => "This vendor does not repair items.",
        "not_repairable" => "That item does not need or cannot get a repair here.",
        "vendor_does_not_recharge" => "This vendor does not recharge items.",
        "not_rechargeable" => "That item does not need or cannot get a recharge here.",
        "cash_overflow" => "You cannot carry that much naquadah.",
        // negative_price, price_overflow and anything new.
        _ => "The vendor cannot complete that transaction.",
    })
}

/// What a row needs to reach the player's client.
#[derive(Clone, Copy)]
struct VendorNet<'a> {
    transport: &'a Arc<dyn Transport>,
    connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// The item a row is about. Every field is optional: a whole-inventory
/// repair names no item, a purchase names a design but no row.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VendorItem {
    pub design_id: Option<i32>,
    pub item_id: Option<i32>,
    pub quantity: Option<i32>,
    pub price: Option<i64>,
    /// The player's cash when the request was judged (on a cash refusal).
    pub cash: Option<i64>,
}

impl VendorItem {
    pub(crate) fn design(design_id: i32) -> Self {
        Self {
            design_id: Some(design_id),
            ..Self::default()
        }
    }

    pub(crate) fn row(item_id: i32) -> Self {
        Self {
            item_id: Some(item_id),
            ..Self::default()
        }
    }

    pub(crate) fn quantity(self, quantity: i32) -> Self {
        Self {
            quantity: Some(quantity),
            ..self
        }
    }

    pub(crate) fn price(self, price: impl Into<i64>) -> Self {
        Self {
            price: Some(price.into()),
            ..self
        }
    }

    pub(crate) fn cash(self, cash: impl Into<i64>) -> Self {
        Self {
            cash: Some(cash.into()),
            ..self
        }
    }
}

/// The correlators every vendor row carries, resolved once per request,
/// and the session the refusal and failure lines go to.
#[derive(Clone, Copy)]
pub(crate) struct VendorLog<'a> {
    action: &'static str,
    account_id: Option<u32>,
    player_id: i32,
    entity_id: u32,
    vendor_entity_id: Option<i32>,
    vendor_template_id: Option<i32>,
    /// `None` only in the row-shape unit tests: no line is sent.
    net: Option<VendorNet<'a>>,
}

impl std::fmt::Debug for VendorLog<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VendorLog")
            .field("action", &self.action)
            .field("account_id", &self.account_id)
            .field("player_id", &self.player_id)
            .field("entity_id", &self.entity_id)
            .field("vendor_entity_id", &self.vendor_entity_id)
            .field("vendor_template_id", &self.vendor_template_id)
            .finish_non_exhaustive()
    }
}

impl<'a> VendorLog<'a> {
    /// Resolve the session's account and build the row context.
    pub(crate) fn new(
        action: &'static str,
        entity_id: u32,
        player_id: i32,
        vendor_entity_id: Option<i32>,
        vendor_template_id: Option<i32>,
        transport: &'a Arc<dyn Transport>,
        connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) -> Self {
        let account_id = entity_to_addr
            .lock()
            .ok()
            .and_then(|m| m.get(&entity_id).copied())
            .and_then(|addr| connected.lock().ok()?.get(&addr).map(|c| c.account_id));
        Self {
            net: Some(VendorNet {
                transport,
                connected,
                entity_to_addr,
            }),
            ..Self::with_account(
                action,
                account_id,
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
            )
        }
    }

    /// Build the row context with an already-known account and no session
    /// to answer (rows only).
    pub(crate) fn with_account(
        action: &'static str,
        account_id: Option<u32>,
        entity_id: u32,
        player_id: i32,
        vendor_entity_id: Option<i32>,
        vendor_template_id: Option<i32>,
    ) -> Self {
        Self {
            action,
            account_id,
            player_id,
            entity_id,
            vendor_entity_id,
            vendor_template_id,
            net: None,
        }
    }

    /// Send the player one feedback line. A line that does not go out is a
    /// WARN: the player saw nothing for this press.
    async fn tell_player(&self, reason: &'static str, text: &str) {
        let Some(net) = self.net else {
            return;
        };
        let addr = net
            .entity_to_addr
            .lock()
            .ok()
            .and_then(|m| m.get(&self.entity_id).copied());
        let outcome = match addr {
            Some(addr) => {
                let ctx = FeedbackCtx {
                    transport: net.transport,
                    connected: net.connected,
                };
                send_feedback_line(&ctx, addr, text).await
            }
            None => FeedbackOutcome::NoSession,
        };
        if outcome != FeedbackOutcome::Sent {
            let player_label = known_names::player_name(self.player_id);
            tracing::warn!(
                target: "vendor",
                event = "feedback_send_failed",
                action = self.action,
                account_id = self.account_id,
                account_name = known_names::account_name(self.account_id),
                player_id = self.player_id,
                player_name = player_label,
                entity_id = self.entity_id,
                entity_name = player_label,
                reason,
                outcome = ?outcome,
                "vendor: refusal line not sent -- the player sees nothing for this press"
            );
        }
    }

    /// `store_opened`.
    pub(crate) fn opened(&self, counts: [usize; 5]) {
        let [buy, sell, buyback, repair, recharge] = counts;
        let player_label = known_names::player_name(self.player_id);
        tracing::info!(
            target: "vendor",
            event = "store_opened",
            action = self.action,
            account_id = self.account_id,
            account_name = known_names::account_name(self.account_id),
            player_id = self.player_id,
            player_name = player_label,
            entity_id = self.entity_id,
            entity_name = player_label,
            vendor_entity_id = self.vendor_entity_id, // nt:id-only vendor NPC, unnamed on base
            vendor_template_id = self.vendor_template_id,
            vendor_template_name = cimmeria_names::owned::template(self.vendor_template_id),
            buy_count = buy,
            sell_count = sell,
            buyback_count = buyback,
            repair_count = repair,
            recharge_count = recharge,
            "vendor: store opened"
        );
    }

    /// `transaction`: the request committed. `item` names the single item
    /// when there was one; `lines` counts every line; `price` on `item` is
    /// the total cash moved.
    pub(crate) fn completed(
        &self,
        item: VendorItem,
        lines: usize,
        cash_before: Option<i64>,
        cash_after: Option<i64>,
    ) {
        let player_label = known_names::player_name(self.player_id);
        tracing::info!(
            target: "vendor",
            event = "transaction",
            action = self.action,
            account_id = self.account_id,
            account_name = known_names::account_name(self.account_id),
            player_id = self.player_id,
            player_name = player_label,
            entity_id = self.entity_id,
            entity_name = player_label,
            vendor_entity_id = self.vendor_entity_id, // nt:id-only vendor NPC, unnamed on base
            vendor_template_id = self.vendor_template_id,
            vendor_template_name = cimmeria_names::owned::template(self.vendor_template_id),
            item_type_id = item.design_id,
            item_name = cimmeria_names::owned::item(item.design_id),
            item_id = item.item_id,
            quantity = item.quantity,
            price = item.price,
            lines,
            cash_before,
            cash_after,
            "vendor: transaction committed"
        );
    }

    /// `refused`: the player's request was refused, nothing was written.
    /// The player gets [`refusal_text`]'s line for `reason`.
    pub(crate) async fn refused(&self, reason: &'static str, item: VendorItem) {
        self.log_refused(reason, item);
        if let Some(text) = refusal_text(reason) {
            self.tell_player(reason, text).await;
        }
    }

    fn log_refused(&self, reason: &'static str, item: VendorItem) {
        let player_label = known_names::player_name(self.player_id);
        tracing::info!(
            target: "vendor",
            event = "refused",
            action = self.action,
            account_id = self.account_id,
            account_name = known_names::account_name(self.account_id),
            player_id = self.player_id,
            player_name = player_label,
            entity_id = self.entity_id,
            entity_name = player_label,
            vendor_entity_id = self.vendor_entity_id, // nt:id-only vendor NPC, unnamed on base
            vendor_template_id = self.vendor_template_id,
            vendor_template_name = cimmeria_names::owned::template(self.vendor_template_id),
            item_type_id = item.design_id,
            item_name = cimmeria_names::owned::item(item.design_id),
            item_id = item.item_id,
            quantity = item.quantity,
            price = item.price,
            cash = item.cash,
            reason,
            "vendor: request refused"
        );
    }

    /// `failed`: the server could not complete the request. The player gets
    /// [`FAILED_TEXT`].
    pub(crate) async fn failed(
        &self,
        reason: &'static str,
        item: VendorItem,
        error: &(dyn std::fmt::Display + Sync),
    ) {
        self.log_failed(reason, item, error);
        self.tell_player(reason, FAILED_TEXT).await;
    }

    fn log_failed(&self, reason: &'static str, item: VendorItem, error: &dyn std::fmt::Display) {
        let player_label = known_names::player_name(self.player_id);
        tracing::warn!(
            target: "vendor",
            event = "failed",
            action = self.action,
            account_id = self.account_id,
            account_name = known_names::account_name(self.account_id),
            player_id = self.player_id,
            player_name = player_label,
            entity_id = self.entity_id,
            entity_name = player_label,
            vendor_entity_id = self.vendor_entity_id, // nt:id-only vendor NPC, unnamed on base
            vendor_template_id = self.vendor_template_id,
            vendor_template_name = cimmeria_names::owned::template(self.vendor_template_id),
            item_type_id = item.design_id,
            item_name = cimmeria_names::owned::item(item.design_id),
            item_id = item.item_id,
            quantity = item.quantity,
            price = item.price,
            reason,
            error = %error,
            "vendor: request failed on the server"
        );
    }
}

/// Test helper: how many packets sent to `addr` (all-zero test key) carry
/// the exact `onPlayerCommunication("SYSTEM", 0, CHAN_FEEDBACK, text)` args.
#[cfg(test)]
pub(super) fn feedback_sent(
    transport: &crate::test_support::TestTransport,
    addr: SocketAddr,
    text: &str,
) -> usize {
    use crate::base::feedback::{serialize_on_player_communication, CHAN_FEEDBACK};
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let args = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, text);
    transport
        .filter_to(addr)
        .iter()
        .filter_map(|p| enc.decrypt(p).ok())
        .filter(|pt| pt.windows(args.len()).any(|w| w == args.as_slice()))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use tracing::Level;

    fn log() -> VendorLog<'static> {
        VendorLog::with_account("buy", Some(6), 900, 72, Some(4100), Some(31))
    }

    /// A refusal row carries the identity, the numeric item fields and
    /// `reason`.
    #[tokio::test]
    async fn refused_row_carries_identity_item_and_reason() {
        let capture = LogCapture::install();
        log()
            .refused(
                "insufficient_cash",
                VendorItem::design(5224).quantity(2).price(150),
            )
            .await;
        let row = capture
            .find_event(Level::INFO, "vendor: request refused", "insufficient_cash")
            .expect("refused row");
        assert_eq!(row.target, "vendor");
        for (k, v) in [
            ("event", "refused"),
            ("action", "buy"),
            ("account_id", "6"),
            ("player_id", "72"),
            ("entity_id", "900"),
            ("vendor_entity_id", "4100"),
            ("vendor_template_id", "31"),
            ("item_type_id", "5224"),
            ("quantity", "2"),
            ("price", "150"),
        ] {
            assert_eq!(row.fields.get(k).map(String::as_str), Some(v), "field {k}");
        }
        assert!(!row.fields.contains_key("item_id"), "no row id, no field");
    }

    /// NT-22 (Rule 6): the transaction row, the vendor's most-read line,
    /// names the character, the login, the vendor and the item. The vendor
    /// NPC's entity id stays unnamed on the base; its template names it.
    #[test]
    fn completed_row_names_the_player_the_vendor_and_the_item() {
        let mut book = cimmeria_names::NameBook::empty();
        book.insert(cimmeria_names::Table::Items, 5224, "Zat'nik'tel");
        book.insert(cimmeria_names::Table::Templates, 31, "Vendor_Weapons_SGC");
        cimmeria_names::global().store(book);
        cimmeria_entity::known_names::remember_player(72, "Jack O'Neill");
        cimmeria_entity::known_names::remember_account(6_u32, "nt22_vendor");
        let capture = LogCapture::install();
        log().completed(
            VendorItem::design(5224).quantity(1).price(150),
            1,
            Some(500),
            Some(350),
        );
        let row = capture
            .find_message(Level::INFO, "vendor: transaction committed")
            .expect("transaction row");
        for (k, v) in [
            ("player_name", "Jack O'Neill"),
            ("entity_name", "Jack O'Neill"),
            ("account_name", "nt22_vendor"),
            ("vendor_template_name", "Vendor_Weapons_SGC"),
            ("item_type_id", "5224"),
            ("item_name", "Zat'nik'tel"),
        ] {
            assert_eq!(
                row.fields.get(k).map(String::as_str),
                Some(v),
                "field {k}: {row:#?}"
            );
        }
    }

    #[tokio::test]
    async fn failed_row_is_warn_with_error() {
        let capture = LogCapture::install();
        log()
            .failed("db_error", VendorItem::row(77), &"connection reset")
            .await;
        let row = capture
            .find_event(Level::WARN, "vendor: request failed", "db_error")
            .expect("failed row");
        assert!(row.has_field("item_id", "77"));
        assert!(row.has_field("error", "connection reset"));
    }

    /// No session for the entity: the row omits `account_id` rather than
    /// inventing one.
    #[test]
    fn missing_session_omits_account() {
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let e2a = Arc::new(Mutex::new(HashMap::new()));
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let log = VendorLog::new("open", 1, 72, Some(1), None, &transport, &connected, &e2a);
        let capture = LogCapture::install();
        log.opened([1, 2, 3, 4, 5]);
        let row = capture
            .find_message(Level::INFO, "vendor: store opened")
            .expect("opened row");
        assert!(!row.fields.contains_key("account_id"), "{row:#?}");
        assert!(row.has_field("recharge_count", "5"));
    }

    const ADDR: &str = "127.0.0.1:54610";

    /// A player in the world at [`ADDR`] as entity 900.
    type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
    type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

    fn session() -> (
        Arc<TestTransport>,
        Arc<dyn Transport>,
        Connected,
        EntityToAddr,
    ) {
        let addr: SocketAddr = ADDR.parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(900);
        let test_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = test_transport.clone();
        (
            test_transport,
            transport,
            Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            Arc::new(Mutex::new(HashMap::from([(900, addr)]))),
        )
    }

    /// Owner rule: a refusal the player caused answers the press with one
    /// line. Removing the send from `refused` fails this.
    #[tokio::test]
    async fn a_refusal_sends_the_player_its_line() {
        let (test_transport, transport, connected, e2a) = session();
        let tel = VendorLog::new(
            "buy",
            900,
            72,
            Some(4100),
            Some(31),
            &transport,
            &connected,
            &e2a,
        );
        tel.refused("insufficient_cash", VendorItem::default())
            .await;
        assert_eq!(test_transport.filter_to(ADDR.parse().unwrap()).len(), 1);
        assert_eq!(
            feedback_sent(
                &test_transport,
                ADDR.parse().unwrap(),
                "You do not have enough naquadah."
            ),
            1
        );
    }

    /// `empty_request` (only a forged packet reaches the base with one) is
    /// logged but not answered.
    #[tokio::test]
    async fn a_forged_empty_request_is_not_answered() {
        let (test_transport, transport, connected, e2a) = session();
        let tel = VendorLog::new(
            "repair",
            900,
            72,
            None,
            Some(31),
            &transport,
            &connected,
            &e2a,
        );
        tel.refused("empty_request", VendorItem::default()).await;
        assert!(test_transport.is_empty());
    }

    #[tokio::test]
    async fn a_server_failure_tells_the_player() {
        let (test_transport, transport, connected, e2a) = session();
        let tel = VendorLog::new(
            "sell",
            900,
            72,
            None,
            Some(31),
            &transport,
            &connected,
            &e2a,
        );
        tel.failed("db_error", VendorItem::default(), &"boom").await;
        assert_eq!(
            feedback_sent(&test_transport, ADDR.parse().unwrap(), FAILED_TEXT),
            1
        );
    }

    /// Every reason a stock client can cause has a line.
    #[test]
    fn every_player_caused_reason_has_a_line() {
        for reason in [
            "insufficient_cash",
            "not_repairable",
            "not_on_vendor_list",
            "quantity_exceeds_stack",
            "not_in_buyback",
            "bags_full",
            "price_overflow",
            "some_future_reason",
        ] {
            assert!(refusal_text(reason).is_some(), "{reason}");
        }
        assert_eq!(refusal_text("empty_request"), None);
        assert_eq!(refusal_text("no_vendor_template"), None);
    }
}
