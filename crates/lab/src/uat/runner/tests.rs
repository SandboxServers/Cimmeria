//! Runner tests over a scripted fake client: chat typed through the
//! macro echoes replies, `client_ui_state` returns the chat tail, and the
//! flows report an in-world client. Each test pins one grading rule the
//! bundle must never get wrong.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde_json::{json, Map, Value};

use super::{LoadedSpec, RunRequest, Runner};
use crate::uat::attest::{attest, AttestRequest};
use crate::uat::evidence::{RowEvidence, RowResult, RunDir, Verdict};
use crate::uat::invoke::{ToolInvoker, ToolOutcome};
use crate::uat::spec;

/// A pretend client.
pub(super) struct Fake {
    tools: HashSet<String>,
    /// Typed chat line → lines the "server" answers with.
    replies: HashMap<String, Vec<String>>,
    chat: Mutex<Vec<String>>,
    typed: Mutex<String>,
    lua: Value,
    pub(super) calls: Mutex<Vec<String>>,
    /// Every call with its arguments (the two-player tests read these).
    pub(super) log: Mutex<Vec<(String, Value)>>,
    /// The flows' client state: running, and the login screen reached.
    running: Mutex<bool>,
    login: Mutex<String>,
    /// The character `client_player_state` reports.
    name: String,
    /// The lab event store (`client_wait_event`), and the events a tool
    /// call or typed line adds to it (see `ability_tests`).
    pub(super) events: Mutex<Vec<Value>>,
    pub(super) triggers: Mutex<Vec<(String, String, Value)>>,
    /// How long after the call the fake press "goes out" (`press_ms`).
    pub(super) press_delay_ms: Mutex<i64>,
}

const BASE_TOOLS: [&str; 9] = [
    "lab_client_status",
    "client_input_focus",
    "client_input_key",
    "client_type_text",
    "client_ui_state",
    "client_lua_eval",
    "client_wait_for",
    "lab_screenshot",
    "lab_logout",
];

impl Fake {
    pub(super) fn new(replies: &[(&str, &[&str])]) -> Self {
        Self {
            tools: BASE_TOOLS.iter().map(|s| s.to_string()).collect(),
            replies: replies
                .iter()
                .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                .collect(),
            chat: Mutex::new(vec!["Welcome to Stargate Worlds".into()]),
            typed: Mutex::new(String::new()),
            lua: json!({ "ok": true, "results": ["40"] }),
            calls: Mutex::new(vec![]),
            log: Mutex::new(vec![]),
            running: Mutex::new(true),
            login: Mutex::new("in_world".into()),
            name: "Labone".into(),
            events: Mutex::new(vec![]),
            triggers: Mutex::new(vec![]),
            press_delay_ms: Mutex::new(0),
        }
    }

    /// A client that is not running yet, playing `name` once in world.
    pub(super) fn stopped(mut self, name: &str) -> Self {
        *self.running.lock().unwrap() = false;
        *self.login.lock().unwrap() = "not_started".into();
        self.name = name.to_string();
        self
    }

