//! Delivery against the stack simulator, and the whole receive-to-Lua path
//! on the host.

use cimmeria_patch_wire::black_market::{
    AuctionItem, ClientCall, OnBMAuctions, OnBMError, OnBMOpen,
};
use cimmeria_patch_wire::{Encode, SliceSource};

use super::fake_lua::{FakeLua, V};
use super::lua_stack::{deliver, Delivery};
use super::plan::plan;
use super::{drain, ui_lua_state};
use crate::addresses::{LUA_STATE_TT, SGW_UI_MANAGER, UI_MANAGER_LUA_SLOT};
use crate::counters::Counters;
use crate::memory::FakeMemory;
use crate::queue::EventQueue;
use std::sync::atomic::Ordering;

const ALL_HANDLERS: &[&str] = &[
    "onOpen",
    "onError",
    "onAuctions",
    "onAuctionRemove",
    "onAuctionUpdate",
    "onWatchedItems",
];

fn item(sequence_id: i32, seller: &str) -> AuctionItem {
    AuctionItem {
        sequence_id,
        item_def_id: 100 + sequence_id,
        stack_size: 1,
        durability: 50,
        charges: 0,
        current_bid: 10,
        buyout_price: 20,
        end_time_value: 3,
        next_min_bid_price: 11,
        seller_name: seller.into(),
    }
}

fn auctions() -> ClientCall {
    ClientCall::Auctions(OnBMAuctions {
        auction_items: vec![item(1, "Vala"), item(2, "Bra'tac")],
        total_results: 40,
        client_key: 1,
    })
}

/// Junk the game had on the stack before the frame; delivery must leave it.
fn with_junk(mut lua: FakeLua) -> FakeLua {
    lua.stack.push(V::Num(7));
    lua.stack.push(V::Str("game".into()));
    lua
}

fn assert_stack_restored(lua: &FakeLua) {
    assert_eq!(lua.stack, vec![V::Num(7), V::Str("game".into())]);
}

#[test]
fn on_auctions_reaches_the_handler_as_tables() {
    let mut lua = with_junk(FakeLua::with_overlay(ALL_HANDLERS));
    assert_eq!(deliver(&mut lua, &plan(&auctions())), Delivery::Delivered);
    assert_stack_restored(&lua);

    assert_eq!(lua.calls.len(), 1);
    let (name, args) = &lua.calls[0];
    assert_eq!(name, "onAuctions");
    assert_eq!(args.len(), 3, "(items, totalResults, clientKey)");
    assert!(lua.is_one_based_array(&args[0], 2), "items is 1-based");
    assert_eq!(
        lua.render(&args[0]),
        "[{buyoutPrice=20,charges=0,currentBid=10,durability=50,endTimeValue=3,\
         itemDefId=101,nextMinBidPrice=11,sellerName=\"Vala\",sequenceId=1,stackSize=1},\
         {buyoutPrice=20,charges=0,currentBid=10,durability=50,endTimeValue=3,\
         itemDefId=102,nextMinBidPrice=11,sellerName=\"Bra'tac\",sequenceId=2,stackSize=1}]"
    );
    assert_eq!(args[1], V::Num(40));
    assert_eq!(args[2], V::Num(1));
}

#[test]
fn simple_calls_pass_one_number() {
    let mut lua = FakeLua::with_overlay(ALL_HANDLERS);
    deliver(
        &mut lua,
        &plan(&ClientCall::Open(OnBMOpen { entity_id: 99 })),
    );
    deliver(
        &mut lua,
        &plan(&ClientCall::Error(OnBMError { error_id: 1 })),
    );
    assert_eq!(
        lua.calls,
        vec![
            ("onOpen".to_string(), vec![V::Num(99)]),
            ("onError".to_string(), vec![V::Num(1)]),
        ]
    );
    assert!(lua.stack.is_empty());
}

