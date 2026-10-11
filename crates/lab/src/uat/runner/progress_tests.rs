//! The running row on `/status` follows the rows the runner drives, and a
//! plan never shows as running (LW-01, D-LW1).

use std::sync::Mutex;

use serde_json::Value;

use super::tests::{request, Fake};
use super::Runner;
use crate::uat::invoke::{ToolInvoker, ToolOutcome};
use crate::uat::progress::{ProgressCell, RunProgress};

const ROWS: &str = r#"
[[row]]
id = "R1"
title = "help"
expected = "Answers."
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help-answers"
text = ".help lists commands"
source = "chat"
contains = "Available commands"

[[row]]
id = "R2"
title = "after"
expected = "Answers."
step = [{ chat = ".who", label = "who" }]
[[row.expect]]
id = "who-answers"
text = ".who lists players"
source = "chat"
contains = "Players"
"#;

/// Delegates to `inner`, and before each call records the typed text (the
/// `text` argument, or "") with what the cell held at that moment.
struct Spy<'a> {
    inner: &'a Fake,
    cell: ProgressCell,
    seen: Mutex<Vec<(String, Option<RunProgress>)>>,
}

impl ToolInvoker for Spy<'_> {
    fn has_tool(&self, name: &str) -> bool {
        self.inner.has_tool(name)
    }

    fn tool_names(&self) -> Vec<String> {
        self.inner.tool_names()
    }

    async fn call(&self, name: &str, args: Value) -> ToolOutcome {
        let text = args["text"].as_str().unwrap_or_default().to_string();
        self.seen.lock().unwrap().push((text, self.cell.snapshot()));
        self.inner.call(name, args).await
    }
}

/// The first snapshot taken while `text` was typed.
fn seen_while_typing(seen: &Mutex<Vec<(String, Option<RunProgress>)>>, text: &str) -> RunProgress {
    seen.lock()
        .unwrap()
        .iter()
        .find(|(t, _)| t == text)
        .and_then(|(_, p)| p.clone())
        .unwrap_or_else(|| panic!("no progress while typing {text:?}"))
}

/// The row being driven shows as current, the rows before it as done, and
/// the cell is clear once the run returns. Fails if `run_all` drops either
/// hook or the guard.
#[tokio::test]
async fn progress_names_the_row_being_driven_and_clears_after_the_run() {
    let fake = Fake::new(&[
        (".help", &["Available commands: .help"]),
        (".who", &["Players online: 1"]),
    ]);
    let tmp = tempfile::tempdir().unwrap().keep();
    let cell = ProgressCell::default();
    let spy = Spy {
        inner: &fake,
        cell: cell.clone(),
        seen: Mutex::new(vec![]),
    };
    let runner = Runner::new(&spy, None, request(&tmp, ROWS))
        .unwrap()
        .with_progress(cell.clone());
    runner.run_all().await.unwrap();

    let help = seen_while_typing(&spy.seen, ".help");
    assert_eq!(help.current_row.as_deref(), Some("R1"));
    assert_eq!(help.row_index, Some(1));
    assert_eq!(help.rows_total, 2);

    let who = seen_while_typing(&spy.seen, ".who");
    assert_eq!(who.current_row.as_deref(), Some("R2"));
    assert_eq!(who.rows_done, 1);
    assert_eq!(who.last_result.as_deref(), Some("PASS"));

    assert_eq!(cell.snapshot(), None);
}

/// A plan drives nothing, so it never begins progress, even with a cell.
#[tokio::test]
async fn a_plan_only_run_never_begins_progress() {
    let fake = Fake::new(&[]);
    let tmp = tempfile::tempdir().unwrap().keep();
    let cell = ProgressCell::default();
    let mut req = request(&tmp, ROWS);
    req.plan_only = true;
    let runner = Runner::new(&fake, None, req)
        .unwrap()
        .with_progress(cell.clone());
    assert!(runner.begin_progress().is_none());
    runner.run_all().await.unwrap();
    assert_eq!(cell.snapshot(), None);
}