    /// An in-world client whose player state reports `name`.
    pub(super) fn named(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub(super) fn names(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn without(mut self, tool: &str) -> Self {
        self.tools.remove(tool);
        self
    }

    pub(super) fn with(mut self, tool: &str) -> Self {
        self.tools.insert(tool.to_string());
        self
    }
}

impl ToolInvoker for Fake {
    fn has_tool(&self, name: &str) -> bool {
        self.tools.contains(name)
    }

    fn tool_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.tools.iter().cloned().collect();
        v.sort();
        v
    }

    async fn call(&self, name: &str, args: Value) -> ToolOutcome {
        self.calls.lock().unwrap().push(name.to_string());
        // A press fires its events itself, after noting the seq before.
        if name != "client_use_ability" {
            self.fire(name);
        }
        self.log
            .lock()
            .unwrap()
            .push((name.to_string(), args.clone()));
        let ok = |json: Value| ToolOutcome {
            ok: true,
            json,
            ..Default::default()
        };
        match name {
            "lab_client_status" => ok(json!({
                "running": *self.running.lock().unwrap(),
                "login_state": *self.login.lock().unwrap(),
            })),
            "lab_client_start" => {
                *self.running.lock().unwrap() = true;
                ok(json!({ "pid": 1 }))
            }
            "lab_login" | "lab_logout" => {
                *self.login.lock().unwrap() = "character_select".into();
                ok(json!({}))
            }
            "lab_play_character" => {
                *self.login.lock().unwrap() = "in_world".into();
                ok(json!({ "dialog_open": false }))
            }
            "client_player_state" => ok(json!({ "name": self.name, "level": 12 })),
            "client_type_text" => {
                *self.typed.lock().unwrap() = args["text"].as_str().unwrap_or_default().to_string();
                ok(json!({ "typed": true }))
            }
            "client_input_key" => {
                let mut typed = self.typed.lock().unwrap();
                if !typed.is_empty() {
                    let line = std::mem::take(&mut *typed);
                    self.fire(&format!("chat:{line}"));
                    let mut chat = self.chat.lock().unwrap();
                    if let Some(note) = line.strip_prefix(".bug ") {
                        chat.push(format!(
                            "Bookmark 1790650000000 recorded: 3 of 3 entities ({note})"
                        ));
                    }
                    for r in self.replies.get(&line).into_iter().flatten() {
                        chat.push(r.clone());
                    }
                }
                ok(json!({ "key": args["key"] }))
            }
            "client_ui_state" => ok(json!({ "chat_tail": *self.chat.lock().unwrap() })),
            "client_lua_eval" => ok(self.lua.clone()),
            "client_wait_for" => ok(json!({ "met": true, "elapsed_ms": 5 })),
            "client_wait_event" => ok(self.wait_event(&args)),
            "client_use_ability" => ok(self.use_ability(&args)),
            "lab_fail" => ToolOutcome::err("scripted failure"),
            // The click lands unless the spec allows the targetUnit fallback,
            // which this fake then reports taking.
            "client_target" if args["name"] == "Nobody" => {
                ToolOutcome::err("no entity named Nobody")
            }
            "client_target" if args["allow_fallback"] == true => {
                ok(json!({ "native_level": "ui_lua", "counts_as_native_pass": false }))
            }
            "client_target" => ok(json!({ "native_level": "real_input", "target": args["name"] })),
            "lab_screenshot" => ToolOutcome {
                ok: true,
                json: json!("client window 8x8"),
                images: vec![("image/png".into(), vec![0x89, b'P', b'N', b'G'])],
                ..Default::default()
            },
            _ => ok(json!({})),
        }
    }
}

pub(super) const HEAD: &str = r#"
schema = 1
[section]
id = "gm-parity"
system = "GM console command parity"
guide = "unified-uat.md#gm-console-command-parity"
ledger = "legacy-command-parity/README.md"
"#;

pub(super) fn request(root: &std::path::Path, rows: &str) -> RunRequest {
    let text = format!("{HEAD}{rows}");
    let spec = spec::parse(&text).unwrap();
    RunRequest {
        sections: vec![LoadedSpec {
            path: "test.toml".into(),
            sha256: "0".into(),
            spec,
        }],
        root: root.to_path_buf(),
        lab_character: Some("Labone".into()),
        operator: "test".into(),
        vars: Map::new(),
        no_settle: true,
        ..Default::default()
    }
}

async fn run_one(fake: &Fake, rows: &str) -> (RowEvidence, RunDir) {
    let tmp = tempfile::tempdir().unwrap().keep();
    let runner = Runner::new(fake, None, request(&tmp, rows)).unwrap();
    let out = runner.run_all().await.unwrap();
    let run = RunDir::open(std::path::Path::new(&out.run_dir)).unwrap();
    let row = run.rows().unwrap().remove(0);
    (row, run)
}

const HELP_ROW: &str = r#"
[[row]]
id = "M1-1"
title = "help"
expected = "Each answers in chat."
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help-answers"
text = ".help lists commands"
source = "chat"
contains = "Available commands"
"#;

#[tokio::test]
async fn a_native_chat_row_passes_with_anchor_bundle_and_ledger() {
    let fake = Fake::new(&[(".help", &["Available commands: .help .bug"])]);
    let (row, run) = run_one(&fake, HELP_ROW).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    let anchor = row.anchor.as_ref().unwrap();
    assert_eq!(anchor.note, "uat M1-1");
    assert_eq!(anchor.bookmark_id, Some(1_790_650_000_000));
    assert!(anchor.server_offset_ms.is_some());
    assert!(row
        .attachments
        .iter()
        .any(|a| a.name == "final" && a.path.ends_with("final.png")));
    let ledger = std::fs::read_to_string(run.ledger_md()).unwrap();
    assert!(ledger.contains("Step id:       M1-1"));
    assert!(ledger.contains("Result:        PASS"));
    assert!(ledger.contains("bookmark_id 1790650000000"));
}

/// Tool names are data: a tool nobody has built yet BLOCKs the row and
/// names it, and nothing is driven.
#[tokio::test]
async fn a_missing_tool_blocks_the_row_and_names_it() {
    let rows = r#"
[[row]]
id = "DP-U1"
title = "cast at a ground point"
expected = "An object appears."
step = [{ tool = "client_world_click", args = { point = [1, 2, 3] }, tier = "N1" }]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "x"
"#;
    let fake = Fake::new(&[]);
    let (row, _) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(
        row.reasons.iter().any(|r| r.contains("client_world_click")),
        "{:?}",
        row.reasons
    );
    assert!(
        fake.calls.lock().unwrap().is_empty(),
        "a blocked row must drive nothing"
    );
}

/// The fallback runs, the clause holds, and the row still is not a PASS:
/// the step ran at N3 and the row needs N1.
#[tokio::test]
async fn an_n3_fallback_is_a_native_shortfall_not_a_pass() {
    let rows = r#"
[[row]]
id = "CD6"
title = "open sciences"
expected = "The sciences show."
step = [{ tool = "client_open_window", args = { window = "Crafting" }, tier = "N1", fallback = [{ tool = "client_lua_eval", args = { chunk = "toggleCrafting()" } }] }]
[[row.expect]]
id = "count"
text = "sciences listed"
source = "lua"
chunk = "return #getAppliedSciences()"
op = "gte"
value = 4
"#;
    let fake = Fake::new(&[]);
    let (row, _) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::NativeShortfall, "{:?}", row.reasons);
    let step = row.actions.iter().find(|a| a.fallback_used).unwrap();
    assert_eq!(step.tool.as_deref(), Some("client_lua_eval"));
    assert_eq!(row.clauses[0].verdict, Verdict::Pass);
}

