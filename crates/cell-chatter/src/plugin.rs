//! [`ChatterPlugin`]: what ambient chatter registers with the cell at startup.
//!
//! - [`TickStage::AfterStatBuffs`]: the chatter tick, which starts a group's
//!   next exchange when a player is in earshot and speaks the lines that are
//!   due. It sends only `onPlayerCommunication` lines, which no other tick
//!   orders against, so the stage only has to be one that runs every tick.

use cimmeria_cell_world::cell::plugin::{BoxFuture, CellPlugin, CellPluginBuilder, TickStage};
use tokio::sync::mpsc;

use crate::cell::chatter;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Ambient chatter, as a cell plugin.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChatterPlugin;

impl CellPlugin for ChatterPlugin {
    fn name(&self) -> &'static str {
        "chatter"
    }

    fn build(&self, plugin: &mut CellPluginBuilder<'_>) {
        plugin.tick(TickStage::AfterStatBuffs, chatter_tick);
    }
}

fn chatter_tick<'a>(
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> BoxFuture<'a, ()> {
    Box::pin(async move {
        chatter::run(tx, space_mgr).await;
    })
}

#[cfg(test)]
mod tests {
    use cimmeria_cell_world::cell::plugin::CellPlugins;

    use super::*;

    /// The plugin builds on its own and owns no cell method: it is a tick
    /// and nothing else, so it can never claim an index another feature
    /// handles.
    #[test]
    fn registers_a_tick_and_no_cell_method() {
        let plugins = CellPlugins::build(&[&ChatterPlugin]).expect("chatter builds");
        assert_eq!(plugins.plugin_names(), ["chatter"]);
        assert_eq!(plugins.cell_method_indices().count(), 0);
    }
}
