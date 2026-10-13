//! `lab_login` and `lab_logout`.
//!
//! Login replaces the old Lua-action autologin: it types the account and
//! password into the real edit boxes, presses the real buttons, and ends
//! at character select with the character list.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::widgets::{
    self, CHAR_SELECT_WIN, EULA_ACCEPT, EULA_WIN, LOGIN_ACCOUNT_EDIT, LOGIN_BUTTON,
    LOGIN_PASSWORD_EDIT, LOGIN_WIN, PROMPT_TEXT, SERVER_SELECT_BUTTON, SERVER_SELECT_WIN,
};
use super::{settle, FlowError, FlowRun};
use crate::supervisor::session_file::LabAccount;
use crate::supervisor::ui::window_click::{click_point, locate_chunk, refuse, ClickTarget};
use crate::supervisor::{LoginState, Supervisor};

/// How long the intro movies may take before the login screen shows.
const REACH_LOGIN_TIMEOUT: Duration = Duration::from_secs(90);
/// How long a fresh client may take to show its window.
const WINDOW_TIMEOUT: Duration = Duration::from_secs(90);
/// Login and server-select round trips.
const SERVER_TIMEOUT: Duration = Duration::from_secs(60);
/// The character-select Back button. Named after its siblings
/// (`CharSelect_PlayButton`); the stock Lua is not in this repo, so the name
/// is not yet checked on the live client.
const CHAR_SELECT_BACK: &str = "CharSelect_BackButton";
const CREDENTIAL_TRIES: usize = 2;

/// `lab_login` arguments; each falls back to `lab-account.json`.
#[derive(Debug, Clone, Default)]
pub struct LoginRequest {
    pub account: Option<String>,
    pub password: Option<String>,
    pub server: Option<String>,
}

/// Credentials resolved from the request and `lab-account.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLogin {
    pub account: String,
    pub password: String,
    pub server: Option<String>,
    /// The server came from the request (a miss is an error) rather than
    /// the account file (a miss falls back to the preselected row).
    pub server_explicit: bool,
}

/// Merge the request over the account file.
pub fn resolve_login(
    req: &LoginRequest,
    file: Option<&LabAccount>,
) -> Result<ResolvedLogin, String> {
    let pick = |a: &Option<String>, f: Option<&String>| {
        a.clone()
            .filter(|s| !s.is_empty())
            .or_else(|| f.filter(|s| !s.is_empty()).cloned())
    };
    let account = pick(&req.account, file.map(|f| &f.username))
        .ok_or("no account: pass `account` or set username in lab-account.json")?;
    let password = pick(&req.password, file.map(|f| &f.password))
        .ok_or("no password: pass `password` or set it in lab-account.json")?;
    let server_explicit = req.server.as_ref().is_some_and(|s| !s.is_empty());
    let server = pick(&req.server, file.map(|f| &f.server));
    Ok(ResolvedLogin {
        account,
        password,
        server,
        server_explicit,
    })
}

/// Which startup screen is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartScreen {
    Eula,
    Login,
    ServerSelect,
    CharSelect,
    /// Movies, loading, or nothing recognisable yet.
    Other,
}

/// Lua returning the visibility of the four startup screens.
pub fn start_screen_chunk() -> String {
    let probe = |w: &str| {
        format!(
            "tostring(select(2, pcall(function() return {} end)) == true)",
            widgets::visible(w)
        )
    };
    format!(
        "return {}, {}, {}, {}",
        probe(EULA_WIN),
        probe(LOGIN_WIN),
        probe(SERVER_SELECT_WIN),
        probe(CHAR_SELECT_WIN)
    )
}

/// Classify [`start_screen_chunk`]'s results. The EULA is modal over the
/// login screen, and the server list opens over a hidden-but-live login
/// window, so later screens win.
pub fn classify_start(results: &[String]) -> StartScreen {
    let v = |i: usize| results.get(i).map(String::as_str) == Some("true");
    if v(0) {
        StartScreen::Eula
    } else if v(3) {
        StartScreen::CharSelect
    } else if v(2) {
        StartScreen::ServerSelect
    } else if v(1) {
        StartScreen::Login
    } else {
        StartScreen::Other
    }
}

