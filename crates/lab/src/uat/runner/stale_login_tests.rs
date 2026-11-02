//! DR-07f: a stale `login_state` must not BLOCK every row. The supervisor can
//! report `character_select` while the client sits at the login screen (a
//! server kick, or a state left by an older disconnect); `ensure_state`
//! logs in once and retries the character-select call.

use serde_json::json;

use super::tests::{request, Fake};
use super::Runner;
use crate::uat::evidence::{RowResult, RunDir};

const ROW: &str = r#"
[[row]]
id = "M1-1"
title = "help"
expected = "Answers in chat."
step = [{ chat = ".help" }]
[[row.expect]]
id = "help-answers"
text = ".help lists commands"
source = "chat"
contains = "Available commands"
"#;

#[tokio::test]
async fn a_stale_character_select_state_logs_in_once_and_retries() {
    let fake = Fake::new(&[(".help", &["Available commands: .help"])])
        .with("lab_login")
        .with("lab_play_character")
        .answer(
            "lab_play_character",
            vec![
                json!({ "__error": "the client is not at character select; visible_screens [LoginWin]" }),
                json!({ "dialog_open": false }),
            ],
        );
    // The supervisor believes character select; the client is at LoginWin.
    *fake.login.lock().unwrap() = "character_select".into();
    let tmp = tempfile::tempdir().unwrap().keep();
    let out = Runner::new(&fake, None, request(&tmp, ROW))
        .unwrap()
        .run_all()
        .await
        .unwrap();
    let row = RunDir::open(std::path::Path::new(&out.run_dir))
        .unwrap()
        .rows()
        .unwrap()
        .remove(0);
    assert_eq!(row.result, RowResult::Pass, "{:?}", row.reasons);
    let calls = fake.names();
    let plays = calls.iter().filter(|c| *c == "lab_play_character").count();
    let logins = calls.iter().filter(|c| *c == "lab_login").count();
    assert_eq!((logins, plays), (1, 2), "{calls:?}");
}
