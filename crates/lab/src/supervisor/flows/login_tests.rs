use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::supervisor::events::fake_bridge;

struct FakeCredentialIo<'a> {
    run: FlowRun<'a>,
    events: Vec<String>,
    typed_fields: Vec<&'static str>,
    cover_password: bool,
    account_mismatches: usize,
    retries: usize,
}

impl LoginCredentialIo for FakeCredentialIo<'_> {
    async fn prepare_field(
        &mut self,
        step: &'static str,
        _window: &'static str,
    ) -> Result<(), FlowError> {
        self.events.push(format!("prepare:{step}"));
        if step == "password_focus" && self.cover_password {
            return Err(self.run.fail(
                step,
                "covered-target: Login_PasswordEdit is covered by LoginNoticeModal",
            ));
        }
        Ok(())
    }

    async fn type_field(
        &mut self,
        step: &'static str,
        _window: &'static str,
        _text: &str,
    ) -> Result<(), FlowError> {
        self.events.push(format!("type:{step}"));
        self.typed_fields.push(step);
        if step == "account" && self.account_mismatches > 0 {
            self.account_mismatches -= 1;
            return Err(self
                .run
                .fail(step, "Login_AccountEdit holds 1 characters after typing"));
        }
        Ok(())
    }

    fn record_retry(&mut self, _attempt: usize, _error: &FlowError) {
        self.retries += 1;
    }
}

fn credential_io(sup: &Supervisor) -> FakeCredentialIo<'_> {
    FakeCredentialIo {
        run: FlowRun::new(sup, "lab_login"),
        events: Vec::new(),
        typed_fields: Vec::new(),
        cover_password: false,
        account_mismatches: 0,
        retries: 0,
    }
}

fn file() -> LabAccount {
    LabAccount {
        server: "colo".into(),
        username: "lab".into(),
        password: "test".into(),
        character: "Labone".into(),
    }
}

#[test]
fn request_overrides_the_account_file() {
    let req = LoginRequest {
        account: Some("other".into()),
        password: None,
        server: Some("local".into()),
    };
    let r = resolve_login(&req, Some(&file())).unwrap();
    assert_eq!(r.account, "other");
    assert_eq!(r.password, "test");
    assert_eq!(r.server.as_deref(), Some("local"));
    assert!(r.server_explicit);
}

#[test]
fn account_file_supplies_the_defaults() {
    let r = resolve_login(&LoginRequest::default(), Some(&file())).unwrap();
    assert_eq!((r.account.as_str(), r.password.as_str()), ("lab", "test"));
    assert_eq!(r.server.as_deref(), Some("colo"));
    assert!(!r.server_explicit);
}

#[test]
fn missing_credentials_are_named() {
    let e = resolve_login(&LoginRequest::default(), None).unwrap_err();
    assert!(e.contains("no account"));
    let req = LoginRequest {
        account: Some("lab".into()),
        ..Default::default()
    };
    assert!(resolve_login(&req, None)
        .unwrap_err()
        .contains("no password"));
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn start_screens_classify_in_priority_order() {
    assert_eq!(
        classify_start(&s(&["true", "true", "false", "false"])),
        StartScreen::Eula
    );
    assert_eq!(
        classify_start(&s(&["false", "true", "true", "false"])),
        StartScreen::ServerSelect
    );
    assert_eq!(
        classify_start(&s(&["false", "false", "false", "true"])),
        StartScreen::CharSelect
    );
    assert_eq!(
        classify_start(&s(&["false", "true", "false", "false"])),
        StartScreen::Login
    );
    assert_eq!(classify_start(&s(&["false"; 4])), StartScreen::Other);
    assert_eq!(classify_start(&[]), StartScreen::Other);
}

#[test]
fn foreground_loss_gets_one_refocus_and_is_rechecked() {
    let checks = AtomicUsize::new(0);
    let refocuses = AtomicUsize::new(0);
    let mut foreground = || checks.fetch_add(1, Ordering::SeqCst) == 1;
    let mut refocus = || {
        refocuses.fetch_add(1, Ordering::SeqCst);
        true
    };
    assert!(ensure_foreground_once(&mut foreground, &mut refocus));
    assert_eq!(checks.load(Ordering::SeqCst), 2);
    assert_eq!(refocuses.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn password_modal_after_account_readback_stops_before_password_input() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let mut io = credential_io(&sup);
    io.cover_password = true;
    let error = type_login_credentials(&mut io, "account", "secret")
        .await
        .unwrap_err();
    assert!(
        error.message.contains("covered by LoginNoticeModal"),
        "{error:?}"
    );
    assert_eq!(
        io.events,
        [
            "prepare:account_focus",
            "type:account",
            "prepare:password_focus"
        ]
    );
    assert_eq!(io.typed_fields, ["account"]);
}

#[tokio::test]
async fn account_readback_mismatch_retries_pair_with_fresh_focus_checks() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let mut io = credential_io(&sup);
    io.account_mismatches = 1;
    type_login_credentials(&mut io, "account", "secret")
        .await
        .unwrap();
    assert_eq!(io.retries, 1);
    assert_eq!(io.typed_fields, ["account", "account", "password"]);
    assert_eq!(
        io.events,
        [
            "prepare:account_focus",
            "type:account",
            "prepare:account_focus",
            "type:account",
            "prepare:password_focus",
            "type:password"
        ]
    );
}

#[tokio::test]
async fn second_readback_mismatch_stops_after_one_retry() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let mut io = credential_io(&sup);
    io.account_mismatches = CREDENTIAL_TRIES;
    let error = type_login_credentials(&mut io, "account", "secret")
        .await
        .unwrap_err();
    assert!(error.message.contains("after typing"));
    assert_eq!(io.retries, 1);
    assert_eq!(io.typed_fields, ["account", "account"]);
}