/// Without the overlay the call is dropped, nothing runs, and the stack is
/// as it was.
#[test]
fn missing_overlay_is_reported_and_harmless() {
    let mut lua = with_junk(FakeLua::new());
    assert_eq!(deliver(&mut lua, &plan(&auctions())), Delivery::NoOverlay);
    assert!(lua.calls.is_empty());
    assert_stack_restored(&lua);

    let mut lua = with_junk(FakeLua::new());
    lua.set_global("CimmeriaBM", V::Str("not a table".into()));
    assert_eq!(deliver(&mut lua, &plan(&auctions())), Delivery::NoOverlay);
    assert_stack_restored(&lua);
}

#[test]
fn missing_handler_is_reported_and_harmless() {
    let mut lua = with_junk(FakeLua::with_overlay(&["onOpen"]));
    assert_eq!(deliver(&mut lua, &plan(&auctions())), Delivery::NoHandler);
    assert!(lua.calls.is_empty());
    assert_stack_restored(&lua);
}

/// A handler error is caught by `lua_pcall`, its message is kept, and the
/// error value does not stay on the stack.
#[test]
fn handler_errors_are_caught_and_the_stack_restored() {
    let mut lua = with_junk(FakeLua::with_overlay(ALL_HANDLERS));
    lua.raise = Some(V::Str("BlackMarket.lua:12: attempt to index nil".into()));
    assert_eq!(
        deliver(&mut lua, &plan(&auctions())),
        Delivery::HandlerError {
            status: 2,
            message: "BlackMarket.lua:12: attempt to index nil".into()
        }
    );
    assert_stack_restored(&lua);

    let mut lua = with_junk(FakeLua::with_overlay(ALL_HANDLERS));
    lua.raise = Some(V::Num(5));
    assert!(matches!(
        deliver(&mut lua, &plan(&auctions())),
        Delivery::HandlerError { status: 2, .. }
    ));
    assert_stack_restored(&lua);
}

#[test]
fn no_stack_space_pushes_nothing() {
    let mut lua = with_junk(FakeLua::with_overlay(ALL_HANDLERS));
    lua.room = 2;
    assert_eq!(
        deliver(&mut lua, &plan(&auctions())),
        Delivery::NoStackSpace
    );
    assert!(lua.calls.is_empty());
    assert_stack_restored(&lua);
}

#[test]
fn drain_respects_the_frame_budget() {
    let queue = EventQueue::new(64);
    for i in 0..20 {
        queue
            .push(ClientCall::Open(OnBMOpen { entity_id: i }))
            .unwrap();
    }
    let counters = Counters::new();
    let mut lua = FakeLua::with_overlay(ALL_HANDLERS);
    assert_eq!(drain(&queue, &counters, &mut lua, 16), 16);
    assert_eq!(queue.len(), 4, "the rest wait for the next frame");
    assert_eq!(counters.delivered.load(Ordering::Relaxed), 16);
    assert_eq!(lua.calls[0].1, vec![V::Num(0)], "oldest first");
    assert_eq!(drain(&queue, &counters, &mut lua, 16), 4);
    assert!(queue.is_empty());
}

/// Once the Lua state is up but the overlay is not, calls are dropped and
/// counted rather than piling up.
#[test]
fn drain_without_overlay_drops_and_counts() {
    let queue = EventQueue::new(8);
    queue.push(auctions()).unwrap();
    queue.push(auctions()).unwrap();
    let counters = Counters::new();
    let mut lua = FakeLua::new();
    assert_eq!(drain(&queue, &counters, &mut lua, 16), 2);
    assert!(queue.is_empty());
    assert_eq!(counters.dropped_no_overlay.load(Ordering::Relaxed), 2);
    assert_eq!(counters.delivered.load(Ordering::Relaxed), 0);
}

// ── the UI lua_State ─────────────────────────────────────────────────────

const MANAGER: usize = 0x3000_0000;
const SLOT: usize = 0x3100_0000;
const STATE: usize = 0x3200_0000;

