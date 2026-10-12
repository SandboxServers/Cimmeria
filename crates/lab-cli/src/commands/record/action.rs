//! The four `lab record` actions over an injected daemon probe and tool call,
//! so the tests run them with no daemon (`Invoke-RecordAction`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::daemon::mcp::{hide_lease_ids, McpError};
use crate::ui::{Mark, Tone, Ui};

use super::checks::{check_arguments, given, is_instance, MAX_MINUTES};
use super::json::{body_from_reply, call_error, compact_json, error_text, failure, Body};
use super::state::{read_state, save_state, state_path, RecordState};
use super::Args;

/// One tool call: name, arguments, timeout for that request.
pub type CallFn<'a> = dyn FnMut(&str, Value, Duration) -> Result<Value, McpError> + 'a;

/// What the actions need from the machine.
#[derive(Debug, Clone)]
pub struct Env {
    /// The lab home: the state files live here.
    pub lab_home: PathBuf,
    /// The UAT root: the raw log path is `<uat root>\records\<id>\record.jsonl`.
    pub uat_root: PathBuf,
    /// The checkout's spec folder, when there is one.
    pub specs_dir: Option<PathBuf>,
    /// The folder a relative `transform` folder is resolved against.
    pub cwd: PathBuf,
}

/// The result of one action.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The process exit code.
    pub exit: u8,
    /// What `--json` prints.
    pub body: Body,
    /// What a person gets (plain, or styled when the `Ui` is fancy); for a
    /// failure, the plain error text.
    pub text: String,
}

/// What to print, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// The text, without its final newline.
    pub text: String,
    /// True for stderr.
    pub to_stderr: bool,
}

struct Job<'a> {
    args: &'a Args,
    env: &'a Env,
    ui: &'a Ui,
    instance: &'a str,
    specs: Option<String>,
}

fn refuse(exit: u8, message: &str) -> Outcome {
    let body = failure(exit, message);
    let text = error_text(&body);
    Outcome { exit, body, text }
}

fn tool(
    call: &mut CallFn<'_>,
    name: &str,
    sent: Map<String, Value>,
    secs: u64,
) -> Result<Value, Outcome> {
    call(name, Value::Object(sent), Duration::from_secs(secs)).map_err(|err| {
        let (exit, body) = call_error(name, &err);
        let text = error_text(&body);
        Outcome { exit, body, text }
    })
}

/// A field as text, the way PowerShell interpolates it: empty for a missing or null one.
fn field_text(reply: &Value, key: &str) -> String {
    match reply.get(key) {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// PowerShell truthiness of a field: missing, null, false, 0 and "" are false.
fn truthy(reply: &Value, key: &str) -> bool {
    match reply.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(_) => true,
    }
}

/// `75` is `1m 15s`, `5` is `5s`, `3700` is `1h 1m`.
pub fn format_age(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, secs % 3600 / 60)
    }
}

fn join_path(base: &str, parts: &[&str]) -> String {
    let mut path = PathBuf::from(base);
    for part in parts {
        path.push(part);
    }
    path.display().to_string()
}

/// Runs one action. `probe` says whether the daemon answers; `call` makes one
/// tool call. Nothing here prints.
pub fn run_action<P, F>(args: &Args, env: &Env, ui: &Ui, probe: P, mut call: F) -> Outcome
where
    P: Fn() -> bool,
    F: FnMut(&str, Value, Duration) -> Result<Value, McpError>,
{
    if let Some(bad) = check_arguments(args) {
        return refuse(2, &bad);
    }
    let action = args.action.as_deref().unwrap_or("");
    let instance = given(&args.instance).unwrap_or("");
    let specs = given(&args.specs_dir)
        .map(str::to_owned)
        .or_else(|| env.specs_dir.as_ref().map(|p| p.display().to_string()));

    // stop's state file is read before the daemon is asked: without it there is nothing to stop.
    let mut state: Option<RecordState> = None;
    let mut state_file: Option<PathBuf> = None;
    if action == "stop" {
        let name = if instance.is_empty() {
            "default"
        } else {
            instance
        };
        let path = match state_path(&env.lab_home, name) {
            Ok(path) => path,
            Err(message) => return refuse(1, &format!("lab record: {message}")),
        };
        state = read_state(&path);
        if state.is_none() && given(&args.recording_id).is_none() {
            return refuse(
                2,
                &format!(
                    "no recording started here for instance {name} ({}); lab record status shows one, -RecordingId <id> stops it",
                    path.display()
                ),
            );
        }
        state_file = Some(path);
    }
    if !probe() {
        let message = if action == "transform" {
            "the transform runs in the lab daemon; start it with `lab start`"
        } else {
            "the lab daemon is not answering (lab status; lab start)"
        };
        return refuse(3, message);
    }

    let mut sent = Map::new();
    if !instance.is_empty() {
        sent.insert("instance".to_owned(), Value::from(instance));
    }
    let job = Job {
        args,
        env,
        ui,
        instance,
        specs,
    };
    let call: &mut CallFn<'_> = &mut call;
    match action {
        "start" => start(&job, sent, call),
        "status" => status(&job, sent, call),
        "stop" => stop(&job, sent, state.as_ref(), state_file.as_deref(), call),
        _ => transform(&job, sent, call),
    }
}

