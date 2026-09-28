//! Running a [`LuaCall`] against a Lua stack.
//!
//! [`LuaStack`] is the handful of Lua 5.1 C API calls delivery needs. In
//! `SGW.exe` it is the client's `lua51.dll`; in tests it is a small stack
//! simulator, so the stack discipline is tested on the host:
//!
//! - Everything that touches the Lua heap runs inside `lua_cpcall`: the
//!   lookups, the argument tables and strings, and the handler call. Those
//!   API calls can raise an allocation error, and outside a protected call
//!   Lua has nothing to unwind to, so it calls its panic function and the
//!   client exits. Inside, the error ends the `lua_cpcall` and the call is
//!   reported as [`Delivery::SetupError`].
//! - The overlay table and its function are looked up with `lua_rawget`,
//!   so no metamethod runs.
//! - The handler runs under a nested `lua_pcall`, never `lua_call`, so its
//!   own errors are told apart from a failure to set the call up.
//! - The stack top is restored whatever happens: delivered, overlay
//!   missing, handler missing, the handler raised an error, or setting up
//!   the call raised one.

use super::plan::{call_slots, LuaCall, LuaValue, TABLE};
use crate::send::Native;

/// `LUA_GLOBALSINDEX`: the stock Lua 5.1 value, which the client uses too.
/// The client's tolua++ `tolua_beginmodule`/`tolua_module` (`0x00403bb0`,
/// `0x00403bf0`) push the globals with `lua_pushvalue(L, -0x2712)`, that is
/// -10002. The `-10000` in its other tolua++ helpers (`0x00402a20`,
/// `0x004035a0`) is `LUA_REGISTRYINDEX`: they read `tolua_ubox`,
/// `tolua_super` and `luaL_getmetatable` names, which tolua++ keeps in the
/// registry.
pub const LUA_GLOBALSINDEX: i32 = -10002;

/// `LUA_TNONE`: an index past the top of the stack, such as a missing
/// argument.
pub const LUA_TNONE: i32 = -1;

/// `LUA_TNIL`.
pub const LUA_TNIL: i32 = 0;

/// `LUA_TBOOLEAN`.
pub const LUA_TBOOLEAN: i32 = 1;

/// `LUA_TNUMBER`.
pub const LUA_TNUMBER: i32 = 3;

/// `LUA_TSTRING`.
pub const LUA_TSTRING: i32 = 4;

/// `LUA_TTABLE`.
pub const LUA_TTABLE: i32 = 5;

/// `LUA_TFUNCTION`.
pub const LUA_TFUNCTION: i32 = 6;

/// The Lua 5.1 C API calls the DLL uses: delivery, and the send natives
/// with their registration ([`crate::send`]). Indices follow the C API.
pub trait LuaStack {
    /// `lua_gettop`.
    fn top(&mut self) -> i32;
    /// `lua_settop`.
    fn set_top(&mut self, index: i32);
    /// `lua_checkstack`: whether `extra` more slots are available.
    fn check_stack(&mut self, extra: i32) -> bool;
    /// `lua_type`.
    fn type_at(&mut self, index: i32) -> i32;
    /// `lua_pushinteger`.
    fn push_integer(&mut self, value: i32);
    /// `lua_pushstring`.
    fn push_string(&mut self, value: &str);
    /// `lua_createtable`.
    fn create_table(&mut self, array: i32, record: i32);
    /// `lua_setfield`: `t[key] = top`, where `t` is at `table`; pops the value.
    fn set_field(&mut self, table: i32, key: &str);
    /// `lua_rawseti`: `t[n] = top`, where `t` is at `table`; pops the value.
    fn raw_set_index(&mut self, table: i32, n: i32);
    /// `lua_rawget`: replaces the key on top with `t[key]`, where `t` is at
    /// `table`, without metamethods.
    fn raw_get(&mut self, table: i32);
    /// `lua_pcall` with no message handler; returns the status.
    fn pcall(&mut self, args: i32, results: i32) -> i32;
    /// `lua_cpcall`: run `body` against this state in protected mode and
    /// return the status. An error raised by an API call inside `body`
    /// unwinds out of `body` to here, so `body` does not finish; the error
    /// value is then on top of the stack.
    fn protected(&mut self, body: &mut dyn FnMut(&mut Self)) -> i32;
    /// The string at `index`, if it is one (`lua_tolstring` on a string).
    fn string_at(&mut self, index: i32) -> Option<String>;
    /// `lua_pushnil`.
    fn push_nil(&mut self);
    /// `lua_pushboolean`.
    fn push_boolean(&mut self, value: bool);
    /// `lua_tonumber`: the number at `index`, 0 if it is not one.
    fn to_number(&mut self, index: i32) -> f64;
    /// `lua_toboolean`: Lua truthiness, so `false` only for `nil`,
    /// `false` and a missing value.
    fn to_boolean(&mut self, index: i32) -> bool;
    /// `lua_pushvalue`: a copy of the value at `index`.
    fn push_value(&mut self, index: i32);
    /// `lua_rawset`: `t[key] = value`, where `t` is at `table` and the key
    /// and value are the top two values; pops both, without metamethods.
    fn raw_set(&mut self, table: i32);
    /// `lua_pushcclosure` with no upvalues, for one of the send natives.
    fn push_native(&mut self, native: Native);
}

