//! A native `lab uat` batch: make the folder, write `plan.json`, run the lanes,
//! write `batch.json` and `summary.md` (also after Ctrl+C), print the result.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use serde_json::Value;

use crate::batch::{read_records, PlanLane, RunRecord, BATCH_JSONL};
use crate::commands::Ctx;
use crate::daemon::mcp::{self, McpClient, McpError};
use crate::daemon::paths;
use crate::daemon::status::Endpoint;
use crate::ui::Ui;

use super::chaos;
use super::ctrlc;
use super::golden::{self, DiffArgs};
use super::heal;
use super::json;
use super::lanes::{
    run_lanes, sleep_unless_stopped, CallFn, FinishFn, LaneConfig, LaneEnv, SleepFn, WaitFn,
};
use super::plan::{batch_folder_name, build_plan, create_batch, run_chaos, RunCall};
use super::preflight::{stop, Ready};
use super::progress::{
    fancy_summary, progress_mode, quiet_line, stdio_emit, Progress, ProgressMode,
};
use super::summary::{exit_code, merge, write_batch_summary, BatchMeta, Summary};
use super::Args;

/// What LX-15's ready pre-flight (`Ready`) gives a real run.
pub struct BatchInput {
    /// The UAT section.
    pub section: String,
    /// The row ids the plan call returned: what every run must report.
    pub row_ids: Vec<String>,
    /// The lanes, in lane order.
    pub lanes: Vec<PlanLane>,
    /// The spec folder, when one was found or given.
    pub specs_dir: Option<String>,
    /// The arguments every run shares (`RunCall::for_lane(instance)` per run).
    pub call: RunCall,
    /// The runner's chaos header from the plan call, else the typed one (LX-17b).
    pub chaos: Option<String>,
    /// `--diff-golden`, or implied by `--chaos` (LX-17b).
    pub diff_golden: bool,
}

impl BatchInput {
    /// What a ready pre-flight hands the batch.
    pub fn from_ready(ready: Ready) -> Self {
        Self {
            section: ready.section,
            row_ids: ready.rows,
            lanes: ready.lanes,
            specs_dir: ready.specs,
            call: ready.call,
            chaos: ready.chaos,
            diff_golden: ready.diff_golden,
        }
    }
}

/// The numbers and switches of a batch.
pub struct BatchOptions {
    /// Runs each lane does, one after another.
    pub runs_per_lease: u32,
    /// Seconds between lane starts.
    pub stagger_seconds: u32,
    /// Minutes before one run times out.
    pub run_timeout_minutes: u32,
    /// Failed runs in a row that stop a lane (0 never).
    pub max_consecutive_failures: u32,
    /// Where progress lines go.
    pub mode: ProgressMode,
    /// `--quiet`: the end of the batch is one line.
    pub quiet: bool,
    /// `--json`: the end of the batch is the compact object (`json::compact_json`).
    pub json: bool,
}

/// What a batch leaves behind.
pub struct BatchOutcome {
    pub summary: Summary,
    /// The Markdown summary (`\r\n` line ends).
    pub md: String,
    pub dir: PathBuf,
    /// `lab uat <section>`.
    pub title: String,
    pub exit: u8,
    /// Every record of `batch.jsonl` (the heal pass reads them).
    pub records: Vec<RunRecord>,
    /// When the batch started (the heal pass rewrites `batch.json` with it).
    pub started: DateTime<Local>,
}

/// The parts of a [`LaneEnv`] the caller supplies.
pub struct Hooks<'a> {
    /// One `lab_uat_run` call.
    pub call: &'a CallFn<'a>,
    /// Waits for an instance's lease to clear.
    pub wait_lease_free: &'a WaitFn<'a>,
    /// Sleeps (the stagger).
    pub sleep: &'a SleepFn<'a>,
    /// After each run: [`finish_with`] in a native batch.
    pub finish: &'a FinishFn<'a>,
    /// Set by Ctrl+C: no new run starts.
    pub stop: &'a AtomicBool,
}

