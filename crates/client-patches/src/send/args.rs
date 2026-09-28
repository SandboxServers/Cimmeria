//! Reading a send native's Lua arguments into a cell method call.
//!
//! The rules, which `crates/client-patches/README.md` states for the
//! overlay's authors:
//!
//! - **Numbers** must be Lua numbers holding an integer in the field's
//!   range: `INT32` fields take -2^31..2^31-1, `UINT8` fields 0..255. A
//!   string such as `"5"` is not coerced, and `2.5` is refused.
//! - **Strings** must be Lua strings. They are sent as UTF-8, at most 255
//!   bytes (the codec's cap).
//! - **`search(opts)`**: `opts` is a table, or `nil` for every default.
//!   A missing field is `0`, or `""` for the three names, except `quality`,
//!   which defaults to `2000` as in the client's own `BMSearchOptions`.
//!   `bForward` also takes a boolean. `clientKey` must be a
//!   `UIAuctionView`: `0`, `1` or `2`. Unknown keys are ignored. Fields are
//!   read with `lua_rawget`, so no metamethod runs.
//! - **`create`**: `buyoutPrice` may be `nil` for no buyout (`0`);
//!   `auctionLength` must be a `UIAuctionTime`, `1` to `5`. The wire order
//!   is the `.def` order (item, buyout, length, starting), whatever order
//!   the Lua call takes them in.
//! - **`watch(itemDefId, enable)`**: `enable` is Lua truthiness, so a
//!   missing `enable` means stop watching.
//!
//! A native is a C function, so its arguments sit at stack indices 1, 2, …
//! and Lua guarantees `LUA_MINSTACK` (20) free slots above them. Reading a
//! table field pushes two values and pops them before the next read.

use cimmeria_patch_wire::black_market::{
    BMCancelAuction, BMCreateAuction, BMPlaceBid, BMSearch, BMSearchOptions, BMStartWatchingItem,
    BMStopWatchingItem, CellCall, UIAuctionTime, UIAuctionView,
};

use super::Native;
use crate::deliver::lua_stack::{
    LuaStack, LUA_TBOOLEAN, LUA_TNIL, LUA_TNONE, LUA_TNUMBER, LUA_TSTRING, LUA_TTABLE,
};

/// The client's default `BMSearchOptions.quality` (constructor
/// `FUN_00adebc0`, evidence §4).
pub const DEFAULT_QUALITY: i32 = 2000;

/// Why an argument was refused, for the log.
type ArgResult<T> = Result<T, String>;

/// The call `native` makes with the arguments on `lua`. `native` must be
/// one of the five send natives; `techCompetency` sends nothing.
pub fn read<L: LuaStack>(lua: &mut L, native: Native) -> ArgResult<CellCall> {
    Ok(match native {
        Native::Search => CellCall::Search(BMSearch {
            search_options: search_options(lua, 1)?,
        }),
        Native::Create => {
            let item_instance_id = required_i32(lua, 1, "itemInstanceId")?;
            let starting_price = required_i32(lua, 2, "startingPrice")?;
            let buyout_price = optional_i32(lua, 3, "buyoutPrice")?.unwrap_or(0);
            let length = required_u8(lua, 4, "auctionLength")?;
            let auction_length = UIAuctionTime::try_from(length)
                .map_err(|_| format!("`auctionLength` {length} is not 1 to 5"))?
                as u8;
            CellCall::CreateAuction(BMCreateAuction {
                item_instance_id,
                buyout_price,
                auction_length,
                starting_price,
            })
        }
        Native::Bid => CellCall::PlaceBid(BMPlaceBid {
            sequence_id: required_i32(lua, 1, "sequenceId")?,
            bid_amount: required_i32(lua, 2, "bidAmount")?,
        }),
        Native::Cancel => CellCall::CancelAuction(BMCancelAuction {
            sequence_id: required_i32(lua, 1, "sequenceId")?,
        }),
        Native::Watch => {
            let item_def_id = required_i32(lua, 1, "itemDefId")?;
            if lua.to_boolean(2) {
                CellCall::StartWatchingItem(BMStartWatchingItem { item_def_id })
            } else {
                CellCall::StopWatchingItem(BMStopWatchingItem { item_def_id })
            }
        }
        Native::TechCompetency => return Err("techCompetency sends nothing".into()),
    })
}