/// How one call into the overlay went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    /// The handler ran and returned.
    Delivered,
    /// The global `CimmeriaBM` is not a table: the overlay is not
    /// installed.
    NoOverlay,
    /// `CimmeriaBM` has no function under the handler's name.
    NoHandler,
    /// The Lua stack could not grow enough for the arguments.
    NoStackSpace,
    /// The handler raised an error.
    HandlerError {
        /// The `lua_pcall` status.
        status: i32,
        /// The error value, if it was a string.
        message: String,
    },
    /// Setting the call up raised an error before the handler ran: in
    /// practice, Lua ran out of memory building the arguments.
    SetupError {
        /// The `lua_cpcall` status.
        status: i32,
        /// The error value, if it was a string.
        message: String,
    },
}

/// Make `call` and put the stack back as it was.
pub fn deliver<L: LuaStack>(lua: &mut L, call: &LuaCall) -> Delivery {
    let top = lua.top();
    let mut outcome = None;
    let status = lua.protected(&mut |lua| outcome = Some(call_handler(lua, call)));
    let outcome = match outcome {
        Some(outcome) if status == 0 => outcome,
        _ => Delivery::SetupError {
            status,
            message: error_message(lua),
        },
    };
    lua.set_top(top);
    outcome
}

/// The error value on top of the stack, as a string.
fn error_message<L: LuaStack>(lua: &mut L) -> String {
    if lua.type_at(-1) == LUA_TSTRING {
        lua.string_at(-1).unwrap_or_default()
    } else {
        String::from("(error value is not a string)")
    }
}

fn call_handler<L: LuaStack>(lua: &mut L, call: &LuaCall) -> Delivery {
    if !lua.check_stack(call_slots(call)) {
        return Delivery::NoStackSpace;
    }
    lua.push_string(TABLE);
    lua.raw_get(LUA_GLOBALSINDEX);
    if lua.type_at(-1) != LUA_TTABLE {
        return Delivery::NoOverlay;
    }
    lua.push_string(call.function);
    lua.raw_get(-2);
    if lua.type_at(-1) != LUA_TFUNCTION {
        return Delivery::NoHandler;
    }
    for arg in &call.args {
        push(lua, arg);
    }
    let status = lua.pcall(call.args.len() as i32, 0);
    if status == 0 {
        return Delivery::Delivered;
    }
    let message = error_message(lua);
    Delivery::HandlerError { status, message }
}

fn push<L: LuaStack>(lua: &mut L, value: &LuaValue) {
    match value {
        LuaValue::Int(v) => lua.push_integer(*v),
        LuaValue::Str(s) => lua.push_string(s),
        LuaValue::Record(fields) => {
            lua.create_table(0, fields.len() as i32);
            for (key, v) in fields {
                push(lua, v);
                lua.set_field(-2, key);
            }
        }
        LuaValue::List(items) => {
            lua.create_table(items.len() as i32, 0);
            for (i, v) in items.iter().enumerate() {
                push(lua, v);
                lua.raw_set_index(-2, i as i32 + 1);
            }
        }
    }
}
