//! Putting [`NATIVE_TABLE`] into the UI Lua, once per `lua_State`.
//!
//! The `FEngineLoop::Tick` detour calls [`ensure`] every few frames once
//! the UI `lua_State` is up. It is idempotent: when the global already holds
//! a table with this DLL's `version` and all six functions, nothing
//! changes. Otherwise (first start, a UI reload that built a fresh state or
//! cleared the global, or a table something else overwrote) a new table is
//! built and assigned. So the overlay sees the table within a few frames of
//! the UI coming up, and should look it up when it needs it, not once when
//! its file loads.
//!
//! Like delivery, everything runs inside `lua_cpcall`, the global is read
//! and written raw (`lua_rawget`/`lua_rawset`, so no metamethod on the
//! globals table runs), and the stack is restored whatever happens.

use super::{Native, NATIVE_TABLE, VERSION};
use crate::deliver::lua_stack::{
    LuaStack, LUA_GLOBALSINDEX, LUA_TFUNCTION, LUA_TNIL, LUA_TSTRING, LUA_TTABLE,
};

/// The key of the version string in [`NATIVE_TABLE`].
pub const VERSION_KEY: &str = "version";

/// Stack slots [`ensure`] needs: the global, a key, and a value.
const SLOTS: i32 = 4;

/// What [`ensure`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Registration {
    /// The table was already there and current.
    AlreadyPresent,
    /// A new table was assigned. `replaced` is true when the global held
    /// something else before (a stale table or another value).
    Registered {
        /// Whether a non-nil value was replaced.
        replaced: bool,
    },
    /// The Lua stack could not grow.
    NoStackSpace,
    /// A Lua error (in practice, out of memory) stopped the registration.
    SetupError {
        /// The `lua_cpcall` status.
        status: i32,
        /// The error value, if it was a string.
        message: String,
    },
}

/// Make sure [`NATIVE_TABLE`] is current in `lua`, and leave the stack as it
/// was.
pub fn ensure<L: LuaStack>(lua: &mut L) -> Registration {
    let top = lua.top();
    let mut outcome = None;
    let status = lua.protected(&mut |lua| outcome = Some(ensure_protected(lua)));
    let outcome = match outcome {
        Some(outcome) if status == 0 => outcome,
        _ => {
            let message = if lua.type_at(-1) == LUA_TSTRING {
                lua.string_at(-1).unwrap_or_default()
            } else {
                String::from("(error value is not a string)")
            };
            Registration::SetupError { status, message }
        }
    };
    lua.set_top(top);
    outcome
}

fn ensure_protected<L: LuaStack>(lua: &mut L) -> Registration {
    if !lua.check_stack(SLOTS) {
        return Registration::NoStackSpace;
    }
    lua.push_string(NATIVE_TABLE);
    lua.raw_get(LUA_GLOBALSINDEX);
    let existing = lua.type_at(-1);
    if existing == LUA_TTABLE && is_current(lua) {
        return Registration::AlreadyPresent;
    }
    lua.create_table(0, Native::ALL.len() as i32 + 1);
    for native in Native::ALL {
        lua.push_native(native);
        lua.set_field(-2, native.lua_name());
    }
    lua.push_string(VERSION);
    lua.set_field(-2, VERSION_KEY);
    lua.push_string(NATIVE_TABLE);
    lua.push_value(-2);
    lua.raw_set(LUA_GLOBALSINDEX);
    Registration::Registered {
        replaced: existing != LUA_TNIL,
    }
}

/// Whether the table on top holds this DLL's version and every function.
fn is_current<L: LuaStack>(lua: &mut L) -> bool {
    lua.push_string(VERSION_KEY);
    lua.raw_get(-2);
    let version_matches =
        lua.type_at(-1) == LUA_TSTRING && lua.string_at(-1).as_deref() == Some(VERSION);
    lua.set_top(-2);
    if !version_matches {
        return false;
    }
    Native::ALL.into_iter().all(|native| {
        lua.push_string(native.lua_name());
        lua.raw_get(-2);
        let is_function = lua.type_at(-1) == LUA_TFUNCTION;
        lua.set_top(-2);
        is_function
    })
}
