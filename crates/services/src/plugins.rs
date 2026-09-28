//! The cell plugin table (#962, `docs/architecture/plugin-architecture.md`).
//!
//! The facade is the composition root: it is the one crate that names every
//! feature plugin, so a feature crate stays a leaf. Order matters where two
//! plugins subscribe to one hook point (they fire in this order), so the
//! table is an explicit list, never link-time discovery.

use cimmeria_cell_pets::PetsPlugin;
use cimmeria_cell_world::cell::plugin::{CellPlugin, CellPlugins, PluginError};

/// Every cell plugin, in hook-firing order.
pub fn cell_plugin_table() -> [&'static dyn CellPlugin; 1] {
    [&PetsPlugin]
}

/// The built and checked plugin registry the orchestrator installs on the
/// cell: `Err` when a registration is invalid or a plugin-owned cell method
/// has no handler.
pub fn cell_plugins() -> Result<CellPlugins, PluginError> {
    let plugins = CellPlugins::build(&cell_plugin_table())?;
    plugins.check_complete()?;
    Ok(plugins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_cell_world::cell::plugin::PLUGIN_OWNED_CELL_METHODS;

    /// The shipped table builds and covers every plugin-owned cell method,
    /// so the orchestrator's startup check passes. (That no static router arm
    /// still claims one of them is `cimmeria-cell-methods`'
    /// `plugin_owned_methods_are_not_routed_here`.)
    #[test]
    fn the_default_table_builds_and_is_complete() {
        let plugins = cell_plugins().expect("the shipped plugin table must build");
        assert_eq!(plugins.plugin_names(), &["pets"]);
        assert_eq!(
            plugins.cell_method_indices().collect::<Vec<_>>(),
            PLUGIN_OWNED_CELL_METHODS.to_vec()
        );
    }
}