/// `BMSearchOptions` from the table (or `nil`) at `index`.
fn search_options<L: LuaStack>(lua: &mut L, index: i32) -> ArgResult<BMSearchOptions> {
    let mut opts = BMSearchOptions {
        quality: DEFAULT_QUALITY,
        ..BMSearchOptions::default()
    };
    match lua.type_at(index) {
        LUA_TNONE | LUA_TNIL => return Ok(opts),
        LUA_TTABLE => {}
        other => {
            return Err(format!(
                "`opts` must be a table or nil, got {}",
                type_name(other)
            ))
        }
    }
    if let Some(v) = field(lua, index, "sortId", |l| optional_u8(l, -1, "sortId"))? {
        opts.sort_id = v;
    }
    if let Some(v) = field(lua, index, "clientKey", |l| {
        optional_i32(l, -1, "clientKey")
    })? {
        UIAuctionView::try_from(v).map_err(|_| format!("`clientKey` {v} is not 0, 1 or 2"))?;
        opts.client_key = v;
    }
    if let Some(v) = field(lua, index, "sequenceId", |l| {
        optional_i32(l, -1, "sequenceId")
    })? {
        opts.sequence_id = v;
    }
    if let Some(v) = field(lua, index, "bForward", |l| b_forward(l, -1))? {
        opts.b_forward = v;
    }
    if let Some(v) = field(lua, index, "sellerName", |l| {
        optional_string(l, -1, "sellerName")
    })? {
        opts.seller_name = v;
    }
    if let Some(v) = field(lua, index, "bidderName", |l| {
        optional_string(l, -1, "bidderName")
    })? {
        opts.bidder_name = v;
    }
    if let Some(v) = field(lua, index, "itemName", |l| {
        optional_string(l, -1, "itemName")
    })? {
        opts.item_name = v;
    }
    if let Some(v) = field(lua, index, "minTC", |l| optional_i32(l, -1, "minTC"))? {
        opts.min_tc = v;
    }
    if let Some(v) = field(lua, index, "maxTC", |l| optional_i32(l, -1, "maxTC"))? {
        opts.max_tc = v;
    }
    if let Some(v) = field(lua, index, "quality", |l| optional_i32(l, -1, "quality"))? {
        opts.quality = v;
    }
    if let Some(v) = field(lua, index, "filterFlags", |l| {
        optional_i32(l, -1, "filterFlags")
    })? {
        opts.filter_flags = v;
    }
    Ok(opts)
}

/// Push `table[key]` (raw), read it with `read` at index -1, and pop it.
fn field<L: LuaStack, T>(
    lua: &mut L,
    table: i32,
    key: &str,
    read: impl FnOnce(&mut L) -> ArgResult<T>,
) -> ArgResult<T> {
    lua.push_string(key);
    lua.raw_get(table);
    let value = read(lua);
    lua.set_top(-2);
    value
}

/// `bForward`: a `UINT8` number, or a boolean (`true` is 1).
fn b_forward<L: LuaStack>(lua: &mut L, index: i32) -> ArgResult<Option<u8>> {
    if lua.type_at(index) == LUA_TBOOLEAN {
        return Ok(Some(u8::from(lua.to_boolean(index))));
    }
    optional_u8(lua, index, "bForward")
}

/// An integer number at `index` within `min..=max`, or `None` if the value
/// is missing or `nil`.
fn optional_int<L: LuaStack>(
    lua: &mut L,
    index: i32,
    name: &str,
    min: i64,
    max: i64,
) -> ArgResult<Option<i64>> {
    match lua.type_at(index) {
        LUA_TNONE | LUA_TNIL => Ok(None),
        LUA_TNUMBER => {
            let n = lua.to_number(index);
            // NaN fails both comparisons, so it is refused here too.
            if n.fract() == 0.0 && n >= min as f64 && n <= max as f64 {
                Ok(Some(n as i64))
            } else {
                Err(format!("`{name}` {n} is not an integer in {min}..={max}"))
            }
        }
        other => Err(format!(
            "`{name}` must be a number, got {}",
            type_name(other)
        )),
    }
}

fn optional_i32<L: LuaStack>(lua: &mut L, index: i32, name: &str) -> ArgResult<Option<i32>> {
    Ok(optional_int(lua, index, name, i32::MIN.into(), i32::MAX.into())?.map(|n| n as i32))
}

fn optional_u8<L: LuaStack>(lua: &mut L, index: i32, name: &str) -> ArgResult<Option<u8>> {
    Ok(optional_int(lua, index, name, 0, u8::MAX.into())?.map(|n| n as u8))
}

fn required_i32<L: LuaStack>(lua: &mut L, index: i32, name: &str) -> ArgResult<i32> {
    optional_i32(lua, index, name)?.ok_or_else(|| format!("`{name}` is missing"))
}

fn required_u8<L: LuaStack>(lua: &mut L, index: i32, name: &str) -> ArgResult<u8> {
    optional_u8(lua, index, name)?.ok_or_else(|| format!("`{name}` is missing"))
}

/// A string at `index`, or `None` if the value is missing or `nil`.
fn optional_string<L: LuaStack>(lua: &mut L, index: i32, name: &str) -> ArgResult<Option<String>> {
    match lua.type_at(index) {
        LUA_TNONE | LUA_TNIL => Ok(None),
        LUA_TSTRING => lua
            .string_at(index)
            .map(Some)
            .ok_or_else(|| format!("`{name}` could not be read")),
        other => Err(format!(
            "`{name}` must be a string, got {}",
            type_name(other)
        )),
    }
}

/// `lua_typename`, for the log.
pub fn type_name(t: i32) -> &'static str {
    match t {
        LUA_TNONE => "no value",
        LUA_TNIL => "nil",
        LUA_TBOOLEAN => "boolean",
        2 => "userdata",
        LUA_TNUMBER => "number",
        LUA_TSTRING => "string",
        LUA_TTABLE => "table",
        6 => "function",
        7 => "userdata",
        8 => "thread",
        _ => "unknown",
    }
}
