//! The command table, the clap surface and the command dispatch.
//!
//! LX-01 has one real command, `version`. Every other name goes to the
//! PowerShell dispatcher through [`crate::fallback`] until its packet moves it
//! here, one file per command.

pub mod clients;
pub mod doctor;
pub mod env;
pub mod instances;
pub mod logs;
pub mod restart;
pub mod start;
pub mod status;
pub mod stop;
pub mod version;

use std::ffi::OsString;

use anstyle::{AnsiColor, Style};
use clap::builder::Styles;
use clap::{Parser, Subcommand};

use crate::fallback;
use crate::ui::Ui;

/// What a command gets: the terminal kit, its name and its raw arguments.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// The terminal kit for this run.
    pub ui: Ui,
    /// The command name, as typed.
    pub command: String,
    /// The arguments after the command name, exactly as typed.
    pub raw: Vec<OsString>,
}

/// The group a command sits under in the root help.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// Run and record UAT.
    RunAndRecord,
    /// The lab daemon.
    Daemon,
    /// Instances and clients.
    InstancesAndClients,
    /// Install and configure.
    InstallAndConfigure,
}

impl Group {
    /// Every group, in the order the root help prints them.
    pub const ALL: [Group; 4] = [
        Group::RunAndRecord,
        Group::Daemon,
        Group::InstancesAndClients,
        Group::InstallAndConfigure,
    ];

    /// The heading the root help prints for this group.
    pub fn title(self) -> &'static str {
        match self {
            Group::RunAndRecord => "Run and record UAT",
            Group::Daemon => "Daemon",
            Group::InstancesAndClients => "Instances and clients",
            Group::InstallAndConfigure => "Install and configure",
        }
    }
}

/// One row of the command table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandInfo {
    /// The name typed after `lab`.
    pub name: &'static str,
    /// The root-help group.
    pub group: Group,
    /// One line for the root help.
    pub summary: &'static str,
}

/// Every `lab` command, in root-help order.
pub const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "uat",
        group: Group::RunAndRecord,
        summary: "Run a UAT spec on the lab with no agent",
    },
    CommandInfo {
        name: "watch",
        group: Group::RunAndRecord,
        summary: "Watch a running lab uat batch, read-only",
    },
    CommandInfo {
        name: "record",
        group: Group::RunAndRecord,
        summary: "Record a person playing a lab instance and draft a spec from it",
    },
    CommandInfo {
        name: "calibrate",
        group: Group::RunAndRecord,
        summary: "Recalibrate a row's poses by hover and propose a spec diff",
    },
    CommandInfo {
        name: "golden",
        group: Group::RunAndRecord,
        summary: "Record or show a golden fingerprint",
    },
    CommandInfo {
        name: "status",
        group: Group::Daemon,
        summary: "Show the daemon and each instance: account, client, port, lease, seed",
    },
    CommandInfo {
        name: "start",
        group: Group::Daemon,
        summary: "Start the lab daemon (the CimmeriaLabDaemon scheduled task)",
    },
    CommandInfo {
        name: "stop",
        group: Group::Daemon,
        summary: "Stop the lab daemon and close its lab clients",
    },
    CommandInfo {
        name: "restart",
        group: Group::Daemon,
        summary: "Restart the lab daemon, picking up a newer cimmeria-lab.exe",
    },
    CommandInfo {
        name: "logs",
        group: Group::Daemon,
        summary: "Show the daemon log, filtered by instance or level",
    },
    CommandInfo {
        name: "doctor",
        group: Group::Daemon,
        summary: "Check the install, the daemon and each instance",
    },
    CommandInfo {
        name: "instances",
        group: Group::InstancesAndClients,
        summary: "Show each instance's account, profile and client, or write account files",
    },
    CommandInfo {
        name: "clients",
        group: Group::InstancesAndClients,
        summary: "Close the game client of one lab instance, or of all of them",
    },
    CommandInfo {
        name: "env",
        group: Group::InstallAndConfigure,
        summary: "Show, get, set or unset labd.env entries (secrets masked)",
    },
    CommandInfo {
        name: "install",
        group: Group::InstallAndConfigure,
        summary: "Build the lab from a worktree and install it",
    },
    CommandInfo {
        name: "setup",
        group: Group::InstallAndConfigure,
        summary: "Refresh the installed CLI and add it to the user PATH",
    },
    CommandInfo {
        name: "local-setup",
        group: Group::InstallAndConfigure,
        summary: "One-shot setup for a lab against a local server",
    },
    CommandInfo {
        name: "version",
        group: Group::InstallAndConfigure,
        summary: "Print the CLI and daemon versions",
    },
    CommandInfo {
        name: "completions",
        group: Group::InstallAndConfigure,
        summary: "Print a shell completion script",
    },
];

