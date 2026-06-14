use cimmeria_commands::intent::GmCommandIntent;
use cimmeria_commands::parser;
use cimmeria_commands::permissions::AccessLevel;
use cimmeria_commands::registry::{CommandHandler, CommandOutcome, CommandRegistry};

/// Register all GM/admin commands into the given registry.
///
/// Each handler PARSES + AUTHORIZES only — it never touches world state.
/// World-mutating commands return [`CommandOutcome::Cell`] carrying a typed
/// [`GmCommandIntent`]; the base forwards that to the cell, which executes it
/// and owns the feedback send back to the GM.
pub fn register_gm_commands(registry: &mut CommandRegistry) {
    registry.register(
        "spawn",
        "Spawn an NPC near you",
        "/spawn <moniker> [count]",
        AccessLevel::GameMaster,
        spawn_handler(),
    );
    registry.register(
        "goto",
        "Teleport to coordinates or a player",
        "/goto <x> <y> <z> | /goto <player>",
        AccessLevel::GameMaster,
        goto_handler(),
    );
    registry.register(
        "kill",
        "Kill an NPC (your current target, or a named one)",
        "/kill [target_name]",
        AccessLevel::GameMaster,
        kill_handler(),
    );
    registry.register(
        "give",
        "Give yourself an item",
        "/give <item_id> [count]",
        AccessLevel::GameMaster,
        give_handler(),
    );
    registry.register(
        "info",
        "Inspect your current target (or yourself)",
        "/info",
        AccessLevel::GameMaster,
        info_handler(),
    );
}

fn spawn_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        let Some(moniker) = args.first() else {
            return CommandOutcome::Usage("/spawn <moniker> [count]".to_string());
        };
        // count default 1, parsed from args[1]. A non-numeric count is
        // silently treated as 1 rather than rejected — the GM almost
        // certainly meant "one of this template" and a usage error here
        // would be more annoying than helpful. The cell caps the count.
        let count: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        tracing::info!(caller = %ctx.caller_name, moniker, count, "GM /spawn");
        CommandOutcome::Cell(GmCommandIntent::Spawn {
            moniker: moniker.to_string(),
            count,
        })
    })
}

fn goto_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        // Three numeric args -> coordinate teleport; exactly one arg ->
        // player teleport. `parse_vector3` consumes the first three args and
        // returns None on any non-numeric, so a `/goto <player>` with a name
        // that happens to be 3 tokens can't be misread as coords.
        if args.len() >= 3 {
            if let Some(pos) = parser::parse_vector3(args) {
                tracing::info!(caller = %ctx.caller_name, ?pos, "GM /goto coords");
                return CommandOutcome::Cell(GmCommandIntent::GotoCoords(pos));
            }
            return CommandOutcome::Usage("/goto <x> <y> <z> | /goto <player>".to_string());
        }
        if args.len() == 1 {
            let player = args[0];
            tracing::info!(caller = %ctx.caller_name, target = player, "GM /goto player");
            return CommandOutcome::Cell(GmCommandIntent::GotoPlayer(player.to_string()));
        }
        CommandOutcome::Usage("/goto <x> <y> <z> | /goto <player>".to_string())
    })
}

fn kill_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        // No arg -> kill the caller's current target. A name -> kill that
        // (NPC) by name. The cell resolves and gates on `!is_player`.
        let target = args.first().map(|s| s.to_string());
        tracing::info!(caller = %ctx.caller_name, target = ?target, "GM /kill");
        CommandOutcome::Cell(GmCommandIntent::Kill { target })
    })
}

fn give_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        // item_id is required and must parse as i32. count default 1.
        let item_id = match args.first().and_then(|s| s.parse::<i32>().ok()) {
            Some(id) => id,
            None => return CommandOutcome::Usage("/give <item_id> [count]".to_string()),
        };
        let count: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        tracing::info!(caller = %ctx.caller_name, item_id, count, "GM /give");
        CommandOutcome::Cell(GmCommandIntent::Give { item_id, count })
    })
}

