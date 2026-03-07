use cimmeria_commands::permissions::AccessLevel;
use cimmeria_commands::registry::{CommandHandler, CommandRegistry, CommandResult};

/// Register all GM/admin commands into the given registry.
pub fn register_gm_commands(registry: &mut CommandRegistry) {
    registry.register("spawn", "Spawn an entity", "/spawn <moniker> [count]", AccessLevel::GameMaster, spawn_handler());
    registry.register("teleport", "Teleport to coordinates or a player", "/teleport <x> <y> <z> | /teleport <player>", AccessLevel::GameMaster, teleport_handler());
    registry.register("kill", "Kill a target entity", "/kill [target]", AccessLevel::GameMaster, kill_handler());
    registry.register("give", "Give an item to a player", "/give <player> <item_id> [count]", AccessLevel::GameMaster, give_handler());
    registry.register("setlevel", "Set a player's level", "/setlevel <player> <level>", AccessLevel::Admin, setlevel_handler());
    registry.register("shutdown", "Shut down the server", "/shutdown [delay_secs]", AccessLevel::Admin, shutdown_handler());
}

fn spawn_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        if args.is_empty() {
            return CommandResult::Usage("/spawn <moniker> [count]".to_string());
        }
        let moniker = args[0];
        let count: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        tracing::info!(caller = %ctx.caller_name, moniker = moniker, count = count, "GM spawn");
        // TODO: create entity via CellApp
        CommandResult::Success(format!("Spawning {} x{}.", moniker, count))
    })
}

fn teleport_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        if args.is_empty() {
            return CommandResult::Usage("/teleport <x> <y> <z> | /teleport <player>".to_string());
        }
        tracing::info!(caller = %ctx.caller_name, args = ?args, "GM teleport");
        // TODO: parse coords or player name, move entity
        CommandResult::Success(format!("Teleporting to {:?}.", args))
    })
}

fn kill_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        let target = args.first().copied().unwrap_or("current target");
        tracing::info!(caller = %ctx.caller_name, target = target, "GM kill");
        // TODO: set target entity health to 0
        CommandResult::Success(format!("Killing {}.", target))
    })
}

fn give_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        if args.len() < 2 {
            return CommandResult::Usage("/give <player> <item_id> [count]".to_string());
        }
        let player = args[0];
        let item_id = args[1];
        let count = args.get(2).copied().unwrap_or("1");
        tracing::info!(
            caller = %ctx.caller_name,
            target = player,
            item = item_id,
            count = count,
            "GM give item"
        );
        // TODO: look up item template, create instance, add to player inventory
        CommandResult::Success(format!("Giving {} x{} of item {} .", player, count, item_id))
    })
}

fn setlevel_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        if args.len() < 2 {
            return CommandResult::Usage("/setlevel <player> <level>".to_string());
        }
        let player = args[0];
        let level = args[1];
        tracing::info!(caller = %ctx.caller_name, target = player, level = level, "GM setlevel");
        // TODO: set player level and recalculate stats
        CommandResult::Success(format!("Setting {} to level {}.", player, level))
    })
}

fn shutdown_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        let delay: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(30);
        tracing::warn!(caller = %ctx.caller_name, delay_secs = delay, "Server shutdown requested");
        // TODO: broadcast warning, save all, shut down
        CommandResult::Success(format!("Server shutting down in {} seconds.", delay))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_commands::registry::CommandContext;
    use cimmeria_common::types::EntityId;

    fn gm_ctx() -> CommandContext {
        CommandContext {
            caller_entity_id: Some(EntityId(1)),
            caller_name: "GM_Test".to_string(),
            access_level: AccessLevel::Developer,
        }
    }

    #[test]
    fn gm_commands_register() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let cmds = registry.list_commands(AccessLevel::Developer);
        assert!(cmds.len() >= 6);
    }

    #[test]
    fn spawn_without_args_shows_usage() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.execute(&gm_ctx(), "/spawn");
        assert!(matches!(result, CommandResult::Usage(_)));
    }

    #[test]
    fn spawn_with_moniker() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let result = registry.execute(&gm_ctx(), "/spawn jaffa_guard 3");
        match result {
            CommandResult::Success(msg) => {
                assert!(msg.contains("jaffa_guard"));
                assert!(msg.contains("x3"));
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn player_cannot_run_gm_commands() {
        let mut registry = CommandRegistry::new();
        register_gm_commands(&mut registry);
        let player_ctx = CommandContext {
            caller_entity_id: Some(EntityId(2)),
            caller_name: "Player".to_string(),
            access_level: AccessLevel::Player,
        };
        let result = registry.execute(&player_ctx, "/spawn jaffa_guard");
        assert!(matches!(result, CommandResult::Error(_)));
    }
}
