//! A small Lua stack simulator for the delivery tests. It models only what
//! [`LuaStack`] exposes, with the same index rules as the C API, and
//! records every handler call.
//!
//! It also models where an allocation error can go. Every call that can
//! allocate on the Lua heap panics unless it runs inside
//! [`LuaStack::protected`], which is how the real client's Lua would reach
//! its panic function and exit. With [`FakeLua::alloc_budget`] set, the
//! allocation after the budget raises a Lua error, which unwinds to the
//! enclosing `protected` the way `lua_cpcall` catches it.

use std::collections::BTreeMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use super::lua_stack::{LuaStack, LUA_GLOBALSINDEX};

/// A simulated Lua value.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum V {
    Nil,
    Num(i32),
    Str(String),
    /// Index into [`FakeLua::tables`].
    Table(usize),
    /// A function, by name.
    Func(String),
}

#[derive(Debug, Default)]
pub(super) struct Table {
    fields: BTreeMap<String, V>,
    items: BTreeMap<i32, V>,
}

/// The panic payload that stands in for a raised Lua error.
struct LuaRaise;

/// `LUA_ERRMEM`.
const LUA_ERRMEM: i32 = 4;

pub(super) struct FakeLua {
    pub(super) stack: Vec<V>,
    tables: Vec<Table>,
    /// Every handler call made through `pcall`: its name and arguments.
    pub(super) calls: Vec<(String, Vec<V>)>,
    /// When set, every handler raises this value.
    pub(super) raise: Option<V>,
    /// Free slots `check_stack` will grant beyond the current top.
    pub(super) room: i32,
    /// How many more allocations succeed; `None` for no limit.
    pub(super) alloc_budget: Option<usize>,
    /// How many `protected` calls are running.
    protected_depth: u32,
}

impl FakeLua {
    /// An empty state: no `CimmeriaBM`.
    pub(super) fn new() -> Self {
        Self {
            stack: Vec::new(),
            tables: vec![Table::default()], // [0] is the globals table
            calls: Vec::new(),
            raise: None,
            room: 64,
            alloc_budget: None,
            protected_depth: 0,
        }
    }

    /// A state whose `CimmeriaBM` defines `handlers`.
    pub(super) fn with_overlay(handlers: &[&str]) -> Self {
        let mut lua = Self::new();
        let id = lua.new_table();
        for h in handlers {
            lua.tables[id]
                .fields
                .insert((*h).to_string(), V::Func((*h).to_string()));
        }
        lua.set_global("CimmeriaBM", V::Table(id));
        lua
    }

    pub(super) fn set_global(&mut self, name: &str, value: V) {
        self.tables[0].fields.insert(name.to_string(), value);
    }

    fn new_table(&mut self) -> usize {
        self.tables.push(Table::default());
        self.tables.len() - 1
    }

    /// Absolute stack slot for a C API index.
    fn slot(&self, index: i32) -> usize {
        if index > 0 {
            index as usize - 1
        } else {
            assert!(index < 0 && index > LUA_GLOBALSINDEX, "bad index {index}");
            (self.stack.len() as i32 + index) as usize
        }
    }

    fn table_id(&self, index: i32) -> usize {
        if index == LUA_GLOBALSINDEX {
            return 0;
        }
        match &self.stack[self.slot(index)] {
            V::Table(id) => *id,
            other => panic!("index {index} is {other:?}, not a table"),
        }
    }

    /// An API call that can allocate on the Lua heap.
    fn allocates(&mut self) {
        assert!(
            self.protected_depth > 0,
            "the Lua heap was touched outside a protected call"
        );
        match &mut self.alloc_budget {
            Some(0) => std::panic::panic_any(LuaRaise),
            Some(n) => *n -= 1,
            None => {}
        }
    }

    fn pop(&mut self) -> V {
        self.stack.pop().expect("stack underflow")
    }