/// Lua that selects the server row named `name` (rows are list items, not
/// named windows, so this is the one Lua-driven selection in the flows;
/// the Select button is still clicked natively). Returns `selected, name, only`
/// (`only` is `only` when the requested name was missing and the list has just
/// one row, which was selected) or `missing, <available names>`.
pub fn select_server_chunk(name: &str) -> String {
    format!(
        "local item = ServerSelect_List:findColumnItemWithText({n}, 0, nil) \
         local only = '' \
         if item == nil then local names = {{}} \
           for i = 1, getShardCount() do names[#names + 1] = tostring(getShardInfo(i).name) end \
           if #names == 1 then \
             item = ServerSelect_List:findColumnItemWithText(names[1], 0, nil) only = 'only' end \
           if item == nil then return 'missing', table.concat(names, ', ') end end \
         ServerSelect_List:clearAllSelections() \
         ServerSelect_List:setItemSelectState(item, true) \
         local s = ServerSelect_List:getFirstSelectedItem() \
         return 'selected', tostring(s and s:getText()), only",
        n = widgets::lua_quote(name)
    )
}

/// Lua returning the preselected server row's name.
pub const SELECTED_SERVER_CHUNK: &str = "local s = ServerSelect_List:getFirstSelectedItem() \
     return 'selected', tostring(s and s:getText())";

impl Supervisor {
    /// Put the desktop foreground on the client once, then confirm that the
    /// real CEGUI dispatch path reaches the requested login edit box.
    async fn prepare_login_field(
        &self,
        run: &mut FlowRun<'_>,
        step: &str,
        window: &str,
    ) -> Result<(), FlowError> {
        let hwnd = self
            .game_hwnd()
            .await
            .map_err(|e| run.fail(step, format!("client foreground: {e}")))?;
        if let Err(e) = focus_client_window_once(hwnd) {
            return Err(run.fail(step, e));
        }

        self.verify_login_field_hit_test(run, step, window).await
    }

    async fn verify_login_field_hit_test(
        &self,
        run: &mut FlowRun<'_>,
        step: &str,
        window: &str,
    ) -> Result<(), FlowError> {
        let target = ClickTarget::Named(window.to_string());
        let loc = self
            .lua_json(&locate_chunk(&target))
            .await
            .map_err(|e| run.fail(step, format!("locate {window}: {e}")))?;
        if let Some(why) = refuse(&target, &loc) {
            return Err(run.fail(step, why));
        }
        let (x, y) = click_point(&loc).expect("refuse checked the login edit rectangle");
        self.verify_click_target(window, x, y)
            .await
            .map(|_| ())
            .map_err(|e| run.fail(step, e))
    }

