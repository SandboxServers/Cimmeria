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

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use super::super::super::super::ConnectedClientState;

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

/// The correlators every vendor row carries, resolved once per request.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VendorLog {
    action: &'static str,
    account_id: Option<u32>,
    player_id: i32,
    entity_id: u32,
    vendor_entity_id: Option<i32>,
    vendor_template_id: Option<i32>,
}

impl VendorLog {
    /// Resolve the session's account and build the row context.
    pub(crate) fn new(
        action: &'static str,
        entity_id: u32,
        player_id: i32,
        vendor_entity_id: Option<i32>,
        vendor_template_id: Option<i32>,
        connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) -> Self {
        let account_id = entity_to_addr
            .lock()
            .ok()
            .and_then(|m| m.get(&entity_id).copied())
            .and_then(|addr| connected.lock().ok()?.get(&addr).map(|c| c.account_id));
        Self::with_account(
            action,
            account_id,
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
        )
    }

    /// Build the row context with an already-known account.
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
    pub(crate) fn refused(&self, reason: &'static str, item: VendorItem) {
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

    /// `failed`: the server could not complete the request.
    pub(crate) fn failed(
        &self,
        reason: &'static str,
        item: VendorItem,
        error: &dyn std::fmt::Display,
    ) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    fn log() -> VendorLog {
        VendorLog::with_account("buy", Some(6), 900, 72, Some(4100), Some(31))
    }

    /// A refusal row carries the identity, the numeric item fields and
    /// `reason`.
    #[test]
    fn refused_row_carries_identity_item_and_reason() {
        let capture = LogCapture::install();
        log().refused(
            "insufficient_cash",
            VendorItem::design(5224).quantity(2).price(150),
        );
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

    #[test]
    fn failed_row_is_warn_with_error() {
        let capture = LogCapture::install();
        log().failed("db_error", VendorItem::row(77), &"connection reset");
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
        let log = VendorLog::new("open", 1, 72, Some(1), None, &connected, &e2a);
        let capture = LogCapture::install();
        log.opened([1, 2, 3, 4, 5]);
        let row = capture
            .find_message(Level::INFO, "vendor: store opened")
            .expect("opened row");
        assert!(!row.fields.contains_key("account_id"), "{row:#?}");
        assert!(row.has_field("recharge_count", "5"));
    }
}