/// Fake client: the login window is hidden and character select is up,
/// so the flow reaches the Back click. The Back widget has no rect, so
/// the click fails at its first step, and the error names that widget.
#[tokio::test]
async fn disconnect_flow_clicks_back() {
    let sup = fake_bridge::supervisor(Arc::new(|method: &str, p: &Value| {
        if method != "lua_eval" {
            return Err(format!("no native path in the fake: {method}"));
        }
        let chunk = p["chunk"].as_str().unwrap_or_default();
        if chunk.contains(CHAR_SELECT_BACK) {
            return Err("no window".into());
        }
        if chunk.contains(LOGIN_WIN) {
            return Ok(fake_bridge::lua_ok(&["false"]));
        }
        Ok(fake_bridge::lua_ok(&["true"]))
    }))
    .await;
    let e = sup.disconnect_flow().await.unwrap_err();
    assert_eq!(e.step, "click_back");
    assert!(e.message.contains(CHAR_SELECT_BACK), "{}", e.message);
}

#[test]
fn server_select_chunk_quotes_the_name_and_lists_on_a_miss() {
    let c = select_server_chunk("Cimmeria \"colo\"");
    assert!(c.contains(r#"findColumnItemWithText("Cimmeria \"colo\"", 0, nil)"#));
    assert!(c.contains("getShardInfo(i).name"));
}

fn creds(server: &str, explicit: bool) -> ResolvedLogin {
    ResolvedLogin {
        account: "a".into(),
        password: "p".into(),
        server: Some(server.into()),
        server_explicit: explicit,
    }
}

/// Live regression (2026-10-12): `server: "colo"` against a client whose
/// list has the one row "Test" failed `pick_server`. The chunk now falls
/// back to the only row and says so; the flow records it.
#[tokio::test]
async fn an_unknown_name_selects_the_only_row_and_says_so() {
    let sup = fake_bridge::supervisor(Arc::new(|method: &str, p: &Value| {
        assert_eq!(method, "lua_eval");
        let chunk = p["chunk"].as_str().unwrap_or_default();
        assert!(chunk.contains("findColumnItemWithText(\"colo\""), "{chunk}");
        Ok(fake_bridge::lua_ok(&["selected", "Test", "only"]))
    }))
    .await;
    let mut run = FlowRun::new(&sup, "lab_login");
    let got = sup
        .pick_server(&mut run, &creds("colo", true))
        .await
        .unwrap();
    assert_eq!(got, "Test");
    let step = run.steps.last().unwrap();
    assert_eq!(step["detail"]["only_row"], true, "{step}");
    assert_eq!(step["detail"]["requested"], "colo", "{step}");
}

/// With several rows and no match the explicit request still fails and
/// lists the rows (the guard against picking the wrong shard).
#[tokio::test]
async fn an_unknown_name_with_several_rows_is_still_an_error() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| {
        Ok(fake_bridge::lua_ok(&["missing", "Test, Dev"]))
    }))
    .await;
    let mut run = FlowRun::new(&sup, "lab_login");
    let e = sup
        .pick_server(&mut run, &creds("colo", true))
        .await
        .unwrap_err();
    assert!(
        e.message.contains("no server named") && e.message.contains("Test, Dev"),
        "{}",
        e.message
    );
}

#[test]
fn the_chunk_falls_back_to_a_single_row() {
    let c = select_server_chunk("colo");
    assert!(c.contains("if #names == 1 then"), "{c}");
    assert!(c.contains("only = 'only'"), "{c}");
    assert!(c.trim_end().ends_with("only"), "{c}");
}
