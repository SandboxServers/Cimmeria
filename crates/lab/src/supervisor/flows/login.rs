//! `lab_login` and `lab_logout`.
//!
//! Login replaces the old Lua-action autologin: it types the account and
//! password into the real edit boxes, presses the real buttons, and ends
//! at character select with the character list.

use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tokio::sync::Mutex;

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
// All lab instances in the daemon share one Windows desktop foreground.
static CREDENTIAL_INPUT_LOCK: Mutex<()> = Mutex::const_new(());

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
    /// Try to put the desktop foreground on this client, then confirm that
    /// the real CEGUI dispatch path reaches the requested login edit box.
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
        let pid = self
            .client_pid()
            .await
            .ok_or_else(|| run.fail(step, "no client running"))?;
        match focus_client_window_once(hwnd, pid) {
            Ok(Some(diagnostic)) => tracing::warn!(
                step,
                diagnostic,
                "lab_login: physical activation denied; checking PID-scoped input path"
            ),
            Ok(None) => {}
            Err(e) => return Err(run.fail(step, e)),
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
            let mut io = SupervisorLoginCredentialIo { sup: self, run };
            type_login_credentials(&mut io, &creds.account, &creds.password).await?;
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

trait LoginCredentialIo {
    async fn prepare_field(
        &mut self,
        step: &'static str,
        window: &'static str,
    ) -> Result<(), FlowError>;

    async fn type_field(
        &mut self,
        step: &'static str,
        window: &'static str,
        text: &str,
    ) -> Result<(), FlowError>;

    fn record_retry(&mut self, attempt: usize, error: &FlowError);
}

struct SupervisorLoginCredentialIo<'a, 'r> {
    sup: &'a Supervisor,
    run: &'a mut FlowRun<'r>,
}

impl LoginCredentialIo for SupervisorLoginCredentialIo<'_, '_> {
    async fn prepare_field(
        &mut self,
        step: &'static str,
        window: &'static str,
    ) -> Result<(), FlowError> {
        self.sup.prepare_login_field(self.run, step, window).await
    }

    async fn type_field(
        &mut self,
        step: &'static str,
        window: &'static str,
        text: &str,
    ) -> Result<(), FlowError> {
        self.run.type_into(step, window, text, true).await
    }

    fn record_retry(&mut self, attempt: usize, error: &FlowError) {
        tracing::warn!(
            attempt,
            step = %error.step,
            error = %error.message,
            "lab_login: credential typing missed; typing both again"
        );
        self.run.record(
            "retry_credentials",
            Instant::now(),
            json!({ "attempt": attempt, "step": error.step, "error": error.message }),
        );
    }
}

async fn type_login_credentials(
    io: &mut impl LoginCredentialIo,
    account: &str,
    password: &str,
) -> Result<(), FlowError> {
    // Hold through password readback and retries: another instance refocusing
    // between the two edit boxes can send real input to the wrong client.
    let _credential_input = CREDENTIAL_INPUT_LOCK.lock().await;
    for attempt in 1..=CREDENTIAL_TRIES {
        let typed = async {
            io.prepare_field("account_focus", LOGIN_ACCOUNT_EDIT)
                .await?;
            io.type_field("account", LOGIN_ACCOUNT_EDIT, account)
                .await?;
            // Account entry can change window focus or expose a modal. Check
            // the password field only after account readback, before use.
            io.prepare_field("password_focus", LOGIN_PASSWORD_EDIT)
                .await?;
            io.type_field("password", LOGIN_PASSWORD_EDIT, password)
                .await
        }
        .await;
        match typed {
            Ok(()) => return Ok(()),
            Err(error) if retryable_credential_error(attempt, &error) => {
                io.record_retry(attempt, &error);
                settle(1000).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("credential retries always return or fail on the final attempt")
}

/// A failed physical activation is diagnostic: key and mouse messages go to
/// this HWND, and virtual focus makes the game process them. A wrong owner
/// PID is fatal because then that input could reach another process.
fn login_foreground_result(
    hwnd: isize,
    expected_pid: u32,
    owner_pid: u32,
    foreground: (isize, u32),
    activated: bool,
) -> Result<Option<String>, String> {
    if owner_pid == 0 || owner_pid != expected_pid {
        return Err(format!(
            "login target hwnd={hwnd:#x} belongs to pid={owner_pid}, expected pid={expected_pid}; input not sent"
        ));
    }
    if activated {
        Ok(None)
    } else {
        Ok(Some(format!(
            "target hwnd={hwnd:#x} pid={expected_pid}; foreground hwnd={:#x} pid={}; direct and attached activation denied",
            foreground.0, foreground.1
        )))
    }
}

fn focus_client_window_once(hwnd: isize, expected_pid: u32) -> Result<Option<String>, String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
        };

        let target = hwnd as HWND;
        let mut owner_pid = 0;
        unsafe { GetWindowThreadProcessId(target, &mut owner_pid) };
        if owner_pid == 0 || owner_pid != expected_pid {
            return login_foreground_result(hwnd, expected_pid, owner_pid, (0, 0), false);
        }
        let mut is_foreground = || unsafe { GetForegroundWindow() == target };
        let mut refocus = || unsafe { SetForegroundWindow(target) != 0 };
        let mut refocus_from_foreground_thread = || unsafe {
            let foreground = GetForegroundWindow();
            if foreground.is_null() {
                return false;
            }
            let foreground_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
            let caller_thread = GetCurrentThreadId();
            if foreground_thread == 0
                || caller_thread == 0
                || foreground_thread == caller_thread
                || AttachThreadInput(caller_thread, foreground_thread, 1) == 0
            {
                return false;
            }

            let activated = SetForegroundWindow(target) != 0;
            let detached = AttachThreadInput(caller_thread, foreground_thread, 0) != 0;
            activated && detached
        };
        let activated = ensure_foreground_once(
            &mut is_foreground,
            &mut refocus,
            &mut refocus_from_foreground_thread,
        );
        let foreground = unsafe { GetForegroundWindow() };
        let mut foreground_pid = 0;
        if !foreground.is_null() {
            unsafe { GetWindowThreadProcessId(foreground, &mut foreground_pid) };
        }
        login_foreground_result(
            hwnd,
            expected_pid,
            owner_pid,
            (foreground as isize, foreground_pid),
            activated,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (hwnd, expected_pid);
        Err("desktop foreground verification is only available on Windows".into())
    }
}

fn ensure_foreground_once(
    is_foreground: &mut impl FnMut() -> bool,
    refocus: &mut impl FnMut() -> bool,
    refocus_from_foreground_thread: &mut impl FnMut() -> bool,
) -> bool {
    if is_foreground() {
        return true;
    }
    let _ = refocus();
    if is_foreground() {
        return true;
    }
    let attached_refocus_succeeded = refocus_from_foreground_thread();
    attached_refocus_succeeded && is_foreground()
}

#[cfg(test)]
#[path = "login_tests.rs"]
mod tests;
