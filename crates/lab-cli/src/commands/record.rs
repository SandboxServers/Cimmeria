//! `lab record`: record a person playing a lab instance and draft a spec from it.
//! LX-02 gives it its clap arguments and help page; the body is still the
//! PowerShell fallback.

use crate::commands::Ctx;
use crate::fallback;

const LONG: &str = concat!(
    "start <section> calls lab_record_start on the shared lab daemon: the daemon samples the instance every 500 ms while a person plays it by hand (it only reads), under a lease of its own, so no agent drives that client meanwhile. It stops by itself after --max-minutes (default and at most 30) or when its lease is taken over. The recording id and instance go to %LOCALAPPDATA%\\cimmeria-lab\\record-<instance>.json, so stop needs no id.",
    "\n\n",
    "status calls lab_record_status: the instance's recording, its age, ticks and anchors (interact, useItem, moveItem and useAbility calls seen).",
    "\n\n",
    "stop calls lab_record_stop with the saved id: the recording ends, its raw log is kept under <uat root>\\records\\<recording id>\\record.jsonl, and a draft spec is written to <specs>\\drafts\\<section>-<yyyyMMdd-HHmm>.toml, never over an existing file. --character-expr replaces the recorded character's name in the draft's SQL. --no-draft keeps the log only. The state file is deleted once the stop succeeds; on a failure the recording stays on the daemon and the file stays, so run stop again (with other options if they were the problem). --recording-id stops a recording this machine did not start.",
    "\n\n",
    "transform <recording id or folder> calls lab_record_transform: the same draft again from a raw log already on disk, with other --section, --character-expr or --profile, as <name>-2.toml, -3, ... when that draft exists. It needs the daemon running, but no lease and no client. The folder must be under <uat root>\\records\\, and the recording stopped.",
    "\n\n",
    "--json prints only one compact JSON object: the tool's answer, or {\"ok\":false,\"exit\":N,\"error\":\"...\"} as the daemon returned it (exit 4 for a busy instance).",
    "\n\n",
    "Exit codes: 0 ok; 1 the tool failed; 2 usage, or refused with nothing changed; 3 no daemon; 4 the instance is busy (leased, or already recording).",
);

const EXAMPLES: &str = "Examples:\n  lab record start first-session --instance p2\n  lab record status --instance p2\n  lab record stop --instance p2 --character-expr 'Px${run_id}'\n  lab record transform 20261012-143000-p2 --section first-session --json";

// Arguments of `lab record`.
#[derive(clap::Args, Debug)]
#[command(
    about = "Record a person playing a lab instance and draft a spec from it",
    long_about = LONG,
    after_help = EXAMPLES
)]
pub struct Args {
    /// start, status, stop or transform
    pub action: Option<String>,
    /// start: the section; transform: a recording id or folder
    pub target: Option<String>,
    /// The lab instance (default: the first)
    #[arg(long)]
    pub instance: Option<String>,
    /// start: the character to record
    #[arg(long)]
    pub character: Option<String>,
    /// start: stop by itself after this many minutes (default and at most 30)
    #[arg(long)]
    pub max_minutes: Option<String>,
    /// Replace the recorded character's name in the draft's SQL (Px${run_id} for a fresh character per run)
    #[arg(long)]
    pub character_expr: Option<String>,
    /// transform: the profile for the draft
    #[arg(long)]
    pub profile: Option<String>,
    /// transform: the section for the draft
    #[arg(long)]
    pub section: Option<String>,
    /// The spec folder the draft goes under (default: the checkout's docs\\guides\\uat-specs; away from a checkout, the daemon's)
    #[arg(long)]
    pub specs_dir: Option<String>,
    /// stop: stop a recording this machine did not start (lab record status shows its id)
    #[arg(long)]
    pub recording_id: Option<String>,
    /// stop: keep the raw log only, write no draft
    #[arg(long)]
    pub no_draft: bool,
    /// Print one compact JSON object only
    #[arg(long)]
    pub json: bool,
}

/// Runs `lab record` (still the PowerShell fallback).
pub fn run(args: &Args, ctx: &Ctx) -> u8 {
    let _ = args;
    fallback::run(ctx)
}

#[cfg(test)]
mod tests {
    use crate::commands::{parse_ok, Command};

    fn record(tokens: &[&str]) -> super::Args {
        let Command::Record(args) = parse_ok("record", tokens) else {
            panic!("not record")
        };
        args
    }

    /// The four EXAMPLES lines parse, with the right values.
    #[test]
    fn examples_parse() {
        let a = record(&["start", "first-session", "-Instance", "p2"]);
        assert_eq!(a.action.as_deref(), Some("start"));
        assert_eq!(a.target.as_deref(), Some("first-session"));
        assert_eq!(a.instance.as_deref(), Some("p2"));

        let a = record(&["status", "-Instance", "p2"]);
        assert_eq!(a.action.as_deref(), Some("status"));
        assert_eq!(a.target, None);

        let a = record(&["stop", "-Instance", "p2", "-CharacterExpr", "Px${run_id}"]);
        assert_eq!(a.character_expr.as_deref(), Some("Px${run_id}"));

        let a = record(&[
            "transform",
            "20261012-143000-p2",
            "-Section",
            "first-session",
            "-Json",
        ]);
        assert_eq!(a.target.as_deref(), Some("20261012-143000-p2"));
        assert_eq!(a.section.as_deref(), Some("first-session"));
        assert!(a.json);
    }

    /// `--max-minutes` stays a string: the PowerShell prints its own message.
    #[test]
    fn max_minutes_stays_a_string() {
        let a = record(&["start", "first-session", "-MaxMinutes", "abc"]);
        assert_eq!(a.max_minutes.as_deref(), Some("abc"));
    }

    #[test]
    fn other_flags() {
        let a = record(&[
            "stop",
            "-NoDraft",
            "-RecordingId",
            "r1",
            "-SpecsDir",
            "C:\\s",
            "-Profile",
            "p",
            "-Character",
            "Bob",
        ]);
        assert!(a.no_draft);
        assert_eq!(a.recording_id.as_deref(), Some("r1"));
        assert_eq!(a.specs_dir.as_deref(), Some("C:\\s"));
        assert_eq!(a.profile.as_deref(), Some("p"));
        assert_eq!(a.character.as_deref(), Some("Bob"));
    }
}
