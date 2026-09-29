//! [`BasePluginBuilder`] collects registrations; [`BasePlugins`] is the
//! validated, frozen registry every session carries.

use std::any::TypeId;
use std::collections::BTreeMap;
use std::sync::Arc;

use cimmeria_wire::base::names::base_method_name;

use super::{
    BaseCtx, BaseMethodHandler, BasePlugin, CellMessageHandler, PluginMsg, PluginOwnership,
    SessionEvent, SessionHook, SessionHookPoint, SessionStateHook, SessionStateHookPoint,
    WorldEntryCall, WorldEntryHook, WorldEntryHookPoint,
};
use crate::base::ConnectedClientState;

/// A registration `BasePlugins::build` or `check_complete` refused. Each is
/// a startup failure: the server must not run with a base method the client
/// can call and nothing handles, with two handlers for one index, or with a
/// feature message nobody consumes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BasePluginError {
    #[error(
        "base method {index} ({name}) registered by both plugin `{first}` and plugin `{second}`"
    )]
    DuplicateBaseMethod {
        index: u8,
        name: &'static str,
        first: &'static str,
        second: &'static str,
    },
    #[error(
        "plugin `{plugin}` registered base method {index}, which is not an SGWPlayer exposed \
         base method"
    )]
    UnknownBaseMethod { index: u8, plugin: &'static str },
    #[error(
        "plugin `{plugin}` registered base method {index} ({name}), which is not in \
         PLUGIN_OWNED_BASE_METHODS: the static router may still handle it"
    )]
    NotPluginOwned {
        index: u8,
        name: &'static str,
        plugin: &'static str,
    },
    #[error(
        "plugin-owned base methods with no handler (is a plugin missing from the table?): \
         {missing:?}"
    )]
    MissingBaseMethods { missing: Vec<(u8, &'static str)> },
    #[error("cell message `{type_name}` consumed by both plugin `{first}` and plugin `{second}`")]
    DuplicateCellMessage {
        type_name: &'static str,
        first: &'static str,
        second: &'static str,
    },
    #[error(
        "plugin `{plugin}` consumes cell message `{type_name}`, which is not in \
         PLUGIN_CELL_MESSAGES"
    )]
    UndeclaredCellMessage {
        type_name: &'static str,
        plugin: &'static str,
    },
    #[error("cell messages with no consumer (is a plugin missing from the table?): {missing:?}")]
    MissingCellMessageConsumers { missing: Vec<&'static str> },
}

/// What a plugin registers, handed to [`BasePlugin::build`].
pub struct BasePluginBuilder<'r> {
    plugin: &'static str,
    registry: &'r mut Registry,
}

impl BasePluginBuilder<'_> {
    /// Handle SGWPlayer base method `index` (flattened: the message id minus
    /// `0xC0`, a constant from `cimmeria-wire`). Validated by
    /// [`BasePlugins::build`].
    pub fn base_method(&mut self, index: u8, handler: BaseMethodHandler) -> &mut Self {
        self.registry
            .pending_methods
            .push((index, self.plugin, handler));
        self
    }

    /// Consume every `CellToBaseMsg::Plugin` envelope whose payload is a
    /// `T`. `T` must be in `PLUGIN_CELL_MESSAGES`.
    pub fn on_cell_message<T: std::any::Any>(&mut self, handler: CellMessageHandler) -> &mut Self {
        self.registry.pending_messages.push((
            TypeId::of::<T>(),
            std::any::type_name::<T>(),
            self.plugin,
            handler,
        ));
        self
    }

    /// Run `hook` at `point`.
    pub fn session_hook(&mut self, point: SessionHookPoint, hook: SessionHook) -> &mut Self {
        self.registry.session_hooks.push((point, self.plugin, hook));
        self
    }

    /// Run `hook` with the session's state at `point`.
    pub fn session_state_hook(
        &mut self,
        point: SessionStateHookPoint,
        hook: SessionStateHook,
    ) -> &mut Self {
        self.registry
            .session_state_hooks
            .push((point, self.plugin, hook));
        self
    }

    /// Run `hook` at world-entry `point`.
    pub fn world_entry_hook(
        &mut self,
        point: WorldEntryHookPoint,
        hook: WorldEntryHook,
    ) -> &mut Self {
        self.registry
            .world_entry_hooks
            .push((point, self.plugin, hook));
        self
    }
}

