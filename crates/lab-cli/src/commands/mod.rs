//! The command table, the clap surface and the command dispatch.
//!
//! LX-01 has one real command, `version`. Every other name goes to the
//! PowerShell dispatcher through [`crate::fallback`] until its packet moves it
//! here, one file per command.

pub mod version;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

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

/// The clap surface of `lab`. Only `version` is parsed by clap for now.
#[derive(Debug, Parser)]
#[command(name = "lab", disable_help_subcommand = true)]
pub struct Cli {
    /// The subcommand, if any.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// The subcommands clap knows about.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print the CLI and daemon versions.
    Version,
    /// Any other command: the PowerShell dispatcher handles it.
    #[command(external_subcommand)]
    Other(Vec<OsString>),
}
