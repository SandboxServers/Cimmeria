use std::collections::HashMap;

use cimmeria_common::types::EntityId;

use crate::parser;
use crate::permissions::AccessLevel;

/// The result of executing a GM/admin command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// Command executed successfully. Contains a message to display to the caller.
    Success(String),
    /// Command failed. Contains an error description.
    Error(String),
    /// Incorrect usage. Contains the correct usage string.
    Usage(String),
}

/// A boxed command handler function.
///
/// Receives the execution context and a slice of parsed arguments,
/// and returns a `CommandResult`.
pub type CommandHandler = Box<dyn Fn(&CommandContext, &[&str]) -> CommandResult + Send + Sync>;

/// Metadata and handler for a registered command.
pub struct CommandInfo {
    /// The command name (e.g., "teleport", "kick", "spawn").
    pub name: String,
    /// A short description of what the command does.
    pub description: String,
    /// Usage string showing expected arguments (e.g., "/teleport <player> <x> <y> <z>").
    pub usage: String,
    /// Minimum access level required to execute this command.
    pub min_access_level: AccessLevel,
    /// The handler function that implements the command logic.
    pub handler: CommandHandler,
}

impl std::fmt::Debug for CommandInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandInfo")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("usage", &self.usage)
            .field("min_access_level", &self.min_access_level)
            .finish_non_exhaustive()
    }
}

/// Execution context passed to command handlers.
///
/// Provides information about who invoked the command so handlers
/// can perform entity lookups, permission checks, and targeted actions.
#[derive(Debug, Clone)]
pub struct CommandContext {
    /// The entity ID of the caller, if they are in-world. `None` for console commands.
    pub caller_entity_id: Option<EntityId>,
    /// Display name of the caller (character name or "Console").
    pub caller_name: String,
    /// The caller's access level, used for permission checks.
    pub access_level: AccessLevel,
}

/// Registry of GM/admin commands.
///
/// Holds all registered commands and dispatches input strings to the
/// appropriate handler after verifying access permissions.
pub struct CommandRegistry {
    commands: HashMap<String, CommandInfo>,
}

impl CommandRegistry {
    /// Create a new, empty command registry.
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a new command.
    ///
    /// # Arguments
    ///
    /// * `name` - The command name (without leading `/`).
    /// * `description` - A short description for help listings.
    /// * `usage` - Usage string (e.g., "/teleport <player> <x> <y> <z>").
    /// * `min_access` - Minimum access level required to execute.
    /// * `handler` - The function that implements the command logic.
    ///
    /// If a command with the same name already exists, it will be replaced
    /// and a warning will be logged.
    pub fn register(
        &mut self,
        name: &str,
        description: &str,
        usage: &str,
        min_access: AccessLevel,
        handler: CommandHandler,
    ) {
        let lower_name = name.to_lowercase();

        if self.commands.contains_key(&lower_name) {
            tracing::warn!(command = %lower_name, "Replacing existing command registration");
        }

        let info = CommandInfo {
            name: lower_name.clone(),
            description: description.to_string(),
            usage: usage.to_string(),
            min_access_level: min_access,
            handler,
        };

        self.commands.insert(lower_name, info);
    }

    /// Parse and execute a command string.
    ///
    /// The input is parsed to extract the command name and arguments,
    /// the caller's access level is checked against the command's requirement,
    /// and the handler is invoked.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The execution context (who is running the command).
    /// * `input` - The raw command string (e.g., "/teleport player1 100 200 300").
    pub fn execute(&self, ctx: &CommandContext, input: &str) -> CommandResult {
        let (command_name, args) = match parser::parse_command(input) {
            Some(parsed) => parsed,
            None => return CommandResult::Error("Empty command.".to_string()),
        };

        let lower_name = command_name.to_lowercase();

        let info = match self.commands.get(&lower_name) {
            Some(info) => info,
            None => {
                tracing::debug!(
                    command = %lower_name,
                    caller = %ctx.caller_name,
                    "Unknown command"
                );
                return CommandResult::Error(format!("Unknown command: {}", lower_name));
            }
        };

        if !ctx.access_level.can_execute(info.min_access_level) {
            tracing::warn!(
                command = %lower_name,
                caller = %ctx.caller_name,
                caller_level = ?ctx.access_level,
                required_level = ?info.min_access_level,
                "Permission denied for command"
            );
            return CommandResult::Error(format!(
                "Permission denied. '{}' requires {} access.",
                lower_name, info.min_access_level
            ));
        }

        tracing::info!(
            command = %lower_name,
            caller = %ctx.caller_name,
            args = ?args,
            "Executing command"
        );

        (info.handler)(ctx, &args)
    }

