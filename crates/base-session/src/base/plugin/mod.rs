//! Feature plugins on the base track (#962 step 5,
//! `docs/architecture/plugin-architecture.md` §3.1 and §4.5).
//!
//! The base-track twin of `cimmeria-cell-world`'s `cell::plugin`. A feature
//! crate implements [`BasePlugin`] and, at startup, registers with a
//! [`BasePluginBuilder`]:
//!
//! - handlers for SGWPlayer exposed **base** methods, by flattened index
//!   (the message id minus `0xC0`; the cell-method index space is separate,
//!   ADR §2 C3);
//! - consumers for the feature payloads cell code sends in the
//!   `CellToBaseMsg::Plugin` envelope ([`PluginMsg`], ADR §3.4);
//! - session lifecycle hooks ([`SessionHookPoint`],
//!   [`SessionStateHookPoint`], [`WorldEntryHookPoint`]).
//!
//! [`BasePlugins::build`] validates the registrations and freezes them. The
//! composition root (`cimmeria-services`) builds the table, the orchestrator
//! hands it to the `BaseService`, and the service stamps it on every
//! session it admits (`ConnectedClientState::plugins`) and hands it to the
//! cell-message loop. The base has no hub struct that every handler holds,
//! as the cell has `SpaceManager`, but nearly every handler holds the
//! connected map and the session's address, so the session carries the
//! registry ([`session_plugins`]).
//!
//! Per-session feature state lives in `ConnectedClientState::extensions`
//! ([`SessionExtensions`], the cell's `EntityExtensions` type, ADR §3.5).
//!
//! # Missing registrations
//!
//! [`PLUGIN_OWNED_BASE_METHODS`] lists the base-method indices that have
//! left the static router in `cimmeria-base`'s `dispatch`, and
//! [`PLUGIN_CELL_MESSAGES`] the payload types that travel in the envelope.
//! `BasePlugins::build` refuses a duplicate, unknown or not-plugin-owned
//! index, and a second or undeclared consumer; [`BasePlugins::check_complete`]
//! reports an owned index with no handler and a declared payload with no
//! consumer. The orchestrator refuses to start on either; a bare
//! `BaseService::start` logs the gap at WARN (target `base.plugin`). At run
//! time an envelope nobody consumes is dropped with a WARN
//! (`reason = "no_consumer"`).
//!
//! Crafting is the first user (`cimmeria-base-crafting`, #962 step 5): its
//! seven cell-to-base payloads are the whole envelope list, and it registers
//! no base method (its verbs 95-100 are cell methods), so the base-method
//! list is still empty.
//!
//! Three hook kinds are seams the inventory and progression code in
//! `cimmeria-base-methods` fire, so crafting could leave the session layer
//! without those callers naming it: [`ItemUseHookPoint`] (a hook that returns
//! what it did with a `useItem`), [`InventoryHookPoint`] (the inventory
//! resync's rows) and [`ProgressionHookPoint`] (the applied-science total an
//! XP grant earned).

mod hook_points;
mod registry;

#[cfg(test)]
mod seam_tests;
#[cfg(test)]
mod tests;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

pub use crate::cell::messages::PluginMsg;
pub use hook_points::{
    InventoryHookPoint, ItemUseHookPoint, ProgressionHookPoint, SessionHookPoint,
    SessionStateHookPoint, WorldEntryHookPoint,
};
pub use registry::{BasePluginBuilder, BasePluginError, BasePlugins};

/// Per-session feature state, keyed by type
/// (`ConnectedClientState::extensions`). The same storage as a cell
/// entity's extensions: one slot per type, never replicated or persisted on
/// its own.
pub type SessionExtensions = cimmeria_entity::cell_entity::EntityExtensions;

/// The SGWPlayer base-method indices (flattened: the message id minus
/// `0xC0`) whose handlers plugins register instead of the static router in
/// `cimmeria-base`'s `dispatch::dispatch_sgw_player_base_method`. Ascending.
///
/// Empty: no base feature with a base method has moved yet. Crafting, the
/// first base plugin, has none (its verbs 95-100 are cell methods).
pub const PLUGIN_OWNED_BASE_METHODS: &[u8] = &[];

