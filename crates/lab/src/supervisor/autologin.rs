//! Lua-driven autologin state machine (issue #685 spike-confirmed
//! recipe).
//!
//! The confirmed approach is to drive the login/select screens' own
//! module handlers and native-bound globals via `client_lua_eval`,
//! **not** synthesized CEGUI clicks. The screens are tracked by polling
//! `:isVisible()`; each screen has one action; character select scans
//! `getCharacterInfo(i).name` (1-based) for the lab character.
//!
//! # The live gap (must read)
//!
//! Phase-1 `client_lua_eval` (issue #684) runs a chunk fire-and-forget
//! and returns **no** results or print output (its `results` /
//! `print_output` fields are empty — return-value capture is a Phase-3
//! RE TODO). This machine's *reads* — `is_visible`, `character_count`,
//! `character_info` — all need those return values. They are abstracted
//! behind [`ClientScreen`] so the state machine is complete and tested
//! today; the live bridge adapter ([`super::mod`]'s `BridgeScreen`)
//! returns those reads as errors until capture lands. The
//! fire-and-forget *actions* (`eval`) work now. So autologin is
//! unit-proven and wired, and its live path unblocks the moment
//! lua_eval return-value capture ships. This is the single biggest item
//! needing live validation.

/// The screen the client is currently showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Eula,
    Login,
    ServerSelect,
    CharSelect,
    /// None of the tracked windows are visible — loading, or in world.
    None,
}

/// CEGUI window names polled with `:isVisible()`, in classification
/// priority order (a modal EULA sits over everything, etc.).
pub const WINDOWS: [(&str, Screen); 4] = [
    ("EULAWin", Screen::Eula),
    ("LoginWin", Screen::Login),
    ("ServerSelectWin", Screen::ServerSelect),
    ("CharSelectWin", Screen::CharSelect),
];

/// Lab account credentials, read from `lab-account.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabCreds {
    pub server: String,
    pub username: String,
    pub password: String,
    pub character: String,
}

/// One character-select slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharInfo {
    pub name: String,
    /// `> 0` means the slot is playable (finished creation, not locked).
    pub playable: i64,
}

/// Outcome of an autologin run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginOutcome {
    /// Reached the world on the lab character.
    EnteredWorld,
    /// Ran out of polls before finishing.
    ExhaustedPolls,
}

/// The client-facing surface autologin drives. Split into fire-and-
/// forget *actions* (`eval`, live today) and *reads* (`is_visible`,
/// character queries — live-blocked on lua_eval return capture).
pub trait ClientScreen {
    /// Poll a CEGUI window's `:isVisible()`.
    fn is_visible(&mut self, window: &str) -> Result<bool, String>;
    /// Run a fire-and-forget Lua action chunk.
    fn eval(&mut self, chunk: &str) -> Result<(), String>;
    /// `getCharacterCount()`.
    fn character_count(&mut self) -> Result<u32, String>;
    /// `getCharacterInfo(index)` (1-based).
    fn character_info(&mut self, index: u32) -> Result<CharInfo, String>;
}

/// Escape a string for embedding in a double-quoted Lua literal.
fn lua_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `EULAMod.onAcceptClicked()` — bypasses the scroll-position gate on
/// the Accept button (spike Q6). EULA gates first launch only; a
/// crash-relaunch will not re-show it (persisted machine variable).
pub fn eula_chunk() -> &'static str {
    "EULAMod.onAcceptClicked()"
}

/// The three chunks that submit the login: set the edits, then call the
/// native `accountLogin` global (spike Q2/Q4).
pub fn login_chunks(creds: &LabCreds) -> Vec<String> {
    vec![
        format!("Login_AccountEdit:setText({})", lua_quote(&creds.username)),
        format!("Login_PasswordEdit:setText({})", lua_quote(&creds.password)),
        format!(
            "accountLogin({}, {}, {})",
            lua_quote(&creds.server),
            lua_quote(&creds.username),
            lua_quote(&creds.password)
        ),
    ]
}

/// `selectServer("<shard>")` (spike Q2).
pub fn server_select_chunk(server: &str) -> String {
    format!("selectServer({})", lua_quote(server))
}

/// `playCharacter(<index>)` — 1-based slot (spike Q2).
pub fn play_chunk(index: u32) -> String {
    format!("playCharacter({index})")
}

/// Find the 1-based slot index of the lab character among scanned
/// slots. Matches on name and requires `playable > 0` (the client's own
/// Play-button gate). Returns `None` if absent or not playable.
pub fn resolve_character_index(slots: &[(u32, CharInfo)], target: &str) -> Option<u32> {
    slots
        .iter()
        .find(|(_, info)| info.name == target && info.playable > 0)
        .map(|(idx, _)| *idx)
}

