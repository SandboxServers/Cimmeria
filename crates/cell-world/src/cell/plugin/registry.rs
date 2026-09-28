//! [`CellPluginBuilder`] collects registrations; [`CellPlugins`] is the
//! validated, frozen registry the `SpaceManager` holds.

use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::mpsc;

use super::{
    CellMethodHandler, CellPlugin, EntityHook, EntityHookPoint, TickHook, TickStage,
    PLUGIN_OWNED_CELL_METHODS,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// A registration `CellPlugins::build` or `check_complete` refused. Each is
/// a startup failure: the server must not run with a method the client can
/// call and nothing handles, or with two handlers for one index.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PluginError {
    #[error(
        "cell method {index} ({name}) registered by both plugin `{first}` and plugin `{second}`"
    )]
    DuplicateCellMethod {
        index: u16,
        name: &'static str,
        first: &'static str,
        second: &'static str,
    },
    #[error("plugin `{plugin}` registered cell method {index}, which is not a client cell method")]
    UnknownCellMethod { index: u16, plugin: &'static str },
    #[error(
        "plugin `{plugin}` registered cell method {index} ({name}), which is not in \
         PLUGIN_OWNED_CELL_METHODS: the static router may still handle it"
    )]
    NotPluginOwned {
        index: u16,
        name: &'static str,
        plugin: &'static str,
    },
    #[error("plugin-owned cell methods with no handler (is a plugin missing from the table?): {missing:?}")]
    MissingCellMethods { missing: Vec<(u16, &'static str)> },
}

/// What a plugin registers, handed to [`CellPlugin::build`].
pub struct CellPluginBuilder<'r> {
    plugin: &'static str,
    registry: &'r mut Registry,
}

impl CellPluginBuilder<'_> {
    /// Handle client cell method `index` (the flattened index, a constant
    /// from `cimmeria-wire`). Validated by [`CellPlugins::build`].
    pub fn cell_method(&mut self, index: u16, handler: CellMethodHandler) -> &mut Self {
        self.registry
            .pending_methods
            .push((index, self.plugin, handler));
        self
    }

    /// Run `hook` once per AoI tick at `stage`.
    pub fn tick(&mut self, stage: TickStage, hook: TickHook) -> &mut Self {
        self.registry.ticks.push((stage, self.plugin, hook));
        self
    }

    /// Run `hook` for an entity at `point`.
    pub fn entity_hook(&mut self, point: EntityHookPoint, hook: EntityHook) -> &mut Self {
        self.registry.entity_hooks.push((point, self.plugin, hook));
        self
    }
}

#[derive(Default)]
struct Registry {
    plugins: Vec<&'static str>,
    pending_methods: Vec<(u16, &'static str, CellMethodHandler)>,
    methods: BTreeMap<u16, (&'static str, CellMethodHandler)>,
    /// In registration (table) order; fired in that order per stage.
    ticks: Vec<(TickStage, &'static str, TickHook)>,
    entity_hooks: Vec<(EntityHookPoint, &'static str, EntityHook)>,
}

/// The validated plugin registry. Cheap to clone (one `Arc`).
#[derive(Clone, Default)]
pub struct CellPlugins {
    inner: Arc<Registry>,
}

fn method_name(index: u16) -> &'static str {
    cimmeria_wire::cell::dispatch::names::cell_method_name(index)
}

impl CellPlugins {
    /// No plugins. What a fresh `SpaceManager` holds until the cell service
    /// installs the real table.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Run each plugin's `build` in table order and validate the result.
    pub fn build(plugins: &[&dyn CellPlugin]) -> Result<Self, PluginError> {
        let mut registry = Registry::default();
        for plugin in plugins {
            registry.plugins.push(plugin.name());
            plugin.build(&mut CellPluginBuilder {
                plugin: plugin.name(),
                registry: &mut registry,
            });
        }
        for (index, plugin, handler) in std::mem::take(&mut registry.pending_methods) {
            let name = method_name(index);
            if name == "unknown" {
                return Err(PluginError::UnknownCellMethod { index, plugin });
            }
            if !PLUGIN_OWNED_CELL_METHODS.contains(&index) {
                return Err(PluginError::NotPluginOwned {
                    index,
                    name,
                    plugin,
                });
            }
            if let Some((first, _)) = registry.methods.get(&index) {
                return Err(PluginError::DuplicateCellMethod {
                    index,
                    name,
                    first,
                    second: plugin,
                });
            }
            registry.methods.insert(index, (plugin, handler));
        }
        Ok(Self {
            inner: Arc::new(registry),
        })
    }

    /// Every index in [`PLUGIN_OWNED_CELL_METHODS`] has a handler. An `Err`
    /// names the ones that do not: the client can call them and nothing
    /// would answer.
    pub fn check_complete(&self) -> Result<(), PluginError> {
        let missing: Vec<(u16, &'static str)> = PLUGIN_OWNED_CELL_METHODS
            .iter()
            .filter(|i| !self.inner.methods.contains_key(i))
            .map(|&i| (i, method_name(i)))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(PluginError::MissingCellMethods { missing })
        }
    }

    /// The installed plugins' names, in table order.
    pub fn plugin_names(&self) -> &[&'static str] {
        &self.inner.plugins
    }

    /// The registered cell-method indices, ascending.
    pub fn cell_method_indices(&self) -> impl Iterator<Item = u16> + '_ {
        self.inner.methods.keys().copied()
    }

    /// The handler for cell method `index`, if a plugin registered one.
    pub fn cell_method(&self, index: u16) -> Option<CellMethodHandler> {
        self.inner.methods.get(&index).map(|(_, h)| *h)
    }

    /// Fire every hook registered for `stage`, in table order.
    pub async fn run_tick(
        &self,
        stage: TickStage,
        tx: &mpsc::Sender<CellToBaseMsg>,
        space_mgr: &mut SpaceManager,
    ) {
        for (s, _, hook) in &self.inner.ticks {
            if *s == stage {
                hook(tx, space_mgr).await;
            }
        }
    }

    /// Fire every hook registered for `point` for `entity_id`, in table
    /// order.
    pub async fn run_entity_hook(
        &self,
        point: EntityHookPoint,
        entity_id: u32,
        tx: &mpsc::Sender<CellToBaseMsg>,
        space_mgr: &mut SpaceManager,
    ) {
        for (p, _, hook) in &self.inner.entity_hooks {
            if *p == point {
                hook(entity_id, tx, space_mgr).await;
            }
        }
    }
}

impl std::fmt::Debug for CellPlugins {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellPlugins")
            .field("plugins", &self.inner.plugins)
            .field(
                "cell_methods",
                &self.inner.methods.keys().collect::<Vec<_>>(),
            )
            .field("ticks", &self.inner.ticks.len())
            .field("entity_hooks", &self.inner.entity_hooks.len())
            .finish()
    }
}