type PendingMessage = (TypeId, &'static str, &'static str, CellMessageHandler);

struct Registry {
    ownership: PluginOwnership,
    plugins: Vec<&'static str>,
    pending_methods: Vec<(u8, &'static str, BaseMethodHandler)>,
    methods: BTreeMap<u8, (&'static str, BaseMethodHandler)>,
    pending_messages: Vec<PendingMessage>,
    /// `(type, type name, plugin, consumer)`, in registration order.
    messages: Vec<PendingMessage>,
    /// In registration (table) order; fired in that order per point.
    session_hooks: Vec<(SessionHookPoint, &'static str, SessionHook)>,
    session_state_hooks: Vec<(SessionStateHookPoint, &'static str, SessionStateHook)>,
    world_entry_hooks: Vec<(WorldEntryHookPoint, &'static str, WorldEntryHook)>,
}

impl Registry {
    fn new(ownership: PluginOwnership) -> Self {
        Self {
            ownership,
            plugins: Vec::new(),
            pending_methods: Vec::new(),
            methods: BTreeMap::new(),
            pending_messages: Vec::new(),
            messages: Vec::new(),
            session_hooks: Vec::new(),
            session_state_hooks: Vec::new(),
            world_entry_hooks: Vec::new(),
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new(PluginOwnership::CURRENT)
    }
}

/// The validated base plugin registry. Cheap to clone (one `Arc`).
#[derive(Clone, Default)]
pub struct BasePlugins {
    inner: Arc<Registry>,
}

impl BasePlugins {
    /// No plugins. What a session holds until the service stamps the real
    /// table, and what a bare `BaseService` runs with.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Run each plugin's `build` in table order and validate the result
    /// against [`PluginOwnership::CURRENT`].
    pub fn build(plugins: &[&dyn BasePlugin]) -> Result<Self, BasePluginError> {
        Self::build_with(plugins, PluginOwnership::CURRENT)
    }

    /// [`build`](Self::build) against the given ownership lists. The
    /// registry keeps them, so [`check_complete`](Self::check_complete)
    /// checks the same lists. Tests use it while the production lists are
    /// empty.
    pub fn build_with(
        plugins: &[&dyn BasePlugin],
        ownership: PluginOwnership,
    ) -> Result<Self, BasePluginError> {
        let mut registry = Registry::new(ownership);
        for plugin in plugins {
            registry.plugins.push(plugin.name());
            plugin.build(&mut BasePluginBuilder {
                plugin: plugin.name(),
                registry: &mut registry,
            });
        }
        for (index, plugin, handler) in std::mem::take(&mut registry.pending_methods) {
            let name = base_method_name(index);
            if name == "unknown" {
                return Err(BasePluginError::UnknownBaseMethod { index, plugin });
            }
            if !ownership.base_methods.contains(&index) {
                return Err(BasePluginError::NotPluginOwned {
                    index,
                    name,
                    plugin,
                });
            }
            if let Some((first, _)) = registry.methods.get(&index) {
                return Err(BasePluginError::DuplicateBaseMethod {
                    index,
                    name,
                    first,
                    second: plugin,
                });
            }
            registry.methods.insert(index, (plugin, handler));
        }
        for pending in std::mem::take(&mut registry.pending_messages) {
            let (type_id, type_name, plugin, _) = pending;
            if !ownership
                .cell_messages
                .iter()
                .any(|k| k.type_id() == type_id)
            {
                return Err(BasePluginError::UndeclaredCellMessage { type_name, plugin });
            }
            if let Some((_, _, first, _)) = registry.messages.iter().find(|m| m.0 == type_id) {
                return Err(BasePluginError::DuplicateCellMessage {
                    type_name,
                    first,
                    second: plugin,
                });
            }
            registry.messages.push(pending);
        }
        Ok(Self {
            inner: Arc::new(registry),
        })
    }

    /// Every plugin-owned base method has a handler and every envelope
    /// payload type has a consumer. An `Err` names what does not: base
    /// methods first, since the client calls those directly.
    pub fn check_complete(&self) -> Result<(), BasePluginError> {
        let ownership = self.inner.ownership;
        let missing: Vec<(u8, &'static str)> = ownership
            .base_methods
            .iter()
            .filter(|i| !self.inner.methods.contains_key(i))
            .map(|&i| (i, base_method_name(i)))
            .collect();
        if !missing.is_empty() {
            return Err(BasePluginError::MissingBaseMethods { missing });
        }
        let missing: Vec<&'static str> = ownership
            .cell_messages
            .iter()
            .filter(|k| !self.inner.messages.iter().any(|m| m.0 == k.type_id()))
            .map(|k| k.type_name())
            .collect();
        if !missing.is_empty() {
            return Err(BasePluginError::MissingCellMessageConsumers { missing });
        }
        Ok(())
    }

    /// The installed plugins' names, in table order.
    pub fn plugin_names(&self) -> &[&'static str] {
        &self.inner.plugins
    }

    /// The registered base-method indices, ascending.
    pub fn base_method_indices(&self) -> impl Iterator<Item = u8> + '_ {
        self.inner.methods.keys().copied()
    }

    /// The handler for base method `index`, if a plugin registered one.
    pub fn base_method(&self, index: u8) -> Option<BaseMethodHandler> {
        self.inner.methods.get(&index).map(|(_, h)| *h)
    }

    /// The consumed envelope payload types' names, in registration order.
    pub fn cell_message_types(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.inner.messages.iter().map(|m| m.1)
    }

    /// Hand `msg` to the plugin that consumes its payload type. `false`
    /// when none does: the envelope is dropped with a WARN, since the
    /// feature that sent it did nothing for the player.
    pub async fn dispatch_cell_message(&self, msg: PluginMsg, ctx: BaseCtx<'_>) -> bool {
        let consumer = self
            .inner
            .messages
            .iter()
            .find(|m| m.0 == msg.type_id())
            .map(|m| m.3);
        match consumer {
            Some(handler) => {
                handler(msg, ctx).await;
                true
            }
            None => {
                tracing::warn!(
                    target: "base.plugin",
                    reason = "no_consumer",
                    type_name = msg.type_name(),
                    plugins = ?self.inner.plugins,
                    "cell message has no base plugin consumer; dropped -- the feature that \
                     sent it did nothing for the player (is its plugin missing from the table?)"
                );
                false
            }
        }
    }

    /// Fire every hook registered for `point`, in table order.
    pub fn run_session_hook(&self, point: SessionHookPoint, event: SessionEvent) {
        for (p, _, hook) in &self.inner.session_hooks {
            if *p == point {
                hook(event);
            }
        }
    }

    /// Fire every hook registered for `point` with `state`, in table order.
    pub fn run_session_state_hook(
        &self,
        point: SessionStateHookPoint,
        state: &mut ConnectedClientState,
    ) {
        for (p, _, hook) in &self.inner.session_state_hooks {
            if *p == point {
                hook(state);
            }
        }
    }

    /// Fire every hook registered for world-entry `point`, in table order.
    pub async fn run_world_entry_hook(&self, point: WorldEntryHookPoint, call: WorldEntryCall<'_>) {
        for (p, _, hook) in &self.inner.world_entry_hooks {
            if *p == point {
                hook(call).await;
            }
        }
    }
}

impl std::fmt::Debug for BasePlugins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasePlugins")
            .field("plugins", &self.inner.plugins)
            .field(
                "base_methods",
                &self.inner.methods.keys().collect::<Vec<_>>(),
            )
            .field(
                "cell_messages",
                &self.inner.messages.iter().map(|m| m.1).collect::<Vec<_>>(),
            )
            .field("session_hooks", &self.inner.session_hooks.len())
            .field("session_state_hooks", &self.inner.session_state_hooks.len())
            .field("world_entry_hooks", &self.inner.world_entry_hooks.len())
            .finish()
    }
}
