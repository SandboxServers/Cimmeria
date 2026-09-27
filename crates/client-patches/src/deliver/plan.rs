//! What each decoded call becomes on the Lua side, as data.
//!
//! This is the contract with the UI overlay (`CimmeriaBM`, written in
//! Lua); `crates/client-patches/README.md` states it for the overlay's
//! authors. Every call is a plain function call on the global table
//! [`TABLE`], with no `self`:
//!
//! | Client method | Lua call |
//! |---|---|
//! | `onBMOpen(entityId)` | `CimmeriaBM.onOpen(entityId)` |
//! | `onBMError(errorId)` | `CimmeriaBM.onError(errorId)` |
//! | `onBMAuctions(items, totalResults, clientKey)` | `CimmeriaBM.onAuctions(items, totalResults, clientKey)` |
//! | `onBMAuctionRemove(sequenceId)` | `CimmeriaBM.onAuctionRemove(sequenceId)` |
//! | `onBMAuctionUpdate(item)` | `CimmeriaBM.onAuctionUpdate(item)` |
//! | `onBMWatchedItemsUpdate(itemList)` | `CimmeriaBM.onWatchedItems(itemList)` |
//!
//! `items` and `itemList` are 1-based arrays. An auction item is a table
//! keyed by the `.def` names: `sequenceId`, `itemDefId`, `stackSize`,
//! `durability`, `charges`, `currentBid`, `buyoutPrice`, `endTimeValue`,
//! `nextMinBidPrice` (numbers) and `sellerName` (a string).

use cimmeria_patch_wire::black_market::{AuctionItem, ClientCall, ClientMethod};

/// The global table the overlay defines.
pub const TABLE: &str = "CimmeriaBM";

/// A value to push onto the Lua stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LuaValue {
    /// A number.
    Int(i32),
    /// A string.
    Str(String),
    /// A table with string keys, in this order.
    Record(Vec<(&'static str, LuaValue)>),
    /// A 1-based array.
    List(Vec<LuaValue>),
}

/// One call into the overlay: `TABLE[function](args...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaCall {
    /// The field of [`TABLE`] to call.
    pub function: &'static str,
    /// The arguments, in order.
    pub args: Vec<LuaValue>,
}

/// The overlay function that handles `method`.
pub fn handler_name(method: ClientMethod) -> &'static str {
    match method {
        ClientMethod::OnBMOpen => "onOpen",
        ClientMethod::OnBMError => "onError",
        ClientMethod::OnBMAuctions => "onAuctions",
        ClientMethod::OnBMAuctionRemove => "onAuctionRemove",
        ClientMethod::OnBMAuctionUpdate => "onAuctionUpdate",
        ClientMethod::OnBMWatchedItemsUpdate => "onWatchedItems",
    }
}

/// An auction row as a Lua table.
pub fn auction_item(item: &AuctionItem) -> LuaValue {
    LuaValue::Record(vec![
        ("sequenceId", LuaValue::Int(item.sequence_id)),
        ("itemDefId", LuaValue::Int(item.item_def_id)),
        ("stackSize", LuaValue::Int(item.stack_size)),
        ("durability", LuaValue::Int(item.durability)),
        ("charges", LuaValue::Int(item.charges)),
        ("currentBid", LuaValue::Int(item.current_bid)),
        ("buyoutPrice", LuaValue::Int(item.buyout_price)),
        (
            "endTimeValue",
            LuaValue::Int(i32::from(item.end_time_value)),
        ),
        ("nextMinBidPrice", LuaValue::Int(item.next_min_bid_price)),
        ("sellerName", LuaValue::Str(item.seller_name.clone())),
    ])
}

/// The overlay call for a decoded client method call.
pub fn plan(call: &ClientCall) -> LuaCall {
    let args = match call {
        ClientCall::Open(a) => vec![LuaValue::Int(a.entity_id)],
        ClientCall::Error(a) => vec![LuaValue::Int(a.error_id)],
        ClientCall::Auctions(a) => vec![
            LuaValue::List(a.auction_items.iter().map(auction_item).collect()),
            LuaValue::Int(a.total_results),
            LuaValue::Int(a.client_key),
        ],
        ClientCall::AuctionRemove(a) => vec![LuaValue::Int(a.sequence_id)],
        ClientCall::AuctionUpdate(a) => vec![auction_item(&a.auction_item)],
        ClientCall::WatchedItemsUpdate(a) => vec![LuaValue::List(
            a.item_list.iter().copied().map(LuaValue::Int).collect(),
        )],
    };
    LuaCall {
        function: handler_name(call.method()),
        args,
    }
}