    /// `lab_login` — from the intro movies (or any startup screen) to
    /// character select.
    pub async fn login_flow(&self, req: LoginRequest) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_login");
        let file = self.lab_account();
        let creds = resolve_login(&req, file.as_ref()).map_err(|e| run.fail("credentials", e))?;
        self.set_login_state(LoginState::LoggingIn).await;
        let out = self.login_steps(&mut run, &creds).await;
        match out {
            Ok(v) => {
                self.set_login_state(LoginState::CharSelect).await;
                Ok(run.finish(v))
            }
            Err(e) => {
                self.set_login_state(LoginState::Failed).await;
                Err(e)
            }
        }
    }

    async fn login_steps(
        &self,
        run: &mut FlowRun<'_>,
        creds: &ResolvedLogin,
    ) -> Result<Value, FlowError> {
        // A fresh client has no window for about 30 s; virtual focus needs
        // one. Wait for it instead of failing at once (2026-10-10).
        let t0 = Instant::now();
        let waited = self
            .wait_main_window(WINDOW_TIMEOUT)
            .await
            .map_err(|e| run.fail("window", e))?;
        if waited > 0 {
            run.record("window", t0, json!({ "waited_ms": waited }));
        }
        self.input_focus(true)
            .await
            .map_err(|e| run.fail("focus", e))?;

        // Skip the intro movies the way a player does: Escape until a
        // startup screen shows.
        let t0 = Instant::now();
        let mut escapes = 0u32;
        let screen = loop {
            let r = run.lua("reach_login", &start_screen_chunk()).await?;
            match classify_start(&r) {
                StartScreen::Other => {}
                StartScreen::Eula => {
                    let en = run
                        .lua(
                            "eula",
                            &format!("return tostring({})", widgets::enabled(EULA_ACCEPT)),
                        )
                        .await?;
                    if en.first().map(String::as_str) != Some("true") {
                        return Err(run
                            .fail_with_state(
                                "eula",
                                "the EULA is showing and its Accept button stays disabled \
                                 until the text is scrolled to the end; accept it once by hand",
                            )
                            .await);
                    }
                    run.click("eula", EULA_ACCEPT).await?;
                    settle(1000).await;
                    continue;
                }
                other => break other,
            }
            if t0.elapsed() > REACH_LOGIN_TIMEOUT {
                return Err(run
                    .fail_with_state(
                        "reach_login",
                        format!("no login screen after {escapes} Escape presses"),
                    )
                    .await);
            }
            // Escape fails harmlessly while the window is still coming up.
            if self.input_key("Escape", "tap", None).await.is_ok() {
                escapes += 1;
            }
            settle(1000).await;
        };
        run.record(
            "reach_login",
            t0,
            json!({ "screen": format!("{screen:?}"), "escapes": escapes }),
        );

        if screen == StartScreen::Login {
            // On a freshly booted client the password box sometimes reads
            // empty after typing (twice on 2026-10-10, both after 16 Escape
            // presses to skip the intro movies; late Escapes are the likely
            // cause). Type the pair again, once.
            let mut attempt = 1;
            loop {
                self.prepare_login_field(run, "account_focus", LOGIN_ACCOUNT_EDIT)
                    .await?;
                self.prepare_login_field(run, "password_focus", LOGIN_PASSWORD_EDIT)
                    .await?;
                let typed = match run
                    .type_into("account", LOGIN_ACCOUNT_EDIT, &creds.account, true)
                    .await
                {
                    Ok(()) => {
                        run.type_into("password", LOGIN_PASSWORD_EDIT, &creds.password, true)
                            .await
                    }
                    Err(e) => Err(e),
                };
                match typed {
                    Ok(()) => break,
                    // Only a box that read back wrong is worth typing again;
                    // a dead client or a revoked lease would fail the same
                    // way twice. A focused test pins the readback-mismatch
                    // retry predicate; the fake bridge cannot type on Windows.
                    Err(e) if retryable_credential_error(attempt, &e) => {
                        tracing::warn!(
                            attempt,
                            step = %e.step,
                            error = %e.message,
                            "lab_login: credential typing missed; typing both again"
                        );
                        run.record(
                            "retry_credentials",
                            Instant::now(),
                            json!({ "attempt": attempt, "step": e.step, "error": e.message }),
                        );
                        attempt += 1;
                        settle(1000).await;
                    }
                    Err(e) => return Err(e),
                }
            }
            run.click("login", LOGIN_BUTTON).await?;
        }

        let mut server_name = Value::Null;
        if screen != StartScreen::CharSelect {
            let cond = format!(
                "{} and {}",
                widgets::visible(SERVER_SELECT_WIN),
                widgets::enabled(SERVER_SELECT_BUTTON)
            );
            run.wait("server_list", &cond, Some(PROMPT_TEXT), SERVER_TIMEOUT)
                .await?;
            server_name = json!(self.pick_server(run, creds).await?);
            run.click("select_server", SERVER_SELECT_BUTTON).await?;
            run.wait(
                "character_select",
                &widgets::visible(CHAR_SELECT_WIN),
                Some(PROMPT_TEXT),
                SERVER_TIMEOUT,
            )
            .await?;
            // The list fills in on the frame after the window shows.
            settle(1000).await;
        }

        let characters = self.read_characters(run).await?;
        Ok(json!({
            "at": "character_select",
            "account": creds.account,
            "server": server_name,
            "characters": characters,
        }))
    }

    /// Select the requested server row, or report the preselected one.
    async fn pick_server(
        &self,
        run: &mut FlowRun<'_>,
        creds: &ResolvedLogin,
    ) -> Result<String, FlowError> {
        let t0 = Instant::now();
        if let Some(name) = &creds.server {
            let r = run.lua("pick_server", &select_server_chunk(name)).await?;
            match r.first().map(String::as_str) {
                Some("selected") => {
                    let got = r.get(1).cloned().unwrap_or_default();
                    if r.get(2).map(String::as_str) == Some("only") {
                        tracing::info!(
                            requested = %name,
                            selected = %got,
                            "lab_login: requested server not in the list; using its only row"
                        );
                        run.record(
                            "pick_server",
                            t0,
                            json!({ "server": got, "requested": name, "only_row": true }),
                        );
                    } else {
                        run.record("pick_server", t0, json!({ "server": got }));
                    }
                    return Ok(got);
                }
                Some("missing") if creds.server_explicit => {
                    return Err(run.fail(
                        "pick_server",
                        format!(
                            "no server named {name:?}; the list has: {}",
                            r.get(1).cloned().unwrap_or_default()
                        ),
                    ));
                }
                Some("missing") => {
                    tracing::warn!(
                        server = %name,
                        available = ?r.get(1),
                        "lab-account.json server not in the list; using the preselected row"
                    );
                }
                _ => return Err(run.fail("pick_server", format!("unexpected results {r:?}"))),
            }
        }
        let r = run.lua("pick_server", SELECTED_SERVER_CHUNK).await?;
        let got = r.get(1).cloned().unwrap_or_default();
        run.record(
            "pick_server",
            t0,
            json!({ "server": got, "preselected": true }),
        );
        Ok(got)
    }

    /// `lab_logout` — `/logout` through chat, back to character select.
    pub async fn logout_flow(&self) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_logout");
        if run.visible("check", CHAR_SELECT_WIN).await? {
            return Ok(run.finish(json!({ "at": "character_select", "already": true })));
        }
        self.input_focus(true)
            .await
            .map_err(|e| run.fail("focus", e))?;
        let t0 = Instant::now();
        run.key("open_chat", "Enter").await?;
        settle(400).await;
        self.type_text("/logout")
            .await
            .map_err(|e| run.fail("type_logout", e))?;
        run.key("send", "Enter").await?;
        run.record("send_logout", t0, Value::Null);
        run.wait(
            "character_select",
            &widgets::visible(CHAR_SELECT_WIN),
            Some(PROMPT_TEXT),
            SERVER_TIMEOUT,
        )
        .await?;
        settle(1000).await;
        self.set_login_state(LoginState::CharSelect).await;
        let characters = self.read_characters(&mut run).await?;
        Ok(run.finish(json!({ "at": "character_select", "characters": characters })))
    }

    /// `lab_disconnect` — at character select, click Back. The client sends
    /// Account `logOff` (0xC2) and shows the login screen again. Already at
    /// the login screen is not an error.
    pub async fn disconnect_flow(&self) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_disconnect");
        if run.visible("check", LOGIN_WIN).await? {
            return Ok(run.finish(json!({ "at": "login", "already": true })));
        }
        if !run.visible("check", CHAR_SELECT_WIN).await? {
            return Err(run.fail(
                "check",
                "not at character select or the login screen; lab_disconnect clicks Back there",
            ));
        }
        run.click("click_back", CHAR_SELECT_BACK).await?;
        run.wait(
            "login_screen",
            &widgets::visible(LOGIN_WIN),
            Some(PROMPT_TEXT),
            SERVER_TIMEOUT,
        )
        .await?;
        settle(1000).await;
        Ok(run.finish(json!({ "at": "login" })))
    }
}