    /// List all commands that the given access level can execute.
    ///
    /// Returns references to `CommandInfo` sorted alphabetically by name.
    pub fn list_commands(&self, access_level: AccessLevel) -> Vec<&CommandInfo> {
        let mut commands: Vec<&CommandInfo> = self
            .commands
            .values()
            .filter(|info| access_level.can_execute(info.min_access_level))
            .collect();

        commands.sort_by(|a, b| a.name.cmp(&b.name));
        commands
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context(access: AccessLevel) -> CommandContext {
        CommandContext {
            caller_entity_id: Some(EntityId(1)),
            caller_name: "TestUser".to_string(),
            access_level: access,
        }
    }

    fn make_handler(response: &'static str) -> CommandHandler {
        Box::new(move |_ctx, _args| CommandResult::Success(response.to_string()))
    }

    fn echo_handler() -> CommandHandler {
        Box::new(|_ctx, args| CommandResult::Success(args.join(" ")))
    }

    #[test]
    fn register_and_execute() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "ping",
            "Replies with pong",
            "/ping",
            AccessLevel::Player,
            make_handler("pong"),
        );

        let ctx = test_context(AccessLevel::Player);
        let result = registry.execute(&ctx, "/ping");
        assert_eq!(result, CommandResult::Success("pong".to_string()));
    }

    #[test]
    fn execute_passes_args() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "echo",
            "Echoes input",
            "/echo <text>",
            AccessLevel::Player,
            echo_handler(),
        );

        let ctx = test_context(AccessLevel::Player);
        let result = registry.execute(&ctx, "/echo hello world");
        assert_eq!(result, CommandResult::Success("hello world".to_string()));
    }

    #[test]
    fn execute_unknown_command() {
        let registry = CommandRegistry::new();
        let ctx = test_context(AccessLevel::Developer);
        let result = registry.execute(&ctx, "/nonexistent");
        match result {
            CommandResult::Error(msg) => assert!(msg.contains("Unknown command")),
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn execute_empty_input() {
        let registry = CommandRegistry::new();
        let ctx = test_context(AccessLevel::Developer);
        let result = registry.execute(&ctx, "");
        match result {
            CommandResult::Error(msg) => assert!(msg.contains("Empty")),
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn execute_permission_denied() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "admin_cmd",
            "Admin only",
            "/admin_cmd",
            AccessLevel::Admin,
            make_handler("secret"),
        );

        let ctx = test_context(AccessLevel::Player);
        let result = registry.execute(&ctx, "/admin_cmd");
        match result {
            CommandResult::Error(msg) => assert!(msg.contains("Permission denied")),
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn execute_permission_granted_higher_level() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "mod_cmd",
            "Moderator command",
            "/mod_cmd",
            AccessLevel::Moderator,
            make_handler("ok"),
        );

        let ctx = test_context(AccessLevel::Admin);
        let result = registry.execute(&ctx, "/mod_cmd");
        assert_eq!(result, CommandResult::Success("ok".to_string()));
    }

    #[test]
    fn command_name_case_insensitive() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "Help",
            "Shows help",
            "/help",
            AccessLevel::Player,
            make_handler("help text"),
        );

        let ctx = test_context(AccessLevel::Player);
        let result = registry.execute(&ctx, "/HELP");
        assert_eq!(result, CommandResult::Success("help text".to_string()));
    }

    #[test]
    fn list_commands_filters_by_access() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "help",
            "Shows help",
            "/help",
            AccessLevel::Player,
            make_handler(""),
        );
        registry.register(
            "kick",
            "Kick player",
            "/kick <player>",
            AccessLevel::Moderator,
            make_handler(""),
        );
        registry.register(
            "shutdown",
            "Shutdown server",
            "/shutdown",
            AccessLevel::Admin,
            make_handler(""),
        );

        let player_cmds = registry.list_commands(AccessLevel::Player);
        assert_eq!(player_cmds.len(), 1);
        assert_eq!(player_cmds[0].name, "help");

        let mod_cmds = registry.list_commands(AccessLevel::Moderator);
        assert_eq!(mod_cmds.len(), 2);

        let admin_cmds = registry.list_commands(AccessLevel::Admin);
        assert_eq!(admin_cmds.len(), 3);
    }

    #[test]
    fn list_commands_sorted_alphabetically() {
        let mut registry = CommandRegistry::new();
        registry.register("zebra", "", "", AccessLevel::Player, make_handler(""));
        registry.register("alpha", "", "", AccessLevel::Player, make_handler(""));
        registry.register("middle", "", "", AccessLevel::Player, make_handler(""));

        let cmds = registry.list_commands(AccessLevel::Player);
        let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);
    }

    #[test]
    fn console_context_no_entity_id() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "status",
            "Server status",
            "/status",
            AccessLevel::Admin,
            Box::new(|ctx, _args| {
                if ctx.caller_entity_id.is_none() {
                    CommandResult::Success("Console caller".to_string())
                } else {
                    CommandResult::Success("In-world caller".to_string())
                }
            }),
        );

        let ctx = CommandContext {
            caller_entity_id: None,
            caller_name: "Console".to_string(),
            access_level: AccessLevel::Developer,
        };
        let result = registry.execute(&ctx, "/status");
        assert_eq!(result, CommandResult::Success("Console caller".to_string()));
    }
}