fn start(job: &Job<'_>, mut sent: Map<String, Value>, call: &mut CallFn<'_>) -> Outcome {
    let args = job.args;
    let ui = job.ui;
    let section = given(&args.target).unwrap_or("").to_owned();
    sent.insert("section".to_owned(), Value::from(section.clone()));
    if let Some(character) = given(&args.character) {
        sent.insert("character".to_owned(), Value::from(character));
    }
    if let Some(minutes) = given(&args.max_minutes) {
        // check_arguments proved it is 1..=30.
        let minutes = minutes.parse::<u32>().unwrap_or(MAX_MINUTES);
        sent.insert("max_minutes".to_owned(), Value::from(minutes));
    }
    if let Some(specs) = &job.specs {
        sent.insert("specs_dir".to_owned(), Value::from(specs.clone()));
    }
    let reply = match tool(call, "lab_record_start", sent, 120) {
        Ok(reply) => reply,
        Err(outcome) => return outcome,
    };
    let id = field_text(&reply, "recording_id");
    let mut on = field_text(&reply, "instance");
    if on.is_empty() {
        on = if job.instance.is_empty() {
            "default".to_owned()
        } else {
            job.instance.to_owned()
        };
    }
    if !is_instance(&on) || id.is_empty() {
        return refuse(
            1,
            &format!(
                "lab_record_start answered without a usable recording_id or instance; lab record status -Instance {on} shows it"
            ),
        );
    }
    let stop_hint = if on == "default" {
        "lab record stop".to_owned()
    } else {
        format!("lab record stop -Instance {on}")
    };
    // The recording runs on the daemon whatever happens here: a state file
    // that cannot be written must still name it.
    let path = match state_path(&job.env.lab_home, &on) {
        Ok(path) => path,
        Err(message) => return refuse(1, &message),
    };
    if let Err(err) = save_state(&path, &id, &on, job.specs.as_deref().unwrap_or("")) {
        return refuse(
            1,
            &format!(
                "recording {id} started on {on}, but its state file could not be written ({err}); stop it with: {stop_hint} -RecordingId {id}"
            ),
        );
    }
    let text = if ui.fancy {
        let minutes = given(&args.max_minutes)
            .and_then(|m| m.parse::<u32>().ok())
            .unwrap_or(MAX_MINUTES);
        format!(
            "{} recording {on} · section {section} · stops by itself in {minutes}m\n{}",
            ui.paint(Tone::Ok, ui.mark(Mark::Running)),
            ui.paint(Tone::Dim, &format!("  stop with: {stop_hint}"))
        )
    } else {
        format!("recording {id} on {on} (section {section}); play, then: {stop_hint}")
    };
    Outcome {
        exit: 0,
        body: body_from_reply(&reply),
        text,
    }
}