fn info_handler() -> CommandHandler {
    Box::new(|ctx, _args| {
        tracing::debug!(caller = %ctx.caller_name, "GM /info");
        CommandOutcome::Cell(GmCommandIntent::Info)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_commands::registry::CommandContext;
    use cimmeria_common::math::Vector3;
    use cimmeria_common::types::EntityId;

    fn gm_ctx() -> CommandContext {
        CommandContext {
            caller_entity_id: Some(EntityId(1)),
            caller_name: "GM_Test".to_string(),
            access_level: AccessLevel::GameMaster,
        }
    }

    #[test]
    fn gm_commands_register() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let cmds = registry.list_commands(AccessLevel::Developer);
        assert_eq!(cmds.len(), 5);
    }

    #[test]
    fn spawn_without_args_shows_usage() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/spawn");
        assert!(matches!(result, CommandOutcome::Usage(_)));
    }

    #[test]
    fn spawn_with_moniker_produces_intent() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/spawn jaffa_guard 3");
        assert_eq!(
            result,
            CommandOutcome::Cell(GmCommandIntent::Spawn {
                moniker: "jaffa_guard".to_string(),
                count: 3,
            })
        );
    }

    #[test]
    fn spawn_defaults_count_to_one() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/spawn jaffa_guard");
        assert_eq!(
            result,
            CommandOutcome::Cell(GmCommandIntent::Spawn {
                moniker: "jaffa_guard".to_string(),
                count: 1,
            })
        );
    }

    #[test]
    fn goto_three_numeric_args_is_coords() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/goto 10 20 30");
        assert_eq!(
            result,
            CommandOutcome::Cell(GmCommandIntent::GotoCoords(Vector3::new(10.0, 20.0, 30.0)))
        );
    }

    #[test]
    fn goto_single_arg_is_player() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/goto Jack");
        assert_eq!(
            result,
            CommandOutcome::Cell(GmCommandIntent::GotoPlayer("Jack".to_string()))
        );
    }

    #[test]
    fn goto_no_args_shows_usage() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        assert!(matches!(
            registry.dispatch(&gm_ctx(), "/goto"),
            CommandOutcome::Usage(_)
        ));
    }

    #[test]
    fn kill_no_arg_targets_current() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/kill");
        assert_eq!(
            result,
            CommandOutcome::Cell(GmCommandIntent::Kill { target: None })
        );
    }

    #[test]
    fn kill_named_target() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/kill goauld_drone");
        assert_eq!(
            result,
            CommandOutcome::Cell(GmCommandIntent::Kill {
                target: Some("goauld_drone".to_string()),
            })
        );
    }

    #[test]
    fn give_parses_item_id_and_count() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.dispatch(&gm_ctx(), "/give 55 4");
        assert_eq!(
            result,
            CommandOutcome::Cell(GmCommandIntent::Give {
                item_id: 55,
                count: 4,
            })
        );
    }

    #[test]
    fn give_missing_item_id_shows_usage() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        assert!(matches!(
            registry.dispatch(&gm_ctx(), "/give"),
            CommandOutcome::Usage(_)
        ));
        assert!(matches!(
            registry.dispatch(&gm_ctx(), "/give notanumber"),
            CommandOutcome::Usage(_)
        ));
    }

    #[test]
    fn info_produces_intent() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        assert_eq!(
            registry.dispatch(&gm_ctx(), "/info"),
            CommandOutcome::Cell(GmCommandIntent::Info)
        );
    }

    /// **Regression guard:** a non-GM caller must be rejected with
    /// `Error`, never reaching the handler. Reverting the `spawn`
    /// `min_access_level` to `Player` (or dropping the registry's
    /// permission check) trips this — a Player would get a `Cell` intent
    /// and gain spawn powers.
    #[test]
    fn player_cannot_run_gm_commands() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let player_ctx = CommandContext {
            caller_entity_id: Some(EntityId(2)),
            caller_name: "Player".to_string(),
            access_level: AccessLevel::Player,
        };
        let result = registry.dispatch(&player_ctx, "/spawn jaffa_guard");
        assert!(
            matches!(result, CommandOutcome::Error(_)),
            "non-GM /spawn must be denied with Error, got {result:?}"
        );
    }
}
