use cimmeria_commands::permissions::AccessLevel;
use cimmeria_commands::registry::{CommandHandler, CommandRegistry, CommandResult};

/// Register all player-accessible commands into the given registry.
pub fn register_player_commands(registry: &mut CommandRegistry) {
    registry.register("stuck", "Unstick your character", "/stuck", AccessLevel::Player, stuck_handler());
    registry.register("wave", "Wave emote", "/wave [target]", AccessLevel::Player, wave_handler());
    registry.register("sit", "Sit down / stand up", "/sit", AccessLevel::Player, sit_handler());
    registry.register("who", "List online players", "/who [filter]", AccessLevel::Player, who_handler());
}

fn stuck_handler() -> CommandHandler {
    Box::new(|ctx, _args| {
        tracing::info!(caller = %ctx.caller_name, "Player used /stuck");
        // TODO: teleport player to nearest safe point
        CommandResult::Success("Attempting to unstick your character...".to_string())
    })
}

fn wave_handler() -> CommandHandler {
    Box::new(|ctx, args| {
        let target = args.first().copied().unwrap_or("everyone");
        tracing::debug!(caller = %ctx.caller_name, target = target, "Wave emote");
        CommandResult::Success(format!("{} waves at {}.", ctx.caller_name, target))
    })
}

fn sit_handler() -> CommandHandler {
    Box::new(|ctx, _args| {
        tracing::debug!(caller = %ctx.caller_name, "Sit toggle");
        // TODO: toggle sit posture on entity
        CommandResult::Success("Toggling sit posture.".to_string())
    })
}

fn who_handler() -> CommandHandler {
    Box::new(|_ctx, _args| {
        // TODO: query online player list from BaseApp
        CommandResult::Success("Online players: (not implemented)".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_commands::registry::CommandContext;
    use cimmeria_common::types::EntityId;

    fn player_ctx() -> CommandContext {
        CommandContext {
            caller_entity_id: Some(EntityId(1)),
            caller_name: "TestPlayer".to_string(),
            access_level: AccessLevel::Player,
        }
    }

    #[test]
    fn player_commands_register() {
        let mut registry = CommandRegistry::new();
        register_player_commands(&mut registry);
        let cmds = registry.list_commands(AccessLevel::Player);
        assert!(cmds.len() >= 4);
    }

    #[test]
    fn stuck_returns_success() {
        let mut registry = CommandRegistry::new();
        register_player_commands(&mut registry);
        let result = registry.execute(&player_ctx(), "/stuck");
        assert!(matches!(result, CommandResult::Success(_)));
    }

    #[test]
    fn wave_with_target() {
        let mut registry = CommandRegistry::new();
        register_player_commands(&mut registry);
        let result = registry.execute(&player_ctx(), "/wave Jack");
        match result {
            CommandResult::Success(msg) => assert!(msg.contains("Jack")),
            _ => panic!("Expected Success"),
        }
    }
}