fn status(job: &Job<'_>, sent: Map<String, Value>, call: &mut CallFn<'_>) -> Outcome {
    let ui = job.ui;
    let reply = match tool(call, "lab_record_status", sent, 30) {
        Ok(reply) => reply,
        Err(outcome) => return outcome,
    };
    let on = field_text(&reply, "instance");
    let id = field_text(&reply, "recording_id");
    let section = field_text(&reply, "section");
    let (age, ticks, anchors) = (
        field_text(&reply, "age_s"),
        field_text(&reply, "ticks"),
        field_text(&reply, "anchors"),
    );
    let running = truthy(&reply, "recording");
    let text = if !truthy(&reply, "recording_id") {
        if ui.fancy {
            ui.paint(
                Tone::Dim,
                &format!("{} no recording on {on}", ui.mark(Mark::Stopped)),
            )
        } else {
            format!("no recording on {on}")
        }
    } else if ui.fancy {
        let head = if running {
            format!(
                "{} recording {id}",
                ui.paint(Tone::Ok, ui.mark(Mark::Running))
            )
        } else {
            format!(
                "{} stopped by itself, waiting for lab record stop: {id}",
                ui.paint(Tone::Warn, ui.mark(Mark::Warn))
            )
        };
        let age_text = reply
            .get("age_s")
            .and_then(Value::as_u64)
            .map(format_age)
            .unwrap_or_default();
        let kv = |key: &str, value: &str| {
            format!("  {}  {value}", ui.paint(Tone::Dim, &format!("{key:<9}")))
        };
        [
            head,
            kv("instance", &on),
            kv("section", &section),
            kv("age", &age_text),
            kv("ticks", &ticks),
            kv("anchors", &anchors),
        ]
        .join("\n")
    } else {
        let what = if running {
            "recording"
        } else {
            "stopped by itself, waiting for lab record stop:"
        };
        format!(
            "{what} {id} on {on} (section {section}): {age} s, {ticks} ticks, {anchors} anchors"
        )
    };
    Outcome {
        exit: 0,
        body: body_from_reply(&reply),
        text,
    }
}

fn stop(
    job: &Job<'_>,
    mut sent: Map<String, Value>,
    state: Option<&RecordState>,
    state_file: Option<&Path>,
    call: &mut CallFn<'_>,
) -> Outcome {
    let args = job.args;
    let id = given(&args.recording_id)
        .map(str::to_owned)
        .or_else(|| state.map(|s| s.recording_id.clone()))
        .unwrap_or_default();
    sent.insert("recording_id".to_owned(), Value::from(id));
    let saved_on = state
        .and_then(|s| s.instance.as_deref())
        .filter(|_| job.instance.is_empty());
    if let Some(saved_on) = saved_on {
        sent.insert("instance".to_owned(), Value::from(saved_on));
    }
    if let Some(expr) = given(&args.character_expr) {
        sent.insert("character_expr".to_owned(), Value::from(expr));
    }
    if let Some(profile) = given(&args.profile) {
        sent.insert("profile".to_owned(), Value::from(profile));
    }
    if args.no_draft {
        sent.insert("no_draft".to_owned(), Value::Bool(true));
    }
    let reply = match tool(call, "lab_record_stop", sent, 180) {
        Ok(reply) => reply,
        Err(mut outcome) => {
            // The recording stays on a failure (LR-07): the file stays, so a retry needs no id.
            if matches!(outcome.exit, 1 | 3) && state.is_some() {
                outcome
                    .text
                    .push_str("\nthe recording is kept; retry with lab record stop");
                if !job.instance.is_empty() {
                    outcome
                        .text
                        .push_str(&format!(" -Instance {}", job.instance));
                }
            }
            return outcome;
        }
    };
    let removed = state_file
        .filter(|path| path.exists())
        .map(std::fs::remove_file)
        .transpose();
    if let Err(err) = removed {
        return refuse(1, &format!("lab record: {err}"));
    }
    // The draft went to the folder fixed at start; only the state file knows it.
    let draft_specs = state.and_then(|s| s.specs_dir.as_deref()).unwrap_or("");
    Outcome {
        exit: 0,
        text: draft_text(job.ui, &reply, draft_specs, &job.env.uat_root),
        body: body_from_reply(&reply),
    }
}

fn transform(job: &Job<'_>, mut sent: Map<String, Value>, call: &mut CallFn<'_>) -> Outcome {
    let args = job.args;
    let target = given(&args.target).unwrap_or("");
    // A folder given relative to here is sent in full; a bare id as it is.
    let path = Path::new(target);
    let rooted = path.is_absolute() || path.has_root();
    let full = job.env.cwd.join(path);
    let dir = if !rooted && full.is_dir() {
        std::path::absolute(&full)
            .unwrap_or(full)
            .display()
            .to_string()
    } else {
        target.to_owned()
    };
    sent.insert("dir".to_owned(), Value::from(dir));
    if let Some(section) = given(&args.section) {
        sent.insert("section".to_owned(), Value::from(section));
    }
    if let Some(expr) = given(&args.character_expr) {
        sent.insert("character_expr".to_owned(), Value::from(expr));
    }
    if let Some(profile) = given(&args.profile) {
        sent.insert("profile".to_owned(), Value::from(profile));
    }
    if let Some(specs) = &job.specs {
        sent.insert("specs_dir".to_owned(), Value::from(specs.clone()));
    }
    let reply = match tool(call, "lab_record_transform", sent, 120) {
        Ok(reply) => reply,
        Err(outcome) => return outcome,
    };
    Outcome {
        exit: 0,
        text: draft_text(
            job.ui,
            &reply,
            job.specs.as_deref().unwrap_or(""),
            &job.env.uat_root,
        ),
        body: body_from_reply(&reply),
    }
}