/// A SigNoz clause is PENDING after the run (so the row is UNVERIFIED),
/// and the attested rows are graded by the spec, not by the agent.
#[tokio::test]
async fn signoz_clauses_stay_pending_until_attested() {
    let rows = r#"
[[row]]
id = "CD3"
title = "no second resync"
expected = "SigNoz shows category 12 up_to_date."
step = [{ chat = ".help" }]
[[row.expect]]
id = "up-to-date"
text = "category 12 up_to_date"
source = "signoz"
filter = "event = 'cooked_data.version_reply' AND category_id = 12"
field = "outcome"
op = "eq"
value = "up_to_date"
"#;
    let fake = Fake::new(&[]);
    let (row, run) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::Unverified);
    let q = row.clauses[0].query.as_ref().unwrap();
    assert!(q["filter"]
        .as_str()
        .unwrap()
        .contains("cooked_data.version_reply"));
    assert!(q["from_ms"].as_i64().unwrap() < q["to_ms"].as_i64().unwrap());

    let mut req = AttestRequest {
        run_dir: run.root.clone(),
        row: Some("CD3".into()),
        clause: Some("up-to-date".into()),
        row_count: Some(1),
        rows: vec![json!({ "outcome": "full_resync" })],
        ..Default::default()
    };
    let out = attest(&req).unwrap();
    assert_eq!(out["result"], "FAIL", "a wrong outcome must fail: {out}");
    req.rows = vec![json!({ "outcome": "up_to_date" })];
    let out = attest(&req).unwrap();
    assert_eq!(out["result"], "PASS", "{out}");
    assert!(out["ledger_block"]
        .as_str()
        .unwrap()
        .contains("Result:        PASS"));
}