fn retryable_credential_error(attempt: usize, error: &FlowError) -> bool {
    attempt < CREDENTIAL_TRIES && error.message.contains("after typing")
}

fn focus_client_window_once(hwnd: isize) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, SetForegroundWindow,
        };

        let hwnd = hwnd as HWND;
        let mut is_foreground = || unsafe { GetForegroundWindow() == hwnd };
        let mut refocus = || unsafe { SetForegroundWindow(hwnd) != 0 };
        if ensure_foreground_once(&mut is_foreground, &mut refocus) {
            Ok(())
        } else {
            Err("client window is not foreground after one refocus attempt".into())
        }
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        Err("desktop foreground verification is only available on Windows".into())
    }
}

fn ensure_foreground_once(
    is_foreground: &mut impl FnMut() -> bool,
    refocus: &mut impl FnMut() -> bool,
) -> bool {
    is_foreground() || {
        let _ = refocus();
        is_foreground()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::supervisor::events::fake_bridge;

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
    async fn login_field_hit_test_names_covering_modal_before_input() {
        let input_calls = Arc::new(AtomicUsize::new(0));
        let observed_calls = input_calls.clone();
        let sup = fake_bridge::supervisor(Arc::new(move |method, params| {
            if method != "lua_eval" {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                return Err(format!("unexpected input call: {method}"));
            }
            let chunk = params["chunk"].as_str().unwrap_or_default();
            if chunk.contains("getTargetChildAtPosition") {
                let hit = json!({ "available": true, "target_name": LOGIN_ACCOUNT_EDIT,
                    "received_by": "LoginNoticeModal", "ancestors": ["LoginNoticeModal", "Root"] });
                return Ok(fake_bridge::lua_ok(&[hit.to_string()]));
            }
            let loc = json!({ "found": true, "name": LOGIN_ACCOUNT_EDIT,
                "visible": true, "enabled": true, "rect": [0, 0, 40, 20] });
            Ok(fake_bridge::lua_ok(&[loc.to_string()]))
        }))
        .await;
        let mut run = FlowRun::new(&sup, "lab_login");
        let error = sup
            .verify_login_field_hit_test(&mut run, "account_focus", LOGIN_ACCOUNT_EDIT)
            .await
            .unwrap_err();
        assert!(
            error.message.contains("covered by LoginNoticeModal"),
            "{error:?}"
        );
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn only_a_real_readback_mismatch_retries_and_only_once() {
        let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
        let run = FlowRun::new(&sup, "lab_login");
        let mismatch = run.fail(
            "account",
            "Login_AccountEdit holds 1 characters after typing",
        );
        let other = run.fail("account", "the bridge disconnected");
        assert!(retryable_credential_error(1, &mismatch));
        assert!(!retryable_credential_error(2, &mismatch));
        assert!(!retryable_credential_error(1, &other));
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
}
