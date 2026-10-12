//! `lab version`: the CLI's version and the daemon's.

use crate::commands::Ctx;
use crate::daemon::status::{self, Endpoint, StatusReply};
use crate::ui::{Mark, Tone, Ui};

/// The daemon line. Plain: `daemon <version>`, or `daemon: not running`.
/// Fancy: `daemon  ● <version>`, or `daemon  ○ not running`.
pub fn daemon_line(ui: &Ui, reply: Option<&StatusReply>) -> String {
    match (reply, ui.fancy) {
        (Some(reply), false) => format!("daemon {}", reply.daemon.version),
        (Some(reply), true) => format!(
            "daemon  {} {}",
            ui.paint(Tone::Ok, ui.mark(Mark::Running)),
            reply.daemon.version
        ),
        (None, false) => "daemon: not running".to_string(),
        (None, true) => format!(
            "daemon  {} {}",
            ui.paint(Tone::Fail, ui.mark(Mark::Stopped)),
            ui.paint(Tone::Dim, "not running")
        ),
    }
}

/// Prints `lab <version> (<git sha or "dev">)` (the sha is set by `lab install`),
/// then the daemon line from `GET /status`. Always exits 0.
pub fn run(ctx: &Ctx) -> u8 {
    let sha = option_env!("CIMMERIA_LAB_CLI_SHA").unwrap_or("dev");
    anstream::println!("lab {} ({sha})", env!("CARGO_PKG_VERSION"));
    let reply = status::fetch(&Endpoint::discover());
    anstream::println!("{}", daemon_line(&ctx.ui, reply.as_ref()));
    0
}