/// The payload types that travel in the `CellToBaseMsg::Plugin` envelope.
/// Each must have exactly one consumer.
///
/// - Crafting (`cimmeria-base-crafting`, #962 step 5): the verbs 95-100,
///   the station reports, `.allcraft`, `.craftkit` / `.learnblueprint`,
///   `.respeccraft`, `gmGiveExpertise` and `gmGiveAppliedSciencePoints`.
pub const PLUGIN_CELL_MESSAGES: &[PluginMsgKind] = &[
    PluginMsgKind::of::<cimmeria_wire::crafting::CraftRequest>(),
    PluginMsgKind::of::<cimmeria_wire::crafting::CraftingStations>(),
    PluginMsgKind::of::<cimmeria_wire::crafting::GmAllCraft>(),
    PluginMsgKind::of::<cimmeria_wire::crafting::GmCraftGrant>(),
    PluginMsgKind::of::<cimmeria_wire::crafting::RespecCraftOpen>(),
    PluginMsgKind::of::<cimmeria_wire::crafting::GmGrantExpertise>(),
    PluginMsgKind::of::<cimmeria_wire::crafting::GmGrantAppliedSciencePoints>(),
];

/// A payload type the envelope carries, for the startup checks.
#[derive(Clone, Copy)]
pub struct PluginMsgKind {
    type_id: fn() -> TypeId,
    type_name: fn() -> &'static str,
}

impl PluginMsgKind {
    /// The kind of payload type `T`.
    pub const fn of<T: Any>() -> Self {
        Self {
            type_id: TypeId::of::<T>,
            type_name: std::any::type_name::<T>,
        }
    }

    /// The payload type.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// The payload type's name, for errors and logs.
    pub fn type_name(&self) -> &'static str {
        (self.type_name)()
    }
}

impl std::fmt::Debug for PluginMsgKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.type_name())
    }
}

/// What the registry validates against: the plugin-owned base methods and
/// the envelope's payload types. [`PluginOwnership::CURRENT`] is the
/// production pair; tests pass their own to [`BasePlugins::build_with`].
#[derive(Clone, Copy, Debug)]
pub struct PluginOwnership {
    pub base_methods: &'static [u8],
    pub cell_messages: &'static [PluginMsgKind],
}

impl PluginOwnership {
    /// [`PLUGIN_OWNED_BASE_METHODS`] and [`PLUGIN_CELL_MESSAGES`].
    pub const CURRENT: Self = Self {
        base_methods: PLUGIN_OWNED_BASE_METHODS,
        cell_messages: PLUGIN_CELL_MESSAGES,
    };
}

/// A boxed, `Send` future borrowing for `'a`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The shared base context a plugin handler gets: the transport, the
/// session maps, the channel to the cell and the database pool. The same
/// fields as crafting's `CraftCtx`, so a handler written against one reads
/// the other. All borrows, so it is `Copy`.
#[derive(Clone, Copy)]
pub struct BaseCtx<'a> {
    pub db_pool: &'a Option<Arc<PgPool>>,
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// One decoded SGWPlayer base-method call, as the router hands it to a
/// plugin. `method_index` is the flattened index (the message id minus
/// `0xC0`); `args` are the method's bytes.
pub struct BaseMethodCall<'a> {
    pub addr: SocketAddr,
    pub method_index: u8,
    pub args: &'a [u8],
    pub player_name: &'a Option<String>,
    pub key: [u8; 32],
    pub entity_manager: &'a Arc<Mutex<EntityManager>>,
    pub ctx: BaseCtx<'a>,
}

/// A plugin's handler for one base method.
pub type BaseMethodHandler = for<'a> fn(BaseMethodCall<'a>) -> BoxFuture<'a, ()>;

/// A plugin's consumer for one envelope payload type. It gets the whole
/// envelope and downcasts it (`msg.downcast::<T>()`); the registry only
/// routes envelopes whose payload is the registered type.
pub type CellMessageHandler = for<'a> fn(PluginMsg, BaseCtx<'a>) -> BoxFuture<'a, ()>;

/// What a [`SessionHookPoint`] hook is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionEvent {
    /// The player entity the session played.
    pub entity_id: u32,
    /// Why, for the hook's logs: `"log_off"`, `"gate_travel"`, or the
    /// teardown's disconnect reason.
    pub cause: &'static str,
}

/// A hook fired at a [`SessionHookPoint`].
pub type SessionHook = fn(SessionEvent);

/// A hook fired at a [`SessionStateHookPoint`], with the session's state.
pub type SessionStateHook = fn(&mut ConnectedClientState);