/// Classify the current screen by polling each tracked window in
/// priority order. First visible wins; none visible ⇒ [`Screen::None`].
fn classify<S: ClientScreen>(s: &mut S) -> Result<Screen, String> {
    for (window, screen) in WINDOWS {
        if s.is_visible(window)? {
            return Ok(screen);
        }
    }
    Ok(Screen::None)
}

/// Scan every character slot and resolve the lab character's index.
fn scan_for_character<S: ClientScreen>(s: &mut S, target: &str) -> Result<Option<u32>, String> {
    let count = s.character_count()?;
    let mut slots = Vec::with_capacity(count as usize);
    for i in 1..=count {
        slots.push((i, s.character_info(i)?));
    }
    Ok(resolve_character_index(&slots, target))
}

/// Drive autologin to completion (or `max_polls`).
///
/// Acts once per screen *transition*: it will not re-submit login while
/// the Login screen is still up during connect. On character select it
/// scans for the lab character and plays it. Once the character has been
/// played and no tracked window is visible, it reports
/// [`LoginOutcome::EnteredWorld`].
pub fn run<S: ClientScreen>(
    s: &mut S,
    creds: &LabCreds,
    max_polls: u32,
) -> Result<LoginOutcome, String> {
    let mut last_acted: Option<Screen> = None;
    let mut played = false;

    for _ in 0..max_polls {
        let screen = classify(s)?;

        // Only act when the screen changes — avoids spamming an action
        // while a screen lingers (e.g. Login during the connect wait).
        if Some(screen) == last_acted {
            continue;
        }

        match screen {
            Screen::Eula => {
                s.eval(eula_chunk())?;
                last_acted = Some(Screen::Eula);
            }
            Screen::Login => {
                for chunk in login_chunks(creds) {
                    s.eval(&chunk)?;
                }
                last_acted = Some(Screen::Login);
            }
            Screen::ServerSelect => {
                s.eval(&server_select_chunk(&creds.server))?;
                last_acted = Some(Screen::ServerSelect);
            }
            Screen::CharSelect => match scan_for_character(s, &creds.character)? {
                Some(idx) => {
                    s.eval(&play_chunk(idx))?;
                    played = true;
                    last_acted = Some(Screen::CharSelect);
                }
                None => {
                    return Err(format!(
                        "lab character {:?} not found or not playable in character select",
                        creds.character
                    ));
                }
            },
            Screen::None => {
                // No tracked window: either loading between screens, or
                // in the world after playing the character.
                if played {
                    return Ok(LoginOutcome::EnteredWorld);
                }
                // Loading — reset last_acted so the next real screen is
                // treated as a fresh transition.
                last_acted = Some(Screen::None);
            }
        }
    }

    Ok(LoginOutcome::ExhaustedPolls)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A scripted screen source: starts at EULA, and each expected
    /// action advances it to the next screen. Records every `eval`
    /// chunk so the test can assert the exact call sequence.
    struct MockScreen {
        visible: HashMap<&'static str, bool>,
        chars: Vec<CharInfo>,
        calls: Vec<String>,
    }

    impl MockScreen {
        fn new(chars: Vec<CharInfo>) -> Self {
            let mut visible = HashMap::new();
            visible.insert("EULAWin", true);
            visible.insert("LoginWin", false);
            visible.insert("ServerSelectWin", false);
            visible.insert("CharSelectWin", false);
            Self {
                visible,
                chars,
                calls: Vec::new(),
            }
        }
        fn show_only(&mut self, w: Option<&'static str>) {
            for v in self.visible.values_mut() {
                *v = false;
            }
            if let Some(w) = w {
                self.visible.insert(w, true);
            }
        }
    }

    impl ClientScreen for MockScreen {
        fn is_visible(&mut self, window: &str) -> Result<bool, String> {
            Ok(*self.visible.get(window).unwrap_or(&false))
        }
        fn eval(&mut self, chunk: &str) -> Result<(), String> {
            self.calls.push(chunk.to_string());
            // Advance the scripted screen on the triggering action.
            if chunk.contains("EULAMod.onAcceptClicked") {
                self.show_only(Some("LoginWin"));
            } else if chunk.contains("accountLogin") {
                self.show_only(Some("ServerSelectWin"));
            } else if chunk.contains("selectServer") {
                self.show_only(Some("CharSelectWin"));
            } else if chunk.contains("playCharacter") {
                self.show_only(None); // entered world
            }
            Ok(())
        }
        fn character_count(&mut self) -> Result<u32, String> {
            Ok(self.chars.len() as u32)
        }
        fn character_info(&mut self, index: u32) -> Result<CharInfo, String> {
            self.chars
                .get((index - 1) as usize)
                .cloned()
                .ok_or_else(|| format!("no character at slot {index}"))
        }
    }

    fn creds() -> LabCreds {
        LabCreds {
            server: "Cimmeria".into(),
            username: "lab".into(),
            password: "pw".into(),
            character: "LabRat".into(),
        }
    }

    /// The full happy path: EULA → login → server → char scan → play →
    /// world, with the exact call sequence and the name→index scan
    /// (LabRat is slot 2).
    #[test]
    fn full_autologin_call_sequence() {
        let chars = vec![
            CharInfo {
                name: "SomeoneElse".into(),
                playable: 1,
            },
            CharInfo {
                name: "LabRat".into(),
                playable: 1,
            },
        ];
        let mut m = MockScreen::new(chars);
        let out = run(&mut m, &creds(), 50).unwrap();
        assert_eq!(out, LoginOutcome::EnteredWorld);
        assert_eq!(
            m.calls,
            vec![
                "EULAMod.onAcceptClicked()".to_string(),
                "Login_AccountEdit:setText(\"lab\")".to_string(),
                "Login_PasswordEdit:setText(\"pw\")".to_string(),
                "accountLogin(\"Cimmeria\", \"lab\", \"pw\")".to_string(),
                "selectServer(\"Cimmeria\")".to_string(),
                "playCharacter(2)".to_string(),
            ]
        );
    }

    /// A crash-relaunch skips the EULA (persisted machine var): the
    /// mock starts already at the Login screen.
    #[test]
    fn relaunch_starts_at_login_no_eula() {
        let mut m = MockScreen::new(vec![CharInfo {
            name: "LabRat".into(),
            playable: 1,
        }]);
        m.show_only(Some("LoginWin"));
        let out = run(&mut m, &creds(), 50).unwrap();
        assert_eq!(out, LoginOutcome::EnteredWorld);
        assert!(!m.calls.iter().any(|c| c.contains("EULAMod")));
        assert_eq!(
            m.calls.first().unwrap(),
            "Login_AccountEdit:setText(\"lab\")"
        );
        assert_eq!(m.calls.last().unwrap(), "playCharacter(1)");
    }

    /// The name→index scan: 1-based, gated on `playable > 0`.
    #[test]
    fn character_scan_is_one_based_and_playable_gated() {
        let slots = vec![
            (
                1,
                CharInfo {
                    name: "Alpha".into(),
                    playable: 1,
                },
            ),
            (
                2,
                CharInfo {
                    name: "LabRat".into(),
                    playable: 0,
                },
            ), // locked
            (
                3,
                CharInfo {
                    name: "LabRat".into(),
                    playable: 1,
                },
            ), // the real one
        ];
        assert_eq!(resolve_character_index(&slots, "LabRat"), Some(3));
        assert_eq!(resolve_character_index(&slots, "Ghost"), None);
        // A name that exists only as a non-playable slot resolves to None.
        assert_eq!(
            resolve_character_index(
                &[(
                    1,
                    CharInfo {
                        name: "Locked".into(),
                        playable: 0
                    }
                )],
                "Locked"
            ),
            None
        );
    }

    /// A missing lab character is an error, not a silent success.
    #[test]
    fn missing_character_errors() {
        let mut m = MockScreen::new(vec![CharInfo {
            name: "NotTheLabChar".into(),
            playable: 1,
        }]);
        m.show_only(Some("CharSelectWin"));
        let err = run(&mut m, &creds(), 50).unwrap_err();
        assert!(err.contains("not found or not playable"));
    }

    /// Login is submitted exactly once even if the Login screen lingers
    /// across several polls during the connect wait.
    #[test]
    fn login_submitted_once_while_screen_lingers() {
        struct Lingering {
            polls: u32,
            calls: Vec<String>,
        }
        impl ClientScreen for Lingering {
            fn is_visible(&mut self, window: &str) -> Result<bool, String> {
                // Login stays visible for the whole run.
                Ok(window == "LoginWin")
            }
            fn eval(&mut self, chunk: &str) -> Result<(), String> {
                self.polls += 1;
                self.calls.push(chunk.to_string());
                Ok(())
            }
            fn character_count(&mut self) -> Result<u32, String> {
                Ok(0)
            }
            fn character_info(&mut self, _: u32) -> Result<CharInfo, String> {
                Err("n/a".into())
            }
        }
        let mut l = Lingering {
            polls: 0,
            calls: Vec::new(),
        };
        let out = run(&mut l, &creds(), 20).unwrap();
        assert_eq!(out, LoginOutcome::ExhaustedPolls);
        // Exactly the 3 login chunks, once — no resubmission.
        assert_eq!(l.calls.len(), 3);
        assert!(l.calls[2].contains("accountLogin"));
    }
}