fn ui_chain(tt_dword: [u8; 4]) -> FakeMemory {
    let mut mem = FakeMemory::default();
    mem.put_u32(SGW_UI_MANAGER, MANAGER as u32)
        .put_u32(MANAGER + UI_MANAGER_LUA_SLOT, SLOT as u32)
        .put_u32(SLOT, STATE as u32)
        .put(STATE + LUA_STATE_TT, &tt_dword);
    mem
}

/// The type tag is the low byte. The byte after it holds GC mark bits, so
/// a dword compare with 8 would refuse a live state.
#[test]
fn ui_lua_state_checks_the_tag_byte_not_the_dword() {
    assert_eq!(ui_lua_state(&ui_chain([0x08, 0x00, 0, 0])), Some(STATE));
    assert_eq!(ui_lua_state(&ui_chain([0x08, 0x05, 0, 0])), Some(STATE));
    assert_eq!(ui_lua_state(&ui_chain([0x05, 0x00, 0, 0])), None);
}

#[test]
fn ui_lua_state_is_none_until_every_hop_is_set() {
    assert_eq!(ui_lua_state(&FakeMemory::default()), None);
    let mut mem = FakeMemory::default();
    mem.put_u32(SGW_UI_MANAGER, MANAGER as u32)
        .put_u32(MANAGER + UI_MANAGER_LUA_SLOT, 0);
    assert_eq!(ui_lua_state(&mem), None, "slot pointer still null");
    let mut mem = FakeMemory::default();
    mem.put_u32(SGW_UI_MANAGER, MANAGER as u32)
        .put_u32(MANAGER + UI_MANAGER_LUA_SLOT, SLOT as u32)
        .put_u32(SLOT, 0);
    assert_eq!(ui_lua_state(&mem), None, "lua_State not created yet");
}

// ── end to end ───────────────────────────────────────────────────────────

/// Server bytes for `onBMAuctions`, claimed from a fake `MethodDescription`
/// on the "network thread", queued, and drained into the overlay: the
/// handler sees what the server sent.
#[test]
fn server_bytes_reach_the_overlay() {
    use crate::addresses::{
        ENTITY_ID, GAME_ENTITY_MANAGER, GEM_LOCAL_PLAYER_ID, METHOD_NAME_BUFFER,
        METHOD_NAME_CAPACITY, METHOD_NAME_LEN,
    };
    use crate::receive::claim::{claim, Claim};

    const MD: usize = 0x4000_0000;
    const GEM: usize = 0x4100_0000;
    const PLAYER: usize = 0x4200_0000;
    let mut mem = FakeMemory::default();
    let mut name = [0u8; 16];
    name[..12].copy_from_slice(b"onBMAuctions");
    mem.put(MD + METHOD_NAME_BUFFER, &name)
        .put_u32(MD + METHOD_NAME_LEN, 12)
        .put_u32(MD + METHOD_NAME_CAPACITY, 15)
        .put_u32(GAME_ENTITY_MANAGER, GEM as u32)
        .put_u32(GEM + GEM_LOCAL_PLAYER_ID, 77)
        .put_u32(PLAYER + ENTITY_ID, 77);

    let wire = auctions().to_bytes().unwrap();
    let Claim::Decoded(call) = claim(&mem, MD, PLAYER, || Some(SliceSource::new(&wire))) else {
        panic!("onBMAuctions for the local player must be claimed");
    };
    let queue = EventQueue::new(4);
    queue.push(call).unwrap();

    let counters = Counters::new();
    let mut lua = FakeLua::with_overlay(ALL_HANDLERS);
    drain(&queue, &counters, &mut lua, 16);
    let (name, args) = &lua.calls[0];
    assert_eq!(name, "onAuctions");
    assert!(lua.render(&args[0]).contains("sellerName=\"Bra'tac\""));
    assert_eq!((&args[1], &args[2]), (&V::Num(40), &V::Num(1)));
}