/// Colo rule 6: `.bm_seed` needs the owner's say-so in the run.
#[tokio::test]
async fn owner_only_commands_block_without_approval() {
    let rows = r#"
[[row]]
id = "U0"
title = "seed"
expected = "Listed 8"
step = [{ chat = ".bm_seed" }]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "Listed"
"#;
    let fake = Fake::new(&[(".bm_seed", &["Listed 8 Black Market auction(s)"])]);
    let (row, _) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::Blocked);
    assert!(row.reasons[0].contains("colo rule 6"), "{:?}", row.reasons);

    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, rows);
    req.owner_approvals = vec!["bm_seed".into()];
    let out = Runner::new(&fake, None, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "PASS", "{:?}", out.rows[0].reasons);
}

/// Chat 9a: a line echoed twice must fail a `count = 1` clause.
#[tokio::test]
async fn a_double_echo_fails_an_exactly_once_clause() {
    let rows = r#"
[[row]]
id = "9a"
title = "say once"
expected = "Each line shows exactly once."
step = [{ chat = "/say hi", tier = "N1" }]
[[row.expect]]
id = "say-once"
text = "say shows once"
source = "chat"
matches = "hi$"
count = 1
"#;
    let fake = Fake::new(&[("/say hi", &["Labone: hi", "Labone: hi"])]);
    let (row, _) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::Fail);
    assert_eq!(row.clauses[0].observed["match_count"], 2);
}