    /// A deterministic rendering: records as `{k=v,...}` with sorted keys,
    /// arrays as `[a,b]`.
    pub(super) fn render(&self, value: &V) -> String {
        match value {
            V::Nil => "nil".into(),
            V::Num(n) => n.to_string(),
            V::Str(s) => format!("{s:?}"),
            V::Func(f) => format!("fn {f}"),
            V::Table(id) => {
                let t = &self.tables[*id];
                if t.fields.is_empty() && !t.items.is_empty() {
                    let items: Vec<String> = t.items.values().map(|v| self.render(v)).collect();
                    format!("[{}]", items.join(","))
                } else {
                    let fields: Vec<String> = t
                        .fields
                        .iter()
                        .map(|(k, v)| format!("{k}={}", self.render(v)))
                        .collect();
                    format!("{{{}}}", fields.join(","))
                }
            }
        }
    }

    /// Whether table `value` is an array with keys exactly `1..=n`.
    pub(super) fn is_one_based_array(&self, value: &V, n: i32) -> bool {
        let V::Table(id) = value else { return false };
        let keys: Vec<i32> = self.tables[*id].items.keys().copied().collect();
        keys == (1..=n).collect::<Vec<_>>()
    }
}

impl LuaStack for FakeLua {
    fn top(&mut self) -> i32 {
        self.stack.len() as i32
    }

    fn set_top(&mut self, index: i32) {
        let len = if index >= 0 {
            index as usize
        } else {
            (self.stack.len() as i32 + index + 1) as usize
        };
        self.stack.resize(len, V::Nil);
    }

    fn check_stack(&mut self, extra: i32) -> bool {
        self.allocates();
        extra <= self.room
    }

    fn type_at(&mut self, index: i32) -> i32 {
        match &self.stack[self.slot(index)] {
            V::Nil => 0,
            V::Num(_) => 3,
            V::Str(_) => 4,
            V::Table(_) => 5,
            V::Func(_) => 6,
        }
    }

    fn push_integer(&mut self, value: i32) {
        self.stack.push(V::Num(value));
    }

    fn push_string(&mut self, value: &str) {
        self.allocates();
        self.stack.push(V::Str(value.to_string()));
    }

    fn create_table(&mut self, _array: i32, _record: i32) {
        self.allocates();
        let id = self.new_table();
        self.stack.push(V::Table(id));
    }

    fn set_field(&mut self, table: i32, key: &str) {
        self.allocates();
        let id = self.table_id(table);
        let value = self.pop();
        self.tables[id].fields.insert(key.to_string(), value);
    }

    fn raw_set_index(&mut self, table: i32, n: i32) {
        self.allocates();
        let id = self.table_id(table);
        let value = self.pop();
        self.tables[id].items.insert(n, value);
    }

    fn raw_get(&mut self, table: i32) {
        let id = self.table_id(table);
        let key = self.pop();
        let value = match key {
            V::Str(k) => self.tables[id].fields.get(&k).cloned(),
            V::Num(n) => self.tables[id].items.get(&n).cloned(),
            _ => None,
        };
        self.stack.push(value.unwrap_or(V::Nil));
    }

    fn pcall(&mut self, args: i32, results: i32) -> i32 {
        assert_eq!(results, 0, "delivery wants no results");
        let at = self.stack.len() - args as usize;
        let call_args = self.stack.split_off(at);
        match self.pop() {
            V::Func(name) => {
                self.calls.push((name, call_args));
                match self.raise.clone() {
                    Some(err) => {
                        self.stack.push(err);
                        2
                    }
                    None => 0,
                }
            }
            other => {
                self.stack
                    .push(V::Str(format!("attempt to call {other:?}")));
                2
            }
        }
    }

    fn protected(&mut self, body: &mut dyn FnMut(&mut Self)) -> i32 {
        let base = self.stack.len();
        self.protected_depth += 1;
        let ran = catch_unwind(AssertUnwindSafe(|| body(self)));
        self.protected_depth -= 1;
        self.stack.truncate(base);
        match ran {
            Ok(()) => 0,
            Err(payload) if payload.is::<LuaRaise>() => {
                self.stack.push(V::Str("not enough memory".into()));
                LUA_ERRMEM
            }
            Err(payload) => resume_unwind(payload),
        }
    }

    fn string_at(&mut self, index: i32) -> Option<String> {
        match &self.stack[self.slot(index)] {
            V::Str(s) => Some(s.clone()),
            _ => None,
        }
    }
}
