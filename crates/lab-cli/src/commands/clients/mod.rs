//! `lab clients`: close the game client of one lab instance, or of all of them.
//! LX-09 ported the removed PowerShell clients command: [`targets`] decides who is
//! stopped or refused, [`win`] stops one pid, and [`execute`] ties them
//! together (the daemon status and the stopper are injected, so the tests need
//! neither a daemon nor a game client).

use crate::commands::Ctx;
use crate::daemon::status::{self, Endpoint, StatusReply};
use crate::ui::Ui;

mod targets;
mod win;

#[cfg(test)]
mod flow_tests;

pub use win::StopOutcome;

const LONG: &str = concat!(
    "Usage: lab clients stop [<instance>|all] [--force].",
    "\n\n",
    "Asks each client window to close (CloseMainWindow) and force-stops it when it is still alive after 8 s. Only a process named SGW.exe is touched, and the pid comes from the daemon's GET /status, read again right before each stop (a client that was relaunched or leased meanwhile is left alone).",
    "\n\n",
    "An instance a session holds (a lease) is not stopped, because its holder would lose the client. --force overrides that, for one named instance only: 'all' with --force is refused. Exit codes: 0 when nothing was refused, 1 when the daemon is not running, 2 on a usage error, 3 when a leased client was refused, 4 when a client could not be stopped (including one that was relaunched since the first status read).",
);

const EXAMPLES: &str = "Examples:\n  lab clients stop p2\n  lab clients stop p2 --force";

// Arguments of `lab clients`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Close the game client of one lab instance, or of all of them",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// What to do; only 'stop' exists
    pub verb: Option<String>,
    /// An instance name, or 'all'
    pub which: Option<String>,
    /// Stop the client of a leased instance too (one named instance only; 'all' with --force is refused)
    #[arg(long)]
    pub force: bool,
}

/// Which output stream a line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    /// Standard output.
    Out,
    /// Standard error.
    Err,
}

/// Runs `lab clients ...`: asks the daemon for its status, stops the clients
/// that may be stopped, prints one line per instance. Returns the exit code.
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    // A closed pipe must not abort the run half way: a panic here would leave
    // later clients unstopped and the exit code 101. Write errors are ignored.
    let mut emit = |stream: Stream, line: String| {
        use std::io::Write;
        let _ = match stream {
            Stream::Out => writeln!(anstream::stdout(), "{line}"),
            Stream::Err => writeln!(anstream::stderr(), "{line}"),
        };
    };
    let endpoint = Endpoint::discover();
    execute(
        args,
        &ctx.ui,
        || status::fetch(&endpoint),
        &mut |pid| win::stop_client(pid),
        &mut emit,
    )
}

/// The command with its three outside effects injected: `fetch` asks the
/// daemon (called only after every usage check, and again right before each
/// stop so a client that was relaunched or leased meanwhile is left alone),
/// `stopper` stops one pid, and `emit` receives each output line as it is
/// produced.
pub fn execute(
    args: &Args,
    ui: &Ui,
    mut fetch: impl FnMut() -> Option<StatusReply>,
    stopper: &mut dyn FnMut(u32) -> Result<StopOutcome, String>,
    emit: &mut dyn FnMut(Stream, String),
) -> u8 {
    let fail = |emit: &mut dyn FnMut(Stream, String), message: &str, hint: Option<&str>| {
        for line in targets::error_lines(ui, message, hint) {
            emit(Stream::Err, line);
        }
    };
    let is_stop = args
        .verb
        .as_deref()
        .is_some_and(|verb| verb.eq_ignore_ascii_case("stop"));
    if !is_stop {
        fail(emit, targets::USAGE, None);
        return 2;
    }
    let which = args.which.as_deref();
    let all = targets::is_all(which);
    // Refused before the daemon is asked, so nothing is stopped.
    if args.force && all {
        fail(
            emit,
            targets::FORCE_NEEDS_NAME,
            Some("name the instance: lab clients stop p2 --force"),
        );
        return 2;
    }
    let Some(status) = fetch() else {
        fail(emit, targets::DAEMON_DOWN, Some("start it: lab start"));
        return 1;
    };
    if let Some(name) = which.filter(|_| !all) {
        let hosted: Vec<&str> = status
            .instances
            .iter()
            .map(|i| i.instance.as_str())
            .collect();
        if !hosted.iter().any(|h| targets::same_name(h, name)) {
            let message = format!(
                "unknown instance '{name}'; the daemon hosts: {}",
                hosted.join(", ")
            );
            fail(emit, &message, None);
            return 2;
        }
    }

    let sel = targets::select_stop_targets(&status, which, args.force);
    for inst in &sel.none {
        emit(Stream::Out, targets::none_line(ui, &inst.instance));
    }
    for inst in &sel.refuse {
        emit(
            Stream::Out,
            targets::refuse_line(ui, &inst.instance, &targets::lease_who(inst)),
        );
    }
    let mut failed = 0u32;
    let mut refused = sel.refuse.len();
    for inst in &sel.stop {
        let label = inst.instance.as_str();
        let Some(pid) = inst.client_pid else { continue };
        let forced = sel
            .forced
            .iter()
            .any(|f| targets::same_name(&f.instance, label));
        // The status above may be seconds old (earlier stops wait up to 10 s):
        // look again so a relaunched or newly leased client is not stopped.
        match targets::recheck(fetch().as_ref(), label, pid, args.force) {
            targets::Recheck::Proceed => {}
            targets::Recheck::Gone => {
                emit(Stream::Out, targets::none_line(ui, label));
                continue;
            }
            targets::Recheck::Changed(now) => {
                failed += 1;
                emit(Stream::Err, targets::changed_line(ui, label, pid, now));
                continue;
            }
            targets::Recheck::Leased(who) => {
                refused += 1;
                emit(Stream::Out, targets::refuse_line(ui, label, &who));
                continue;
            }
            targets::Recheck::Unknown => {
                failed += 1;
                emit(Stream::Err, targets::unchecked_line(ui, label, pid));
                continue;
            }
        }
        if forced {
            emit(
                Stream::Out,
                targets::forced_line(ui, label, &targets::lease_who(inst)),
            );
        }
        match stopper(pid) {
            Ok(StopOutcome::Stopped) => emit(Stream::Out, targets::stopped_line(ui, label, pid)),
            Ok(StopOutcome::NotSgw) => emit(Stream::Out, targets::not_sgw_line(ui, label, pid)),
            Ok(StopOutcome::Failed) => {
                failed += 1;
                emit(Stream::Err, targets::failed_line(ui, label, pid, None));
            }
            Err(reason) => {
                failed += 1;
                emit(
                    Stream::Err,
                    targets::failed_line(ui, label, pid, Some(&reason)),
                );
            }
        }
    }
    if failed > 0 {
        4
    } else if refused > 0 {
        3
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    /// The `.EXAMPLE` lines of the removed PowerShell clients command parse, with the right values.
    #[test]
    fn examples_parse() {
        let Command::Clients(a) = parse_ok("clients", &["stop", "p2"]) else {
            panic!("not clients")
        };
        assert_eq!(a.verb.as_deref(), Some("stop"));
        assert_eq!(a.which.as_deref(), Some("p2"));
        assert!(!a.force);
        let Command::Clients(a) = parse_ok("clients", &["stop", "p2", "-Force"]) else {
            panic!("not clients")
        };
        assert!(a.force);
    }
}
