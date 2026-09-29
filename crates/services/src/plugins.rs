//! The cell plugin table (#962, `docs/architecture/plugin-architecture.md`).
//!
//! The facade is the composition root: it is the one crate that names every
//! feature plugin, so a feature crate stays a leaf. Order matters where two
//! plugins subscribe to one hook point (they fire in this order), so the
//! table is an explicit list, never link-time discovery.

use cimmeria_cell_duel::DuelPlugin;
use cimmeria_cell_pets::PetsPlugin;
use cimmeria_cell_world::cell::plugin::{CellPlugin, CellPlugins, PluginError};

/// Every cell plugin, in hook-firing order.
///
/// No two plugins share a hook point yet: pets use the owner-sweep and
/// arrival stages and the base-destroy hook; duels use the gate-crossing
/// stage and the disconnect, travel and death hooks. So the order reaches no
/// wire output today; a plugin that joins a shared point must pin its order
/// with a test (ADR §3.2).
pub fn cell_plugin_table() -> [&'static dyn CellPlugin; 2] {
    [&PetsPlugin, &DuelPlugin]
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
        assert_eq!(plugins.plugin_names(), &["pets", "duel"]);
        assert_eq!(
            plugins.cell_method_indices().collect::<Vec<_>>(),
            PLUGIN_OWNED_CELL_METHODS.to_vec()
        );
    }

    /// #962 test rule, for step 2: a table missing `DuelPlugin` must not
    /// start. `check_complete` names both duel methods, so the orchestrator
    /// refuses to start the cell (`plugin_table_incomplete`) instead of
    /// letting a duel answer or forfeit fall through to "Unhandled cell
    /// method call".
    #[test]
    fn a_table_without_the_duel_plugin_fails_the_startup_check() {
        use cimmeria_wire::cell::cell_methods::player::constants::{
            DUEL_FORFEIT, SEND_DUEL_RESPONSE,
        };

        let plugins = CellPlugins::build(&[&PetsPlugin]).expect("pets alone still builds");
        match plugins.check_complete() {
            Err(PluginError::MissingCellMethods { missing }) => {
                let indices: Vec<u16> = missing.iter().map(|(i, _)| *i).collect();
                assert_eq!(indices, vec![SEND_DUEL_RESPONSE, DUEL_FORFEIT]);
                assert_eq!(
                    missing.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
                    vec!["sendDuelResponse", "duelForfeit"]
                );
            }
            other => panic!("a table without DuelPlugin must fail check_complete: {other:?}"),
        }
    }
}