/// A human question holds the row at NEEDS_HUMAN until someone answers.
#[tokio::test]
async fn a_human_clause_needs_an_answer() {
    let rows = r#"
[[row]]
id = "U1"
title = "summon"
expected = "A Straegis Fighter appears."
step = [{ chat = ".pet summon 350" }]
[[row.expect]]
id = "reply"
text = "chat names the pet"
source = "chat"
contains = "summoned"
[[row.expect]]
id = "renders"
text = "the body renders"
source = "human"
question = "Does the Straegis Fighter render next to you (see final.png)?"
"#;
    let fake = Fake::new(&[(".pet summon 350", &["Pet 350 summoned"])]);
    let (row, run) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::NeedsHuman, "{:?}", row.reasons);
    let out = attest(&AttestRequest {
        run_dir: run.root.clone(),
        row: Some("U1".into()),
        clause: Some("renders".into()),
        verdict: Some(Verdict::Pass),
        answer: Some("yes, standing to my left".into()),
        by: Some("owner".into()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(out["result"], "PASS");
}

/// Captures feed later actions: a mail id read from chat lands in the
/// next command.
#[tokio::test]
async fn a_capture_feeds_the_next_action() {
    let rows = r##"
[[row]]
id = "6"
title = "expire"
expected = "Each reply names the outcome."
step = [
  { chat = ".mailbox" },
  { capture = "chat", regex = "#(\\d+) plain", var = "mail_id" },
  { chat = ".mail_expire ${mail_id}", label = "expire" },
]
[[row.expect]]
id = "deleted"
text = "the plain mail is deleted"
source = "chat"
since = "expire"
contains = "deleted"
"##;
    let fake = Fake::new(&[
        (".mailbox", &["#41 cod", "#42 plain"]),
        (".mail_expire 42", &["mail 42 deleted"]),
    ]);
    let (row, _) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    // A whole number is kept as one (it still types as "42").
    assert_eq!(row.vars["mail_id"], 42);
}

/// Plan-only drives nothing and reports every row SKIPPED or BLOCKED.
#[tokio::test]
async fn plan_only_drives_nothing() {
    let fake = Fake::new(&[]).without("client_type_text");
    let tmp = tempfile::tempdir().unwrap().keep();
    let mut req = request(&tmp, HELP_ROW);
    req.plan_only = true;
    let out = Runner::new(&fake, None, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "BLOCKED");
    assert!(out.rows[0]
        .reasons
        .iter()
        .any(|r| r.contains("client_type_text")));
    assert!(fake.calls.lock().unwrap().is_empty());

    let fake = Fake::new(&[]);
    let mut req = request(&tmp, HELP_ROW);
    req.plan_only = true;
    let out = Runner::new(&fake, None, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows[0].result, "SKIPPED");
}

/// The tools the router exposes on `main` today (#1080, #1090, #1099),
/// from a live `run.json` on 2026-09-29 plus the #1099 world tools.
const MAIN_TOOLS: &str =
    "client_call_native client_console client_cursor_move client_entity_table \
client_events_read client_hook_install client_hook_list client_hook_remove client_input_focus \
client_input_key client_input_mouse client_input_release client_input_status client_lua_eval \
client_mem_read client_mem_write client_module_info client_type_text client_ui_click \
client_ui_state client_wait_for lab_characters lab_client_restart lab_client_start \
lab_client_status lab_client_stop lab_crash_report lab_create_character lab_delete_character \
lab_ensure_character_slot lab_finish_dialog lab_login lab_logout lab_pixel_probe \
lab_play_character lab_screenshot lab_screenshot_region lab_timeline \
client_entity_find client_target client_world_click client_move_to client_camera \
client_hotbar client_use_ability client_combat_log client_die_and_respawn client_wait_event \
client_player_state";

/// Plan every committed spec against today's tools: the rows the lab can
/// drive now come back SKIPPED (ready), and the rows waiting on a planned
/// tool are BLOCKED naming it. Guards the spec set against a row that
/// silently needs a tool nobody listed.
#[tokio::test]
async fn committed_specs_plan_against_main_tools() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
    let sections = crate::uat::load_sections(&dir, None).unwrap();
    let abilities = sections
        .iter()
        .find(|s| s.spec.section.id == "ability-mechanics")
        .expect("abilities.toml is committed")
        .spec
        .clone();
    let mut fake = Fake::new(&[]);
    fake.tools = MAIN_TOOLS.split_whitespace().map(str::to_string).collect();
    let tmp = tempfile::tempdir().unwrap().keep();
    let req = RunRequest {
        sections,
        root: tmp,
        lab_character: Some("Labone".into()),
        plan_only: true,
        no_settle: true,
        ..Default::default()
    };
    let out = Runner::new(&fake, None, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let result = |s: &str, r: &str| {
        out.rows
            .iter()
            .find(|x| x.section == s && x.row == r)
            .unwrap_or_else(|| panic!("{s}/{r} missing"))
    };
    for (s, r) in [
        ("gm-parity", "M1-1"),
        ("gm-parity", "M4-1b"),
        ("chat", "9a"),
        ("cooked-data", "CD3"),
    ] {
        assert_eq!(
            result(s, r).result,
            "SKIPPED",
            "{s}/{r}: {:?}",
            result(s, r).reasons
        );
    }
    // Unblocked by #1099's client_world_click.
    assert_eq!(result("bank", "2").result, "SKIPPED");
    let i2 = result("consumables", "I2");
    assert_eq!(i2.result, "BLOCKED");
    assert!(i2.reasons.iter().any(|x| x.contains("client_item_action")));
    let cd1 = result("cooked-data", "CD1");
    assert!(cd1.reasons.iter().any(|x| x.contains("client_cache_files")));
    assert_eq!(
        result("black-market", "U0").result,
        "BLOCKED",
        "rule 6 without approval"
    );
    // Ability mechanics (AB-R0), every row: a one-player row with no
    // standing reason plans as ready against today's tools (so a row that
    // picks up an unrouted tool fails here); every other row is BLOCKED
    // with its own reason, or the second-player one.
    for row in &abilities.rows {
        let got = result("ability-mechanics", &row.id);
        let want = if row.blocked.is_some() || row.players > 1 {
            "BLOCKED"
        } else {
            "SKIPPED"
        };
        assert_eq!(got.result, want, "{}: {:?}", row.id, got.reasons);
        let why = row.blocked.as_deref().unwrap_or("second lab instance");
        if want == "BLOCKED" {
            assert!(
                got.reasons.iter().any(|x| x.contains(why)),
                "{}: {:?}",
                row.id,
                got.reasons
            );
        }
    }
    assert!(abilities.rows.len() >= 33, "the section lost rows");
    // Two players: BLOCKED until a second lab instance is configured.
    let m12 = result("gm-parity", "M1-2");
    assert_eq!(m12.result, "BLOCKED");
    assert!(
        m12.reasons[0].contains("second lab instance"),
        "{:?}",
        m12.reasons
    );
    assert!(fake.calls.lock().unwrap().is_empty());
    for r in &out.rows {
        println!(
            "{:<18} {:<8} {:<8} {}",
            r.section,
            r.row,
            r.result,
            r.reasons.join(" | ")
        );
    }
}

/// A world tool that says it fell back to a stock UI Lua call (#1099's
/// `native_level: ui_lua`) costs the row its PASS, even though the tool
/// itself is N1 in the capability table.
#[tokio::test]
async fn a_reported_ui_lua_fallback_is_a_native_shortfall() {
    let rows = r#"
[[row]]
id = "CP1"
title = "target a soldier"
expected = "targeted"
step = [{ tool = "@target", args = { name = "Op-CORE Soldier", allow_fallback = true } }]
[[row.expect]]
id = "c"
text = "t"
source = "wait"
lua_condition = "true"
"#;
    let mut fake = Fake::new(&[]);
    fake.tools.insert("client_target".into());
    let (row, _) = run_one(&fake, rows).await;
    assert_eq!(row.result, RowResult::NativeShortfall, "{:?}", row.reasons);
    let step = row
        .actions
        .iter()
        .find(|a| a.tool.as_deref() == Some("client_target"))
        .unwrap();
    assert_eq!(step.tier_source.as_deref(), Some("reported:ui_lua"));
}

/// AB-U20 and AB-U22 stage their NPC cast with the `.dummy caster` GM
/// command (#1188: a lab dummy that casts one ability at its owner through
/// the real launch). They are unblocked and plan as ready: nothing in the
/// rows is missing.
#[tokio::test]
async fn the_caster_dummy_rows_are_ready_once_unblocked() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/guides/uat-specs/abilities.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        !text.contains("blocked = \".dummy caster"),
        "AB-U20 and AB-U22 are unblocked"
    );
    let spec = crate::uat::spec::parse(&text).unwrap();
    let mut fake = Fake::new(&[]);
    fake.tools = MAIN_TOOLS.split_whitespace().map(str::to_string).collect();
    let req = RunRequest {
        sections: vec![LoadedSpec {
            path: "abilities.toml".into(),
            sha256: "0".into(),
            spec,
        }],
        rows: Some(vec!["AB-U20".into(), "AB-U22".into()]),
        root: tempfile::tempdir().unwrap().keep(),
        lab_character: Some("Labone".into()),
        plan_only: true,
        no_settle: true,
        ..Default::default()
    };
    let out = Runner::new(&fake, None, req)
        .unwrap()
        .run_all()
        .await
        .unwrap();
    assert_eq!(out.rows.len(), 2);
    for r in &out.rows {
        assert_eq!(r.result, "SKIPPED", "{}: {:?}", r.row, r.reasons);
    }
}
