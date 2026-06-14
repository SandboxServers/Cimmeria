//! Base-side GM command registry.
//!
//! Built once via [`std::sync::OnceLock`] and shared across every chat
//! interception. The registry holds the parse + authorize logic only
//! (`cimmeria_game::commands::{register_gm_commands, register_player_commands}`);
//! world mutation happens on the cell after the base ships a
//! [`GmCommandIntent`](cimmeria_commands::GmCommandIntent).
//!
//! `once_cell` is not a dependency of `cimmeria-services`, so we use the std
//! `OnceLock` (stable since 1.70) instead.

use std::sync::OnceLock;

use cimmeria_commands::registry::CommandRegistry;
use cimmeria_game::commands::{register_gm_commands, register_player_commands};

static GM_REGISTRY: OnceLock<CommandRegistry> = OnceLock::new();

/// Get the process-wide GM command registry, building it on first use.
pub(crate) fn gm_registry() -> &'static CommandRegistry {
    GM_REGISTRY.get_or_init(|| {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        register_player_commands(&mut registry);
        registry
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_commands::permissions::AccessLevel;

    /// The shared registry must carry both the GM set and the player set.
    /// A GameMaster sees the 5 GM commands; a Moderator sees `/who`.
    #[test]
    fn registry_holds_gm_and_player_commands() {
        let reg = gm_registry();
        let gm_cmds: Vec<&str> = reg
            .list_commands(AccessLevel::GameMaster)
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        for expected in ["spawn", "goto", "kill", "give", "info", "who"] {
            assert!(
                gm_cmds.contains(&expected),
                "GameMaster registry missing /{expected}; has {gm_cmds:?}"
            );
        }
        // `/who` is Moderator-gated and must be visible at Moderator.
        let mod_cmds: Vec<&str> = reg
            .list_commands(AccessLevel::Moderator)
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(mod_cmds.contains(&"who"), "Moderator registry missing /who");
    }
}
