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

use super::lua_stack::{LuaStack, LUA_GLOBALSINDEX, LUA_TNONE};
use crate::send::Native;

/// A simulated Lua value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum V {
    Nil,
    Bool(bool),
    Num(i32),
    /// A number that is not an integer, as a script can pass one.
    Float(f64),
    Str(String),
    /// Index into [`FakeLua::tables`].
    Table(usize),
    /// A function, by name.
    Func(String),
}

#[derive(Debug, Default)]
pub(crate) struct Table {
    fields: BTreeMap<String, V>,
    items: BTreeMap<i32, V>,
}

/// The panic payload that stands in for a raised Lua error.
struct LuaRaise;

/// `LUA_ERRMEM`.
const LUA_ERRMEM: i32 = 4;

pub(crate) struct FakeLua {
    pub(crate) stack: Vec<V>,
    tables: Vec<Table>,
    /// Every handler call made through `pcall`: its name and arguments.
    pub(crate) calls: Vec<(String, Vec<V>)>,
    /// When set, every handler raises this value.
    pub(crate) raise: Option<V>,
    /// Free slots `check_stack` will grant beyond the current top.
    pub(crate) room: i32,
    /// How many more allocations succeed; `None` for no limit.
    pub(crate) alloc_budget: Option<usize>,
    /// How many `protected` calls are running.
    protected_depth: u32,
}

impl FakeLua {
    /// An empty state: no `CimmeriaBM`.
    pub(crate) fn new() -> Self {
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
    pub(crate) fn with_overlay(handlers: &[&str]) -> Self {
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

    /// Global `name`, if set.
    pub(crate) fn global(&self, name: &str) -> Option<V> {
        self.tables[0].fields.get(name).cloned()
    }

    /// Field `key` of the table `value`, if it is a table and has one.
    pub(crate) fn field(&self, value: &V, key: &str) -> Option<V> {
        let V::Table(id) = value else { return None };
        self.tables[*id].fields.get(key).cloned()
    }

    /// Replace field `key` of the table `value`.
    pub(crate) fn set_field_of(&mut self, value: &V, key: &str, field: V) {
        let V::Table(id) = value else {
            panic!("{value:?} is not a table")
        };
        self.tables[*id].fields.insert(key.to_string(), field);
    }

    /// A new table with these string fields, not yet on the stack.
    pub(crate) fn table(&mut self, fields: &[(&str, V)]) -> V {
        let id = self.new_table();
        for (k, v) in fields {
            self.tables[id].fields.insert((*k).to_string(), v.clone());
        }
        V::Table(id)
    }

    /// A state as a native function sees it when a script calls it: the
    /// arguments are the whole stack, and the call runs under the script's
    /// own protected call, so the Lua heap may be touched.
    pub(crate) fn called_with(args: Vec<V>) -> Self {
        let mut lua = Self::new();
        lua.stack = args;
        lua.protected_depth = 1;
        lua
    }

    pub(crate) fn set_global(&mut self, name: &str, value: V) {
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
    pub(crate) fn render(&self, value: &V) -> String {
        match value {
            V::Nil => "nil".into(),
            V::Bool(b) => b.to_string(),
            V::Float(f) => f.to_string(),
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
    pub(crate) fn is_one_based_array(&self, value: &V, n: i32) -> bool {
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
        if index > self.stack.len() as i32 {
            return LUA_TNONE;
        }
        match &self.stack[self.slot(index)] {
            V::Nil => 0,
            V::Bool(_) => 1,
            V::Num(_) | V::Float(_) => 3,
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

    fn push_nil(&mut self) {
        self.stack.push(V::Nil);
    }

    fn push_boolean(&mut self, value: bool) {
        self.stack.push(V::Bool(value));
    }

    fn to_number(&mut self, index: i32) -> f64 {
        match &self.stack[self.slot(index)] {
            V::Num(n) => f64::from(*n),
            V::Float(f) => *f,
            _ => 0.0,
        }
    }

    fn to_boolean(&mut self, index: i32) -> bool {
        if index > self.stack.len() as i32 {
            return false;
        }
        !matches!(self.stack[self.slot(index)], V::Nil | V::Bool(false))
    }

    fn push_value(&mut self, index: i32) {
        let value = self.stack[self.slot(index)].clone();
        self.stack.push(value);
    }

    fn raw_set(&mut self, table: i32) {
        self.allocates();
        let id = self.table_id(table);
        let value = self.pop();
        let key = self.pop();
        match key {
            V::Str(k) => {
                self.tables[id].fields.insert(k, value);
            }
            V::Num(n) => {
                self.tables[id].items.insert(n, value);
            }
            other => panic!("unsupported key {other:?}"),
        }
    }

    fn push_native(&mut self, native: Native) {
        self.allocates();
        self.stack
            .push(V::Func(format!("native:{}", native.lua_name())));
    }
}