/// Stack slots pushing `value` needs at its deepest: the value itself,
/// plus, while a table is being filled, the table and the entry on top of
/// it.
pub fn slots(value: &LuaValue) -> i32 {
    match value {
        LuaValue::Int(_) | LuaValue::Str(_) => 1,
        LuaValue::Record(fields) => 1 + fields.iter().map(|(_, v)| slots(v)).max().unwrap_or(0),
        LuaValue::List(items) => 1 + items.iter().map(slots).max().unwrap_or(0),
    }
}

/// Stack slots a whole call needs: the table and the function, then each
/// argument on top of the ones before it.
pub fn call_slots(call: &LuaCall) -> i32 {
    let args = call
        .args
        .iter()
        .enumerate()
        .map(|(i, v)| i as i32 + slots(v))
        .max()
        .unwrap_or(0);
    2 + args
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_patch_wire::black_market::{
        OnBMAuctionRemove, OnBMAuctionUpdate, OnBMAuctions, OnBMError, OnBMOpen,
        OnBMWatchedItemsUpdate,
    };

    fn item() -> AuctionItem {
        AuctionItem {
            sequence_id: 1,
            item_def_id: 2,
            stack_size: 3,
            durability: 4,
            charges: 5,
            current_bid: 6,
            buyout_price: 7,
            end_time_value: 8,
            next_min_bid_price: 9,
            seller_name: "Teal'c".into(),
        }
    }

    #[test]
    fn every_method_has_its_handler() {
        let calls = [
            (ClientCall::Open(OnBMOpen { entity_id: 1 }), "onOpen"),
            (ClientCall::Error(OnBMError { error_id: 1 }), "onError"),
            (ClientCall::Auctions(OnBMAuctions::default()), "onAuctions"),
            (
                ClientCall::AuctionRemove(OnBMAuctionRemove { sequence_id: 1 }),
                "onAuctionRemove",
            ),
            (
                ClientCall::AuctionUpdate(OnBMAuctionUpdate::default()),
                "onAuctionUpdate",
            ),
            (
                ClientCall::WatchedItemsUpdate(OnBMWatchedItemsUpdate::default()),
                "onWatchedItems",
            ),
        ];
        for (call, handler) in calls {
            assert_eq!(plan(&call).function, handler);
        }
    }

    #[test]
    fn auction_item_keys_are_the_def_names_in_def_order() {
        let LuaValue::Record(fields) = auction_item(&item()) else {
            panic!("an auction item is a record");
        };
        let keys: Vec<&str> = fields.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            keys,
            [
                "sequenceId",
                "itemDefId",
                "stackSize",
                "durability",
                "charges",
                "currentBid",
                "buyoutPrice",
                "endTimeValue",
                "nextMinBidPrice",
                "sellerName"
            ]
        );
        assert_eq!(fields[7].1, LuaValue::Int(8), "endTimeValue as a number");
        assert_eq!(fields[9].1, LuaValue::Str("Teal'c".into()));
    }

    /// `(items, totalResults, clientKey)`, the `.def` order, with items as
    /// a list of records.
    #[test]
    fn on_auctions_passes_items_total_then_client_key() {
        let call = ClientCall::Auctions(OnBMAuctions {
            auction_items: vec![item(), item()],
            total_results: 12,
            client_key: 1,
        });
        let planned = plan(&call);
        assert_eq!(
            planned.args,
            vec![
                LuaValue::List(vec![auction_item(&item()), auction_item(&item())]),
                LuaValue::Int(12),
                LuaValue::Int(1),
            ]
        );
    }

    #[test]
    fn watched_items_is_a_list_of_numbers() {
        let call = ClientCall::WatchedItemsUpdate(OnBMWatchedItemsUpdate {
            item_list: vec![4, 5],
        });
        assert_eq!(
            plan(&call).args,
            vec![LuaValue::List(vec![LuaValue::Int(4), LuaValue::Int(5)])]
        );
    }

    #[test]
    fn stack_needs_cover_the_deepest_push() {
        let open = plan(&ClientCall::Open(OnBMOpen { entity_id: 1 }));
        assert_eq!(call_slots(&open), 3, "table, function, one number");
        let page = plan(&ClientCall::Auctions(OnBMAuctions {
            auction_items: vec![item()],
            total_results: 1,
            client_key: 0,
        }));
        // table + function + (list + record + field) while the first
        // argument is built; later arguments sit on top of it but are flat.
        assert_eq!(call_slots(&page), 5);
        let empty = plan(&ClientCall::Auctions(OnBMAuctions::default()));
        assert_eq!(
            call_slots(&empty),
            5,
            "the last two args sit above the list"
        );
    }
}