/// Runs a batch under `root` (`batch-<yyyyMMdd-HHmmss>`): header, folder and
/// `plan.json` (before any lane starts), the lanes, then, always, the summary
/// files. An error is a message for exit 3: the folder or `plan.json` could not
/// be written (before any run), or the records or summary could not be read or
/// written after.
pub fn run_batch(
    input: &BatchInput,
    opts: &BatchOptions,
    ui: &Ui,
    root: &Path,
    hooks: &Hooks<'_>,
    emit: &(dyn Fn(&str) + Sync),
) -> Result<BatchOutcome, String> {
    let started = Local::now();
    let name = batch_folder_name(&started.naive_local());
    let progress = Progress::new(*ui, opts.mode, &input.lanes, opts.runs_per_lease, emit);
    progress.say(&progress.header(&input.section, &input.lanes, &input.row_ids, &name));

    // LX-15's plan.json writer: the folder and plan.json exist before any lane starts.
    let plan = build_plan(
        &input.section,
        &input.row_ids,
        &input.lanes,
        opts.runs_per_lease,
        started.with_timezone(&Utc),
        std::process::id(),
    );
    let dir = create_batch(root, &name, &plan).map_err(|failed| failed.message)?;

    let jsonl = dir.join(BATCH_JSONL);
    let cfg = LaneConfig {
        row_ids: &input.row_ids,
        runs_per_lease: opts.runs_per_lease,
        stagger_seconds: opts.stagger_seconds,
        run_timeout: Duration::from_secs(u64::from(opts.run_timeout_minutes) * 60),
        max_consecutive_failures: opts.max_consecutive_failures,
        jsonl: &jsonl,
    };
    let on_record = |rec: &RunRecord| progress.say(&progress.run_line(rec));
    let on_notice = |instance: &str, rest: &str| progress.say(&progress.notice(instance, rest));
    let env = LaneEnv {
        call: hooks.call,
        wait_lease_free: hooks.wait_lease_free,
        sleep: hooks.sleep,
        on_record: &on_record,
        on_notice: &on_notice,
        finish: hooks.finish,
        stop: hooks.stop,
    };
    run_lanes(&input.lanes, &cfg, &env);

    // Also after Ctrl+C: summarise whatever finished.
    let records =
        read_records(&jsonl).map_err(|e| format!("cannot read {}: {e}", jsonl.display()))?;
    let mut braked: Vec<String> = Vec::new();
    for rec in records.iter().filter(|r| r.braked) {
        if !braked.iter().any(|b| b.eq_ignore_ascii_case(&rec.instance)) {
            braked.push(rec.instance.clone());
        }
    }
    let mut summary = merge(&records, &input.lanes, &braked);
    summary.chaos = chaos::select_header(&records, input.chaos.as_deref());
    let title = format!("lab uat {}", input.section);
    let meta = BatchMeta {
        dir: &dir,
        title: &title,
        section: &input.section,
        row_ids: &input.row_ids,
        lane_count: input.lanes.len() as u32,
        runs_per_lease: opts.runs_per_lease,
        started,
    };
    let md = write_batch_summary(&summary, &meta, Local::now())
        .map_err(|e| format!("cannot write the summary in {}: {e}", dir.display()))?;
    let exit = exit_code(&summary);
    Ok(BatchOutcome {
        summary,
        md,
        dir,
        title,
        exit,
        records,
        started,
    })
}

/// Prints the end of a batch: under `--quiet` one line; in the fancy style a blank
/// line and the tables; else a blank line and the Markdown (as the PowerShell's
/// `Write-Host ''; Write-Host $md`).
pub fn print_outcome(ui: &Ui, opts: &BatchOptions, outcome: &BatchOutcome) {
    let dir = outcome.dir.display().to_string();
    if opts.json {
        // Write errors are ignored here too (a closed pipe), as below.
        let _ = writeln!(
            std::io::stdout().lock(),
            "{}",
            json::compact_json(&outcome.summary, &dir)
        );
        return;
    }
    // Write errors are ignored (`| head`, a closed pipe): `println!` would panic.
    let mut out = std::io::stdout().lock();
    let _ = if opts.quiet {
        writeln!(
            out,
            "{}",
            quiet_line(&outcome.title, &outcome.summary, &dir)
        )
    } else if ui.fancy && opts.mode == ProgressMode::Host {
        write!(
            out,
            "\n{}",
            fancy_summary(ui, &outcome.summary, &outcome.title, &dir)
        )
    } else {
        write!(out, "\n{}\n", outcome.md.replace("\r\n", "\n"))
    };
}

