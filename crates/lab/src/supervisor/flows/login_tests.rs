use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::supervisor::events::fake_bridge;

struct FakeCredentialIo<'a> {
    run: FlowRun<'a>,
    events: Vec<String>,
    typed_fields: Vec<&'static str>,
    cover_password: bool,
    activation_denied: bool,
    wrong_pid: bool,
    activation_diagnostics: Vec<String>,
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
        let owner_pid = if self.wrong_pid { 8 } else { 7 };
        let activation =
            login_foreground_result(0x1234, 7, owner_pid, (0x5678, 9), !self.activation_denied)
                .map_err(|e| self.run.fail(step, e))?;
        if let Some(diagnostic) = activation {
            self.activation_diagnostics.push(diagnostic);
        }
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
        activation_denied: false,
        wrong_pid: false,
        activation_diagnostics: Vec::new(),
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
    let attached_refocuses = AtomicUsize::new(0);
    let mut foreground = || checks.fetch_add(1, Ordering::SeqCst) == 1;
    let mut refocus = || {
        refocuses.fetch_add(1, Ordering::SeqCst);
        true
    };
    let mut attached_refocus = || {
        attached_refocuses.fetch_add(1, Ordering::SeqCst);
        false
    };
    assert!(ensure_foreground_once(
        &mut foreground,
        &mut refocus,
        &mut attached_refocus
    ));
    assert_eq!(checks.load(Ordering::SeqCst), 2);
    assert_eq!(refocuses.load(Ordering::SeqCst), 1);
    assert_eq!(attached_refocuses.load(Ordering::SeqCst), 0);
}

#[test]
fn failed_foreground_recovery_is_bounded_to_one_fallback() {
    let checks = AtomicUsize::new(0);
    let refocuses = AtomicUsize::new(0);
    let attached_refocuses = AtomicUsize::new(0);
    let mut foreground = || {
        checks.fetch_add(1, Ordering::SeqCst);
        false
    };
    let mut refocus = || {
        refocuses.fetch_add(1, Ordering::SeqCst);
        false
    };
    let mut attached_refocus = || {
        attached_refocuses.fetch_add(1, Ordering::SeqCst);
        false
    };
    assert!(!ensure_foreground_once(
        &mut foreground,
        &mut refocus,
        &mut attached_refocus
    ));
    assert_eq!(checks.load(Ordering::SeqCst), 2);
    assert_eq!(refocuses.load(Ordering::SeqCst), 1);
    assert_eq!(attached_refocuses.load(Ordering::SeqCst), 1);
}

#[test]
fn attached_foreground_thread_recovers_when_direct_activation_is_denied() {
    let active = AtomicBool::new(false);
    let direct_attempts = AtomicUsize::new(0);
    let attached_attempts = AtomicUsize::new(0);
    let mut foreground = || active.load(Ordering::SeqCst);
    let mut refocus = || {
        direct_attempts.fetch_add(1, Ordering::SeqCst);
        false
    };
    let mut attached_refocus = || {
        attached_attempts.fetch_add(1, Ordering::SeqCst);
        active.store(true, Ordering::SeqCst);
        true
    };
    assert!(ensure_foreground_once(
        &mut foreground,
        &mut refocus,
        &mut attached_refocus
    ));
    assert_eq!(direct_attempts.load(Ordering::SeqCst), 1);
    assert_eq!(attached_attempts.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_input_queue_detach_refuses_even_when_client_is_foreground() {
    let active = AtomicBool::new(false);
    let mut foreground = || active.load(Ordering::SeqCst);
    let mut refocus = || false;
    let mut attached_refocus = || {
        active.store(true, Ordering::SeqCst);
        false
    };
    assert!(!ensure_foreground_once(
        &mut foreground,
        &mut refocus,
        &mut attached_refocus
    ));
    assert!(active.load(Ordering::SeqCst));
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
async fn wrong_window_pid_stops_before_account_input() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let mut io = credential_io(&sup);
    io.wrong_pid = true;
    let error = type_login_credentials(&mut io, "account", "secret")
        .await
        .unwrap_err();
    assert!(error.message.contains("expected pid=7"), "{error:?}");
    assert_eq!(io.events, ["prepare:account_focus"]);
    assert!(io.typed_fields.is_empty());
}

#[tokio::test]
async fn activation_denial_uses_verified_pid_scoped_input_and_readback() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let mut io = credential_io(&sup);
    io.activation_denied = true;
    io.account_mismatches = 1;
    type_login_credentials(&mut io, "account", "secret")
        .await
        .unwrap();
    assert_eq!(io.typed_fields, ["account", "account", "password"]);
    assert_eq!(io.retries, 1);
    assert_eq!(io.activation_diagnostics.len(), 3);
    assert!(io.activation_diagnostics[0].contains("target hwnd=0x1234 pid=7"));
    assert!(io.activation_diagnostics[0].contains("foreground hwnd=0x5678 pid=9"));
}

#[tokio::test]
async fn activation_denial_does_not_bypass_covered_password_field() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let mut io = credential_io(&sup);
    io.activation_denied = true;
    io.cover_password = true;
    let error = type_login_credentials(&mut io, "account", "secret")
        .await
        .unwrap_err();
    assert!(error.message.contains("covered by LoginNoticeModal"));
    assert_eq!(io.typed_fields, ["account"]);
}

