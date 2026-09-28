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

#[cfg(test)]
mod tests {
    //! Registration against the Lua stack simulator: idempotent, repairs a
    //! lost or stale table, leaves the overlay alone, restores the stack.

    use super::*;
    use crate::deliver::fake_lua::{FakeLua, V};

    fn s(text: &str) -> V {
        V::Str(text.into())
    }

    fn registered_table(lua: &FakeLua) -> V {
        lua.global(NATIVE_TABLE).expect("the global is set")
    }

    #[test]
    fn registration_builds_the_table_once() {
        let mut lua = FakeLua::new();
        lua.stack.push(V::Str("game".into()));
        assert_eq!(
            ensure(&mut lua),
            Registration::Registered { replaced: false }
        );
        assert_eq!(lua.stack, [V::Str("game".into())], "stack restored");

        let table = registered_table(&lua);
        for native in Native::ALL {
            assert_eq!(
                lua.field(&table, native.lua_name()),
                Some(V::Func(format!("native:{}", native.lua_name())))
            );
        }
        assert_eq!(lua.field(&table, VERSION_KEY), Some(s(VERSION)));
        assert_eq!(
            lua.render(&table).matches("fn native:").count(),
            6,
            "exactly the six functions"
        );

        // Idempotent: the same table stays, so a reference the overlay holds
        // keeps working.
        assert_eq!(ensure(&mut lua), Registration::AlreadyPresent);
        assert_eq!(registered_table(&lua), table);
        assert_eq!(lua.stack, [V::Str("game".into())]);
    }

    #[test]
    fn registration_never_touches_the_overlay_table() {
        let mut lua = FakeLua::with_overlay(&["onOpen"]);
        let overlay = lua.global("CimmeriaBM");
        ensure(&mut lua);
        assert_eq!(lua.global("CimmeriaBM"), overlay);
        assert_eq!(lua.field(overlay.as_ref().unwrap(), "search"), None);
    }

    /// A UI reload that loses the global, a stale version, a missing function
    /// or a foreign value each get a fresh table.
    #[test]
    fn registration_repairs_a_missing_or_stale_table() {
        let mut lua = FakeLua::new();
        ensure(&mut lua);

        lua.set_global(NATIVE_TABLE, V::Nil);
        assert_eq!(
            ensure(&mut lua),
            Registration::Registered { replaced: false }
        );

        let table = registered_table(&lua);
        lua.set_field_of(&table, VERSION_KEY, s("0.0.0-old"));
        assert_eq!(
            ensure(&mut lua),
            Registration::Registered { replaced: true }
        );
        assert_ne!(registered_table(&lua), table);

        let table = registered_table(&lua);
        lua.set_field_of(&table, "bid", V::Nil);
        assert_eq!(
            ensure(&mut lua),
            Registration::Registered { replaced: true }
        );
        assert_eq!(
            lua.field(&registered_table(&lua), "bid"),
            Some(V::Func("native:bid".into()))
        );

        lua.set_global(NATIVE_TABLE, s("not a table"));
        assert_eq!(
            ensure(&mut lua),
            Registration::Registered { replaced: true }
        );
        assert_eq!(ensure(&mut lua), Registration::AlreadyPresent);
    }

    /// Out of Lua memory part-way, the registration reports it, the stack is
    /// restored, and the next attempt succeeds.
    #[test]
    fn registration_survives_running_out_of_memory() {
        let mut lua = FakeLua::new();
        lua.stack.push(V::Num(7));
        lua.alloc_budget = Some(5);
        assert!(matches!(
            ensure(&mut lua),
            Registration::SetupError { status: 4, .. }
        ));
        assert_eq!(lua.stack, [V::Num(7)]);
        assert_eq!(
            lua.global(NATIVE_TABLE),
            None,
            "nothing half-built is visible"
        );

        lua.alloc_budget = None;
        assert_eq!(
            ensure(&mut lua),
            Registration::Registered { replaced: false }
        );
    }

    #[test]
    fn registration_reports_a_full_stack() {
        let mut lua = FakeLua::new();
        lua.room = 1;
        assert_eq!(ensure(&mut lua), Registration::NoStackSpace);
        assert!(lua.stack.is_empty());
    }
}
