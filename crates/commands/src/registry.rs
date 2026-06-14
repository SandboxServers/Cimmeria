use std::collections::HashMap;

use cimmeria_common::types::EntityId;

use crate::parser;
use crate::permissions::AccessLevel;

/// The outcome of dispatching a `/`-prefixed command on the base side.
///
/// The base PARSES + AUTHORIZES; the cell EXECUTES. A handler that needs to
/// mutate world state returns [`CommandOutcome::Cell`] carrying a typed
/// [`crate::intent::GmCommandIntent`] — the base forwards it to the cell and
/// the cell owns every client-method send (including the GM's feedback line).
/// The text variants ([`Reply`](CommandOutcome::Reply),
/// [`Usage`](CommandOutcome::Usage), [`Error`](CommandOutcome::Error)) are
/// fully handled base-side and route a single feedback line back to the caller.
///
/// Not `Eq`: the [`Cell`](CommandOutcome::Cell) variant carries a
/// [`GmCommandIntent`](crate::intent::GmCommandIntent) which holds an
/// `f32`-backed `Vector3` (`GotoCoords`), so only `PartialEq` is derivable.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandOutcome {
    /// World-mutating command — the cell executes the carried intent.
    Cell(crate::intent::GmCommandIntent),
    /// Fully handled base-side; text back to the caller (e.g. `/help`).
    Reply(String),
    /// Bad/missing args; the command's usage string back to the caller.
    Usage(String),
    /// Unknown command / permission denied / other error.
    Error(String),
}

/// A boxed command handler function.
///
/// Receives the execution context and a slice of parsed arguments,
/// and returns a [`CommandOutcome`].
pub type CommandHandler = Box<dyn Fn(&CommandContext, &[&str]) -> CommandOutcome + Send + Sync>;

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

    /// Parse and dispatch a command string.
    ///
    /// The input is parsed to extract the command name and arguments,
    /// the caller's access level is checked against the command's requirement,
    /// and the handler is invoked.
    ///
    /// On an unknown command or denied permission, returns
    /// [`CommandOutcome::Error`] — the base maps that to a feedback line.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The execution context (who is running the command).
    /// * `input` - The raw command string (e.g., "/goto 100 200 300").
    pub fn dispatch(&self, ctx: &CommandContext, input: &str) -> CommandOutcome {
        let (command_name, args) = match parser::parse_command(input) {
            Some(parsed) => parsed,
            None => return CommandOutcome::Error("Empty command.".to_string()),
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
                return CommandOutcome::Error(format!("Unknown command: {}", lower_name));
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
            return CommandOutcome::Error(format!(
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
        Box::new(move |_ctx, _args| CommandOutcome::Reply(response.to_string()))
    }

    fn echo_handler() -> CommandHandler {
        Box::new(|_ctx, args| CommandOutcome::Reply(args.join(" ")))
    }

    #[test]
    fn register_and_dispatch() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "ping",
            "Replies with pong",
            "/ping",
            AccessLevel::Player,
            make_handler("pong"),
        );

        let ctx = test_context(AccessLevel::Player);
        let result = registry.dispatch(&ctx, "/ping");
        assert_eq!(result, CommandOutcome::Reply("pong".to_string()));
    }

    #[test]
    fn dispatch_passes_args() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "echo",
            "Echoes input",
            "/echo <text>",
            AccessLevel::Player,
            echo_handler(),
        );

        let ctx = test_context(AccessLevel::Player);
        let result = registry.dispatch(&ctx, "/echo hello world");
        assert_eq!(result, CommandOutcome::Reply("hello world".to_string()));
    }

    #[test]
    fn dispatch_unknown_command() {
        let registry = CommandRegistry::new();
        let ctx = test_context(AccessLevel::Developer);
        let result = registry.dispatch(&ctx, "/nonexistent");
        match result {
            CommandOutcome::Error(msg) => assert!(msg.contains("Unknown command")),
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn dispatch_empty_input() {
        let registry = CommandRegistry::new();
        let ctx = test_context(AccessLevel::Developer);
        let result = registry.dispatch(&ctx, "");
        match result {
            CommandOutcome::Error(msg) => assert!(msg.contains("Empty")),
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn dispatch_permission_denied() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "admin_cmd",
            "Admin only",
            "/admin_cmd",
            AccessLevel::Admin,
            make_handler("secret"),
        );

        let ctx = test_context(AccessLevel::Player);
        let result = registry.dispatch(&ctx, "/admin_cmd");
        match result {
            CommandOutcome::Error(msg) => assert!(msg.contains("Permission denied")),
            _ => panic!("Expected Error result"),
        }
    }

    #[test]
    fn dispatch_permission_granted_higher_level() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "mod_cmd",
            "Moderator command",
            "/mod_cmd",
            AccessLevel::Moderator,
            make_handler("ok"),
        );

        let ctx = test_context(AccessLevel::Admin);
        let result = registry.dispatch(&ctx, "/mod_cmd");
        assert_eq!(result, CommandOutcome::Reply("ok".to_string()));
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
        let result = registry.dispatch(&ctx, "/HELP");
        assert_eq!(result, CommandOutcome::Reply("help text".to_string()));
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
                    CommandOutcome::Reply("Console caller".to_string())
                } else {
                    CommandOutcome::Reply("In-world caller".to_string())
                }
            }),
        );

        let ctx = CommandContext {
            caller_entity_id: None,
            caller_name: "Console".to_string(),
            access_level: AccessLevel::Developer,
        };
        let result = registry.dispatch(&ctx, "/status");
        assert_eq!(result, CommandOutcome::Reply("Console caller".to_string()));
    }
}