/// What a [`WorldEntryHookPoint`] hook gets.
#[derive(Clone, Copy)]
pub struct WorldEntryCall<'a> {
    pub addr: SocketAddr,
    pub entity_id: u32,
    /// The character entering the world.
    pub player_id: i32,
    pub ctx: BaseCtx<'a>,
}

/// A hook fired at a [`WorldEntryHookPoint`].
pub type WorldEntryHook = for<'a> fn(WorldEntryCall<'a>) -> BoxFuture<'a, ()>;

/// What an [`ItemUseHookPoint`] hook gets: one `useItem` of inventory
/// instance `item_id`.
#[derive(Clone, Copy)]
pub struct ItemUseCall<'a> {
    pub entity_id: u32,
    pub player_id: i32,
    pub item_id: i32,
    /// The database pool `useItem` already resolved (it refuses without one).
    pub pool: &'a Arc<PgPool>,
    pub ctx: BaseCtx<'a>,
}

/// What an [`ItemUseHookPoint`] hook did with a `useItem`.
#[derive(Debug)]
pub enum ItemUseOutcome {
    /// Not this plugin's item: the next plugin, then the core, decides.
    NotHandled,
    /// The plugin refused the use and told the player; nothing changed.
    Refused,
    /// The plugin consumed the item; the core brings the client's inventory
    /// and the cell up to date.
    Consumed(ItemConsumed),
}

/// An item a plugin consumed on `useItem`.
#[derive(Debug)]
pub struct ItemConsumed {
    /// The instance is gone (its last one was used): the client needs
    /// `onRemoveItem` as well as the inventory update.
    pub removed_all: bool,
    /// The cell notification for a removed instance, enqueued with the
    /// commit, for the core to dispatch now.
    pub outbox: Option<(i64, crate::base::outbox::CellOutboxPayload)>,
}

/// A hook fired at an [`ItemUseHookPoint`]. The first hook in table order
/// that does not return [`ItemUseOutcome::NotHandled`] decides.
pub type ItemUseHook = for<'a> fn(ItemUseCall<'a>) -> BoxFuture<'a, ItemUseOutcome>;

/// What an [`InventoryHookPoint`] hook gets: the player's whole inventory as
/// the resync just read it.
#[derive(Clone, Copy)]
pub struct InventoryCall<'a> {
    pub entity_id: u32,
    pub player_id: i32,
    pub pool: &'a PgPool,
    /// `(item_id, type_id, container_id)` per inventory row.
    pub rows: &'a [(i32, i32, i32)],
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// A hook fired at an [`InventoryHookPoint`].
pub type InventoryHook = for<'a> fn(InventoryCall<'a>) -> BoxFuture<'a, ()>;

/// What a [`ProgressionHookPoint`] hook gets: the applied-science total an
/// XP grant left the player with.
#[derive(Clone, Copy)]
pub struct AppliedScienceCall<'a> {
    pub entity_id: u32,
    pub player_id: i32,
    /// The new total, not the points earned.
    pub total: i32,
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// A hook fired at a [`ProgressionHookPoint`].
pub type ProgressionHook = for<'a> fn(AppliedScienceCall<'a>) -> BoxFuture<'a, ()>;

/// A feature that registers with the base at startup.
pub trait BasePlugin: Send + Sync + 'static {
    /// Stable name for logs and startup errors (`"crafting"`).
    fn name(&self) -> &'static str;

    /// Register this plugin's handlers and hooks. Called once, at startup,
    /// in plugin-table order.
    fn build(&self, plugin: &mut BasePluginBuilder<'_>);
}

/// The registry of the session at `addr`, cloned (one `Arc`) so the
/// connected-map lock is released before a hook runs. Empty when there is
/// no such session or the lock is poisoned: a hook site with no session
/// has nothing to fire for.
pub fn session_plugins(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    addr: SocketAddr,
) -> BasePlugins {
    connected
        .lock()
        .ok()
        .and_then(|clients| clients.get(&addr).map(|c| c.plugins.clone()))
        .unwrap_or_default()
}

/// [`session_plugins`] for the session playing player entity `entity_id`,
/// for the hook sites that hold an entity id rather than an address.
pub fn entity_plugins(
    connected: &Mutex<HashMap<SocketAddr, ConnectedClientState>>,
    entity_to_addr: &Mutex<HashMap<u32, SocketAddr>>,
    entity_id: u32,
) -> BasePlugins {
    let addr = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied());
    match addr {
        Some(addr) => session_plugins(connected, addr),
        None => BasePlugins::empty(),
    }
}
