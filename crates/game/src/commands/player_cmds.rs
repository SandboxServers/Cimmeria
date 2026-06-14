use cimmeria_commands::intent::GmCommandIntent;
use cimmeria_commands::permissions::AccessLevel;
use cimmeria_commands::registry::{CommandHandler, CommandOutcome, CommandRegistry};

/// Register player-accessible (sub-GM) commands into the given registry.
///
/// Today this is just `/who`, gated at `Moderator`. The earlier
/// `stuck`/`wave`/`sit` stubs were removed: they had no typed
/// [`GmCommandIntent`] and would have surfaced as live-but-inert commands,
/// which is worse than not registering them at all. Re-add them here when
/// they get a real intent + cell executor.
pub fn register_player_commands(registry: &mut CommandRegistry) {
    registry.register(
        "who",
        "List players in your space",
        "/who",
        AccessLevel::Moderator,
        who_handler(),
    );
}

fn who_handler() -> CommandHandler {
    Box::new(|ctx, _args| {
        tracing::debug!(caller = %ctx.caller_name, "/who");
        CommandOutcome::Cell(GmCommandIntent::Who)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_commands::registry::CommandContext;
    use cimmeria_common::types::EntityId;

    fn ctx(access: AccessLevel) -> CommandContext {
        CommandContext {
            caller_entity_id: Some(EntityId(1)),
            caller_name: "Tester".to_string(),
            access_level: access,
        }
    }

    #[test]
    fn player_commands_register() {
        let mut registry = CommandRegistry::new();
        register_player_commands(&mut registry);
        let cmds = registry.list_commands(AccessLevel::Moderator);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].name, "who");
    }

    #[test]
    fn who_produces_intent_for_moderator() {
        let mut registry = CommandRegistry::new();
        register_player_commands(&mut registry);
        let result = registry.dispatch(&ctx(AccessLevel::Moderator), "/who");
        assert_eq!(result, CommandOutcome::Cell(GmCommandIntent::Who));
    }

    /// `/who` is Moderator-gated, so a plain Player must be denied.
    #[test]
    fn who_denied_for_player() {
        let mut registry = CommandRegistry::new();
        register_player_commands(&mut registry);
        let result = registry.dispatch(&ctx(AccessLevel::Player), "/who");
        assert!(matches!(result, CommandOutcome::Error(_)));
    }
}