async fn prepare_with_denied_activation(covered: bool) -> Result<(), FlowError> {
    let probes = Arc::new(AtomicUsize::new(0));
    let seen = probes.clone();
    let sup = fake_bridge::supervisor(Arc::new(move |method: &str, p: &Value| {
        assert_eq!(method, "lua_eval");
        let chunk = p["chunk"].as_str().unwrap_or_default();
        seen.fetch_add(1, Ordering::SeqCst);
        let value = if chunk.contains("getTargetChildAtPosition") {
            let receiver = if covered {
                "LoginNoticeModal"
            } else {
                LOGIN_ACCOUNT_EDIT
            };
            json!({
                "available": true,
                "target_name": LOGIN_ACCOUNT_EDIT,
                "received_by": receiver,
                "ancestors": [receiver]
            })
        } else {
            assert!(chunk.contains("rect_centre"), "unexpected Lua: {chunk}");
            json!({
                "found": true,
                "name": LOGIN_ACCOUNT_EDIT,
                "visible": true,
                "enabled": true,
                "rect": [10, 20, 30, 40]
            })
        };
        Ok(fake_bridge::lua_ok(&[value.to_string()]))
    }))
    .await;
    let mut run = FlowRun::new(&sup, "lab_login");
    let result = sup
        .prepare_login_field_with_probe(
            &mut run,
            "account_focus",
            LOGIN_ACCOUNT_EDIT,
            0x1234,
            7,
            |hwnd, pid| login_foreground_result(hwnd, pid, 7, (0x5678, 9), false),
        )
        .await;
    assert_eq!(probes.load(Ordering::SeqCst), 2);
    result
}

#[tokio::test]
async fn supervisor_field_preparation_continues_after_denied_activation() {
    prepare_with_denied_activation(false).await.unwrap();
}

#[tokio::test]
async fn supervisor_field_preparation_refuses_a_cover_after_denial() {
    let error = prepare_with_denied_activation(true).await.unwrap_err();
    assert!(error.message.contains("covered by LoginNoticeModal"));
}

#[tokio::test]
async fn supervisor_field_preparation_rejects_wrong_pid_before_cegui_queries() {
    let queries = Arc::new(AtomicUsize::new(0));
    let seen = queries.clone();
    let sup = fake_bridge::supervisor(Arc::new(move |_, _| {
        seen.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Null)
    }))
    .await;
    let mut run = FlowRun::new(&sup, "lab_login");
    let error = sup
        .prepare_login_field_with_probe(
            &mut run,
            "account_focus",
            LOGIN_ACCOUNT_EDIT,
            0x1234,
            7,
            |hwnd, pid| login_foreground_result(hwnd, pid, 9, (0x5678, 9), false),
        )
        .await
        .unwrap_err();
    assert!(error.message.contains("belongs to pid=9"));
    assert_eq!(queries.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unsupported_secret_character_is_absent_from_error_and_steps() {
    let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let mut run = FlowRun::new(&sup, "lab_login");
    let error = run
        .type_into("password", LOGIN_PASSWORD_EDIT, "!", true)
        .await
        .unwrap_err();
    let output = format!("{} {}", error.summary(), json!(error.steps));
    assert!(error.message.contains("unsupported characters"));
    assert!(!output.contains('!'), "secret character leaked: {output}");
    assert!(error.steps.is_empty());
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

#[path = "login_credential_concurrency_tests.rs"]
mod concurrency;
