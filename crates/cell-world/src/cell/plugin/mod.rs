//! Feature plugins over `SpaceManager` (`docs/architecture/plugin-architecture.md`).
//!
//! A feature crate implements [`CellPlugin`] and, at startup, registers its
//! cell-method handlers and its hooks with a [`CellPluginBuilder`].
//! [`CellPlugins::build`] validates the registrations and freezes them; the
//! cell service installs the result on the `SpaceManager` before its loop
//! starts ([`SpaceManager::install_plugins`](crate::cell::space_manager::SpaceManager::install_plugins)).
//!
//! Core then fires the hooks without naming the feature:
//!
//! - the cell-method router asks [`CellPlugins::cell_method`] after the GM
//!   gate, before the static per-interface routers;
//! - the cell loop fires [`TickStage`] hooks at the positions the features'
//!   inline tick calls used to occupy;
//! - the base-message handlers fire [`EntityHookPoint`] hooks.
//!
//! A hook call site clones the registry's `Arc` first
//! (`let plugins = space_mgr.plugins().clone();`), so the registry never
//! aliases the `&mut SpaceManager` it hands to the handler.
//!
//! Handlers are `fn` pointers: a plugin keeps its state in the world (the
//! entity's `extensions` or a `SpaceManager` field), never in the plugin
//! value.
//!
//! # Missing registrations
//!
//! [`PLUGIN_OWNED_CELL_METHODS`] lists the cell-method indices that have left
//! the static routers. `CellPlugins::build` refuses a duplicate index, an
//! index that is not a client cell method and an index not on that list;
//! [`CellPlugins::check_complete`] reports an index on the list with no
//! handler. The orchestrator refuses to start on either; a bare
//! `CellService::start` logs the gap at WARN (target `cell.plugin`).

mod hook_points;
mod registry;

#[cfg(test)]
mod tests;

use std::future::Future;
use std::pin::Pin;

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub use hook_points::{EntityHookPoint, TickStage};
pub use registry::{CellPluginBuilder, CellPlugins, PluginError};

/// The cell-method indices whose handlers are registered by plugins rather
/// than the static routers in `cimmeria-cell-methods`. Grows as features
/// migrate; when every exposed index is here, the static routers are gone and
/// the startup check is "every client cell method has exactly one handler".
///
/// - 88-90: `petInvokeAbility`, `petAbilityToggle`, `petChangeStance`
///   (`cimmeria-cell-pets`).
pub const PLUGIN_OWNED_CELL_METHODS: &[u16] = &[
    cimmeria_wire::cell::cell_methods::player::constants::PET_INVOKE_ABILITY,
    cimmeria_wire::cell::cell_methods::player::constants::PET_ABILITY_TOGGLE,
    cimmeria_wire::cell::cell_methods::player::constants::PET_CHANGE_STANCE,
];

/// A boxed, `Send` future borrowing for `'a`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One decoded client cell-method call, as the router hands it to a plugin.
/// `method_index` is the flattened index (the `0x80` / `0xBD` decode and the
/// entity-id prefix are already stripped); `args` are the method's bytes.
pub struct CellMethodCall<'a> {
    pub entity_id: u32,
    pub method_index: u16,
    pub args: &'a [u8],
    pub tx: &'a mpsc::Sender<CellToBaseMsg>,
    pub space_mgr: &'a mut SpaceManager,
    pub engine: &'a ChainEngine,
}

/// A plugin's handler for one cell method.
pub type CellMethodHandler = for<'a> fn(CellMethodCall<'a>) -> BoxFuture<'a, ()>;

/// A hook fired once per cell-loop tick at its [`TickStage`].
pub type TickHook =
    for<'a> fn(&'a mpsc::Sender<CellToBaseMsg>, &'a mut SpaceManager) -> BoxFuture<'a, ()>;

/// A hook fired for one entity at its [`EntityHookPoint`].
pub type EntityHook =
    for<'a> fn(u32, &'a mpsc::Sender<CellToBaseMsg>, &'a mut SpaceManager) -> BoxFuture<'a, ()>;

/// A feature that registers with the cell at startup.
pub trait CellPlugin: Send + Sync + 'static {
    /// Stable name for logs and startup errors (`"pets"`).
    fn name(&self) -> &'static str;

    /// Register this plugin's handlers and hooks. Called once, at startup,
    /// in plugin-table order.
    fn build(&self, plugin: &mut CellPluginBuilder<'_>);
}

impl SpaceManager {
    /// The installed plugin registry. Clone it (one `Arc`) before firing a
    /// hook, so the registry does not borrow the `SpaceManager` the hook
    /// mutates.
    pub fn plugins(&self) -> &CellPlugins {
        &self.plugins
    }

    /// Install the plugin table. The cell service calls this once, before
    /// its loop starts; tests call it on the managers they build.
    pub fn install_plugins(&mut self, plugins: CellPlugins) {
        self.plugins = plugins;
    }
}