struct DraftParts {
    draft: Option<String>,
    counts: String,
    stopped: String,
    raw: String,
    ended: String,
}

fn draft_parts(reply: &Value, specs_dir: &str, uat_root: &Path) -> DraftParts {
    let raw = field_text(reply, "raw");
    let uat = uat_root.display().to_string();
    let raw_path = if !uat.is_empty() && !raw.is_empty() {
        join_path(&uat, &["records", &raw, "record.jsonl"])
    } else {
        raw
    };
    let draft = field_text(reply, "draft");
    let draft_path = if draft.is_empty() {
        None
    } else if !specs_dir.is_empty() {
        Some(join_path(specs_dir, &["drafts", &draft]))
    } else {
        Some(format!("<specs>/drafts/{draft}"))
    };
    DraftParts {
        draft: draft_path,
        counts: format!(
            "{} row(s), {} step(s), {} TODO(s), {} dropped",
            field_text(reply, "rows"),
            field_text(reply, "steps"),
            field_text(reply, "todo"),
            field_text(reply, "dropped")
        ),
        stopped: format!("stopped; {} tick(s), no draft", field_text(reply, "ticks")),
        raw: raw_path,
        ended: field_text(reply, "ended"),
    }
}

/// A stop or transform answer for a person: the draft's and raw log's paths
/// (the tools answer with names only) and the counts. `specs_dir` is empty when
/// it is not known here, and the draft's path then starts `<specs>/drafts/`.
pub fn draft_text(ui: &Ui, reply: &Value, specs_dir: &str, uat_root: &Path) -> String {
    let parts = draft_parts(reply, specs_dir, uat_root);
    let mut lines: Vec<String> = Vec::new();
    if ui.fancy {
        let pass = ui.paint(Tone::Ok, ui.mark(Mark::Pass));
        match &parts.draft {
            Some(path) => {
                lines.push(format!(
                    "{pass} draft written: {}",
                    ui.paint(Tone::Cmd, path)
                ));
                lines.push(ui.paint(Tone::Dim, &format!("  {}", parts.counts)));
            }
            None => lines.push(format!("{pass} {}", parts.stopped)),
        }
        lines.push(ui.paint(Tone::Dim, &format!("  raw   {}", parts.raw)));
        if !parts.ended.is_empty() {
            lines.push(format!(
                "{} the recorder stopped early: {}",
                ui.paint(Tone::Warn, ui.mark(Mark::Warn)),
                parts.ended
            ));
        }
    } else {
        match &parts.draft {
            Some(path) => {
                lines.push(format!("draft {path}"));
                lines.push(format!("  {}", parts.counts));
            }
            None => lines.push(parts.stopped.clone()),
        }
        lines.push(format!("raw   {}", parts.raw));
        if !parts.ended.is_empty() {
            lines.push(format!("the recorder stopped early: {}", parts.ended));
        }
    }
    hide_lease_ids(&lines.join("\n"))
}

/// What the command prints for `outcome`: with `json`, the compact line on
/// stdout whatever the exit; otherwise the text on stdout for exit 0 and on
/// stderr (`error:` painted red when fancy) for any other exit.
pub fn render(ui: &Ui, json: bool, outcome: &Outcome) -> Rendered {
    if json {
        return Rendered {
            text: hide_lease_ids(&compact_json(&outcome.body)),
            to_stderr: false,
        };
    }
    let text = hide_lease_ids(&outcome.text);
    if outcome.exit == 0 {
        Rendered {
            text,
            to_stderr: false,
        }
    } else if ui.fancy {
        Rendered {
            text: format!("{} {text}", ui.paint(Tone::Fail, "error:")),
            to_stderr: true,
        }
    } else {
        Rendered {
            text,
            to_stderr: true,
        }
    }
}
