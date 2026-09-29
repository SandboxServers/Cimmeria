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
use crate::supervisor::session_file::{self, LabAccount};
use crate::supervisor::{LoginState, Supervisor};

/// How long the intro movies may take before the login screen shows.
const REACH_LOGIN_TIMEOUT: Duration = Duration::from_secs(90);
/// Login and server-select round trips.
const SERVER_TIMEOUT: Duration = Duration::from_secs(60);

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
/// the Select button is still clicked natively). Returns `selected, name`
/// or `missing, <available names>`.
pub fn select_server_chunk(name: &str) -> String {
    format!(
        "local item = ServerSelect_List:findColumnItemWithText({n}, 0, nil) \
         if item == nil then local names = {{}} \
           for i = 1, getShardCount() do names[#names + 1] = tostring(getShardInfo(i).name) end \
           return 'missing', table.concat(names, ', ') end \
         ServerSelect_List:clearAllSelections() \
         ServerSelect_List:setItemSelectState(item, true) \
         local s = ServerSelect_List:getFirstSelectedItem() \
         return 'selected', tostring(s and s:getText())",
        n = widgets::lua_quote(name)
    )
}

/// Lua returning the preselected server row's name.
pub const SELECTED_SERVER_CHUNK: &str = "local s = ServerSelect_List:getFirstSelectedItem() \
     return 'selected', tostring(s and s:getText())";

impl Supervisor {
    /// `lab_login` — from the intro movies (or any startup screen) to
    /// character select.
    pub async fn login_flow(&self, req: LoginRequest) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_login");
        let file = self
            .config
            .install_dir
            .as_ref()
            .and_then(|d| session_file::read_lab_account(d).ok());
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
            run.type_into("account", LOGIN_ACCOUNT_EDIT, &creds.account, false)
                .await?;
            run.type_into("password", LOGIN_PASSWORD_EDIT, &creds.password, true)
                .await?;
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
                    run.record("pick_server", t0, json!({ "server": got }));
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn server_select_chunk_quotes_the_name_and_lists_on_a_miss() {
        let c = select_server_chunk("Cimmeria \"colo\"");
        assert!(c.contains(r#"findColumnItemWithText("Cimmeria \"colo\"", 0, nil)"#));
        assert!(c.contains("getShardInfo(i).name"));
    }
}