/// The command table entry for `name`, if there is one.
pub fn find(name: &str) -> Option<&'static CommandInfo> {
    COMMANDS.iter().find(|info| info.name == name)
}

/// The clap help styles: headings bold yellow, literals cyan, placeholders dim.
pub fn styles() -> Styles {
    let heading = Style::new().bold().fg_color(Some(AnsiColor::Yellow.into()));
    Styles::styled()
        .header(heading)
        .usage(heading)
        .literal(Style::new().fg_color(Some(AnsiColor::Cyan.into())))
        .placeholder(Style::new().dimmed())
}

/// The clap surface of `lab`.
#[derive(Debug, Parser)]
#[command(name = "lab", disable_help_subcommand = true, styles = styles())]
pub struct Cli {
    /// The subcommand, if any.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// The subcommands clap knows about.
#[derive(Debug, Subcommand)]
pub enum Command {
    Status(status::Args),
    Start(start::Args),
    Stop(stop::Args),
    Restart(restart::Args),
    Logs(logs::Args),
    Doctor(doctor::Args),
    Instances(instances::Args),
    Clients(clients::Args),
    Env(env::Args),
    /// Print the CLI and daemon versions.
    Version,
    /// A command that has not moved to clap yet: the PowerShell dispatcher
    /// handles it. LX-02b deletes this variant.
    #[command(external_subcommand)]
    Other(Vec<OsString>),
}

/// Runs a parsed command and returns its exit code.
pub fn dispatch(cmd: &Command, ctx: &Ctx) -> u8 {
    match cmd {
        Command::Status(args) => status::run(args, ctx),
        Command::Start(args) => start::run(args, ctx),
        Command::Stop(args) => stop::run(args, ctx),
        Command::Restart(args) => restart::run(args, ctx),
        Command::Logs(args) => logs::run(args, ctx),
        Command::Doctor(args) => doctor::run(args, ctx),
        Command::Instances(args) => instances::run(args, ctx),
        Command::Clients(args) => clients::run(args, ctx),
        Command::Env(args) => env::run(args, ctx),
        Command::Version => version::run(),
        Command::Other(_) => fallback::run(ctx),
    }
}

/// Parses `lab <command> <tokens...>` after `argv::rewrite`, as `main_with` does.
#[cfg(test)]
pub(crate) fn try_parse(command: &str, tokens: &[&str]) -> Result<Cli, clap::Error> {
    let mut args = vec![OsString::from(command)];
    args.extend(tokens.iter().map(|t| OsString::from(*t)));
    let rewritten = crate::argv::rewrite(args);
    Cli::try_parse_from(std::iter::once(OsString::from("lab")).chain(rewritten))
}

/// [`try_parse`] that panics with clap's message when the tokens do not parse.
#[cfg(test)]
pub(crate) fn parse_ok(command: &str, tokens: &[&str]) -> Command {
    match try_parse(command, tokens) {
        Ok(cli) => cli.command.expect("a subcommand"),
        Err(err) => panic!("`lab {command} {tokens:?}` should parse: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    /// Every clap subcommand is a command in the table (the table may hold
    /// more until LX-02b converts the rest).
    #[test]
    fn clap_subcommands_are_in_the_command_table() {
        for sub in Cli::command().get_subcommands() {
            assert!(
                find(sub.get_name()).is_some(),
                "clap subcommand '{}' is missing from COMMANDS",
                sub.get_name()
            );
        }
    }

    #[test]
    fn converted_commands_are_clap_subcommands() {
        let names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        for want in [
            "status",
            "start",
            "stop",
            "restart",
            "logs",
            "doctor",
            "instances",
            "clients",
            "env",
            "version",
        ] {
            assert!(
                names.iter().any(|n| n == want),
                "no clap subcommand '{want}'"
            );
        }
    }
}
