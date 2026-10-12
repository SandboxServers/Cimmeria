//! The command table, the clap surface and the command dispatch.
//!
//! Every command has its clap arguments and help page in its own file. Only
//! `version` does its own work so far; the others call [`crate::fallback`]
//! (the PowerShell dispatcher) until their packet ports the body.

pub mod calibrate;
pub mod clients;
pub mod doctor;
pub mod env;
pub mod golden;
pub mod install;
pub mod instances;
pub mod local_setup;
pub mod logs;
pub mod record;
pub mod restart;
pub mod setup;
pub mod start;
pub mod status;
pub mod stop;
pub mod uat;
pub mod version;
pub mod watch;

use std::ffi::OsString;

use anstyle::{AnsiColor, Style};
use clap::builder::Styles;
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
    Uat(uat::Args),
    Watch(watch::Args),
    Record(record::Args),
    Calibrate(calibrate::Args),
    Golden(golden::Args),
    Status(status::Args),
    Start(start::Args),
    Stop(stop::Args),
    Restart(restart::Args),
    Logs(logs::Args),
    Doctor(doctor::Args),
    Instances(instances::Args),
    Clients(clients::Args),
    Env(env::Args),
    Install(install::Args),
    Setup(setup::Args),
    LocalSetup(local_setup::Args),
    /// Print the CLI and daemon versions.
    Version,
}

/// Runs a parsed command and returns its exit code.
pub fn dispatch(cmd: &Command, ctx: &Ctx) -> u8 {
    match cmd {
        Command::Uat(args) => uat::run(args, ctx),
        Command::Watch(args) => watch::run(args, ctx),
        Command::Record(args) => record::run(args, ctx),
        Command::Calibrate(args) => calibrate::run(args, ctx),
        Command::Golden(args) => golden::run(args, ctx),
        Command::Status(args) => status::run(args, ctx),
        Command::Start(args) => start::run(args, ctx),
        Command::Stop(args) => stop::run(args, ctx),
        Command::Restart(args) => restart::run(args, ctx),
        Command::Logs(args) => logs::run(args, ctx),
        Command::Doctor(args) => doctor::run(args, ctx),
        Command::Instances(args) => instances::run(args, ctx),
        Command::Clients(args) => clients::run(args, ctx),
        Command::Env(args) => env::run(args, ctx),
        Command::Install(args) => install::run(args, ctx),
        Command::Setup(args) => setup::run(args, ctx),
        Command::LocalSetup(args) => local_setup::run(args, ctx),
        Command::Version => version::run(ctx),
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

    /// COMMANDS and the clap subcommands list the same names, except
    /// `completions`, which LX-18 adds.
    #[test]
    fn command_table_and_clap_list_the_same_names() {
        let mut clap_names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        let mut table_names: Vec<String> = COMMANDS
            .iter()
            .map(|c| c.name.to_string())
            .filter(|n| n != "completions")
            .collect();
        clap_names.sort();
        table_names.sort();
        assert_eq!(clap_names, table_names);
    }

    /// A `///` doc comment is not a Rust string, so a doubled backslash in an
    /// argument's help prints as two: no help page may show one.
    #[test]
    fn help_pages_have_no_doubled_backslashes() {
        for mut sub in Cli::command().get_subcommands().cloned() {
            let page = sub.render_long_help().to_string();
            assert!(
                !page.contains("\\\\"),
                "{}: doubled backslash in\n{page}",
                sub.get_name()
            );
        }
    }

    /// Splits an example line on spaces, keeping a 'single quoted' word whole.
    fn shell_words(line: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut word = String::new();
        let mut quoted = false;
        for c in line.chars() {
            match c {
                '\'' => quoted = !quoted,
                ' ' if !quoted => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                }
                _ => word.push(c),
            }
        }
        if !word.is_empty() {
            words.push(word);
        }
        words
    }

    /// Every `lab ...` line in a help page's Examples section parses in the
    /// `--flag` form it shows, and names the command whose page it is on.
    #[test]
    fn every_help_example_parses() {
        let mut checked = 0;
        for sub in Cli::command().get_subcommands() {
            let Some(after) = sub.get_after_help() else {
                assert_eq!(
                    sub.get_name(),
                    "version",
                    "no Examples on {}",
                    sub.get_name()
                );
                continue;
            };
            let after = after.to_string();
            let lines: Vec<&str> = after.lines().skip(1).collect();
            assert!(!lines.is_empty(), "{}: no example lines", sub.get_name());
            for line in lines {
                let words = shell_words(line.trim());
                assert_eq!(words[0], "lab", "{line}");
                assert_eq!(words[1], sub.get_name(), "{line}");
                let parsed = Cli::try_parse_from(words.iter().map(String::as_str));
                if let Err(err) = parsed {
                    panic!("example `{line}` does not parse: {err}");
                }
                checked += 1;
            }
        }
        assert!(checked >= 35, "only {checked} examples were checked");
    }

    /// No line of any help page's Examples section is wider than 76 columns,
    /// so the pages fit a 80-column terminal with room to spare.
    #[test]
    fn help_examples_fit_76_columns() {
        for sub in Cli::command().get_subcommands() {
            let Some(after) = sub.get_after_help() else {
                continue;
            };
            for line in after.to_string().lines() {
                assert!(
                    line.chars().count() <= 76,
                    "{}: {} columns in `{line}`",
                    sub.get_name(),
                    line.chars().count()
                );
            }
        }
    }
}