/// One `lab_uat_run` call on its own MCP session: connect, call (bounded by
/// `timeout`), close. `call` is LX-15's `RunCall::for_lane(instance)`.
fn call_lab_uat_run(ep: &Endpoint, call: &RunCall, timeout: Duration) -> Result<Value, McpError> {
    let mut client = McpClient::connect(ep)?;
    let reply = client.call_tool("lab_uat_run", call.to_value(), timeout);
    client.close();
    reply
}

/// The `FinishFn` of a native batch: a run's own chaos header from its reply and,
/// for a passing run under `--diff-golden` (`diff` is `Some`), the golden value
/// of its folder. The run's verdict is never changed. `golden_call` is the
/// `lab_golden_diff` call (`golden::call_golden_diff` in `execute`, a stub in tests).
pub fn finish_with<'a>(
    diff: Option<&'a DiffArgs>,
    golden_call: &'a (dyn Fn(Value) -> Result<Value, McpError> + Sync),
) -> impl Fn(&mut RunRecord, Option<&Value>) + Sync + 'a {
    move |rec: &mut RunRecord, reply: Option<&Value>| {
        if let Some(header) = reply.and_then(run_chaos) {
            rec.chaos = Some(header);
        }
        let mut call = |a: Value| golden_call(a);
        if let Some(value) = golden::run_golden(rec, diff, &mut call) {
            rec.golden = Some(value);
        }
    }
}

/// Runs a real batch against the daemon at `ep` and returns the exit code: 0 all
/// passed, 1 a run failed, 4 a lane stopped early, 5 a golden divergence or a diff
/// that could not be made, 8 a heal proposal was written, 3 when the batch folder
/// cannot be written.
pub fn execute(args: &Args, ctx: &Ctx, ep: &Endpoint, input: BatchInput) -> u8 {
    let mode = progress_mode(args.json, args.quiet, args.progress_to_stderr);
    let call = |instance: &str, timeout: Duration| {
        call_lab_uat_run(ep, &input.call.for_lane(instance), timeout)
    };
    let wait = |instance: &str, max: Duration| {
        mcp::wait_lease_free_stoppable(ep, instance, max, &ctrlc::stop_requested)
    };
    let sleep = |total: Duration| sleep_unless_stopped(total, &ctrlc::STOP);
    // --diff-golden (or --chaos, which implies it): compare each passing run with the golden.
    let diff = input
        .diff_golden
        .then(|| DiffArgs::new(&input.section, input.specs_dir.as_deref().map(Path::new)));
    let golden_call = |a: Value| golden::call_golden_diff(ep, a);
    let finish = finish_with(diff.as_ref(), &golden_call);
    let hooks = Hooks {
        call: &call,
        wait_lease_free: &wait,
        sleep: &sleep,
        finish: &finish,
        stop: &ctrlc::STOP,
    };
    let opts = BatchOptions {
        runs_per_lease: args.runs_per_lease,
        stagger_seconds: args.stagger_seconds,
        run_timeout_minutes: args.run_timeout_minutes,
        max_consecutive_failures: args.max_consecutive_failures,
        mode,
        quiet: args.quiet,
        json: args.json,
    };
    ctrlc::install();
    let emit = stdio_emit(mode);
    match run_batch(&input, &opts, &ctx.ui, &paths::uat_root(), &hooks, &emit) {
        Ok(mut outcome) => {
            // -Heal runs after the summary is on disk, not before: a heal is up to 15
            // minutes of lab time per pose, so Ctrl+C during the batch skips it.
            if args.heal && !ctrlc::stop_requested() {
                heal::heal_batch(ctx, ep, &input, &opts, &mut outcome);
            }
            print_outcome(&ctx.ui, &opts, &outcome);
            outcome.exit
        }
        Err(message) => {
            // Exit 3, one JSON object under --json, else the message on stderr.
            let _ = stop(
                &ctx.ui,
                args.json,
                3,
                &message,
                &mut anstream::stdout(),
                &mut anstream::stderr(),
            );
            3
        }
    }
}
