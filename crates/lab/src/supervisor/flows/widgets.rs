//! Widget names and Lua condition builders for the client flows.
//!
//! Every name here comes from the stock client UI Lua
//! (`SGWGame/Content/UI/Startup/*` and `Core/Dialog`, `Common/Prompt`) and
//! was proven on the live client by the prototype flow scripts. Keeping
//! them in one pure module means a renamed widget is a one-line fix and
//! the builders are unit-tested without a client.

/// The client's character-slot cap (`CharSelectMod.MAX_CHARS`).
pub const MAX_CHARACTERS: usize = 8;

/// Prompt layout instances the client creates (`Prompt1_PromptWin` ..
/// `Prompt5_PromptWin`, `Common/Prompt/Prompt.lua`).
pub const PROMPT_INSTANCES: u32 = 5;

pub const LOGIN_WIN: &str = "LoginWin";
pub const LOGIN_ACCOUNT_EDIT: &str = "Login_AccountEdit";
pub const LOGIN_PASSWORD_EDIT: &str = "Login_PasswordEdit";
pub const LOGIN_BUTTON: &str = "Login_LoginButton";
pub const EULA_WIN: &str = "EULAWin";
pub const EULA_ACCEPT: &str = "EULA_AcceptButton";
pub const SERVER_SELECT_WIN: &str = "ServerSelectWin";
pub const SERVER_SELECT_BUTTON: &str = "ServerSelect_SelectButton";
pub const CHAR_SELECT_WIN: &str = "CharSelectWin";
pub const CHAR_SELECT_PLAY: &str = "CharSelect_PlayButton";
pub const CHAR_SELECT_CREATE: &str = "CharSelect_CreateButton";
pub const CHAR_SELECT_DELETE: &str = "CharSelect_DeleteButton";
pub const CHAR_CREATE_WIN: &str = "CharCreateWin";
pub const CHAR_CREATE_FIRST: &str = "CharCreate_Name1Edit";
pub const CHAR_CREATE_LAST: &str = "CharCreate_Name2Edit";
pub const CHAR_CREATE_BUTTON: &str = "CharCreate_CreateButton";
pub const DIALOG_WIN: &str = "DialogWin";
pub const DIALOG_NEXT: &str = "Dialog_NextButton";
pub const DIALOG_DONE: &str = "Dialog_DoneButton";
pub const DIALOG_ACCEPT: &str = "Dialog_AcceptButton";
pub const MOVIE_WIN: &str = "MoviePlayerWin";

/// Every button a dialog can show (`Core/Dialog/Dialog.lua`), reported by
/// `client_ui_state` and by a stuck `lab_finish_dialog`.
pub const DIALOG_BUTTONS: [&str; 10] = [
    "Dialog_NextButton",
    "Dialog_DoneButton",
    "Dialog_AcceptButton",
    "Dialog_DeclineButton",
    "Dialog_PrevButton1",
    "Dialog_PrevButton2",
    "Dialog_GenericButton1",
    "Dialog_GenericButton2",
    "Dialog_GenericButton3",
    "DialogWin/DialogWin__auto_closebutton__",
];

/// Screens worth naming in an error when a wait times out.
pub const KNOWN_SCREENS: [&str; 8] = [
    EULA_WIN,
    LOGIN_WIN,
    SERVER_SELECT_WIN,
    CHAR_SELECT_WIN,
    CHAR_CREATE_WIN,
    DIALOG_WIN,
    MOVIE_WIN,
    "SelfStatusWin",
];

/// Quote a string as a Lua string literal.
pub fn lua_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\0' => out.push_str("\\0"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A window looked up as a Lua global by name (safe for any name).
pub fn global(window: &str) -> String {
    format!("_G[{}]", lua_quote(window))
}

/// Lua expression: the window exists and is visible. A nil guard matters:
/// `CharSelectWin` is nil once the client is in the world.
pub fn visible(window: &str) -> String {
    let g = global(window);
    format!("({g} ~= nil and {g}:isVisible())")
}

/// Lua expression: the window exists and is not disabled.
pub fn enabled(window: &str) -> String {
    let g = global(window);
    format!("({g} ~= nil and not {g}:isDisabled())")
}

/// Lua expression: the window's text (empty string when it is missing).
pub fn text_of(window: &str) -> String {
    let g = global(window);
    format!("(({g} ~= nil and {g}:getText()) or \"\")")
}

/// The player's own status frame: part of the world HUD.
pub const SELF_STATUS_WIN: &str = "SelfStatusWin";

/// Lua expression: the in-world HUD is up, meaning `SelfStatusWin` is
/// visible. Nothing else will do: on the first live run (colo,
/// 2026-10-04) a new character's arrival cutscene (a Bink movie, not
/// `MoviePlayerWin`) was still on screen while `MinimapWin` or the intro
/// `DialogWin` already read visible, and `SelfStatusWin` became visible
/// only after an Escape skipped the cutscene.
pub fn world_up() -> String {
    visible(SELF_STATUS_WIN)
}

/// Lua expression: the `title: message` of the first visible prompt, or nil.
/// Failed logins, failed character creation and server messages all
/// surface through a prompt.
pub const PROMPT_TEXT: &str = "(function() for i = 1, 5 do \
     local w = _G['Prompt' .. i .. '_PromptWin'] \
     if w ~= nil and w:isVisible() then \
       local m = _G['Prompt' .. i .. '_Message'] \
       return tostring(w:getText()) .. ': ' .. tostring(m and m:getText() or '') \
     end end return nil end)()";

/// `CharSelect_CharContainer_<i>`, 1-based as the client numbers them.
pub fn char_container(index: usize) -> Result<String, String> {
    if (1..=MAX_CHARACTERS).contains(&index) {
        Ok(format!("CharSelect_CharContainer_{index}"))
    } else {
        Err(format!(
            "character slot {index} is outside 1..={MAX_CHARACTERS}"
        ))
    }
}

/// `Prompt<N>_Button1` (the Yes/first button of a two-button prompt).
pub fn prompt_button1(n: u32) -> String {
    format!("Prompt{n}_Button1")
}

/// Character-creation alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Sgu,
    Praxis,
}

impl Alignment {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "sgu" => Ok(Self::Sgu),
            "praxis" => Ok(Self::Praxis),
            other => Err(format!("alignment must be sgu or praxis, not {other:?}")),
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Sgu => "SGU",
            Self::Praxis => "Praxis",
        }
    }

    /// `CharCreate_SGUButton` / `CharCreate_PraxisButton`.
    pub fn button(self) -> String {
        format!("CharCreate_{}Button", self.prefix())
    }

    /// The archetypes this alignment offers, as the button-name suffix
    /// (`CharacterCreate.lua` registers six per alignment).
    pub fn archetypes(self) -> &'static [&'static str] {
        match self {
            Self::Sgu => &[
                "Soldier",
                "Commando",
                "Scientist",
                "Archeologist",
                "Asgard",
                "Sholva",
            ],
            Self::Praxis => &[
                "Soldier",
                "Commando",
                "Scientist",
                "Archeologist",
                "Goauld",
                "Jaffa",
            ],
        }
    }

    /// `CharCreate_<Alignment><Archetype>Button`, refusing an archetype
    /// the alignment does not offer (an SGU Jaffa button does not exist,
    /// and clicking a missing window would silently do nothing).
    pub fn archetype_button(self, archetype: &str) -> Result<String, String> {
        let want = archetype.to_ascii_lowercase().replace(['\'', ' '], "");
        // Accept the common spelling "archaeologist" for the client's
        // "Archeologist".
        let want = if want == "archaeologist" {
            "archeologist".to_string()
        } else {
            want
        };
        self.archetypes()
            .iter()
            .find(|a| a.to_ascii_lowercase() == want)
            .map(|a| format!("CharCreate_{}{a}Button", self.prefix()))
            .ok_or_else(|| {
                format!(
                    "{archetype:?} is not a {} archetype; choose one of {:?}",
                    self.prefix(),
                    self.archetypes()
                )
            })
    }
}

/// `CharCreate_MaleButton` / `CharCreate_FemaleButton`.
pub fn gender_button(gender: &str) -> Result<&'static str, String> {
    match gender.to_ascii_lowercase().as_str() {
        "male" | "m" => Ok("CharCreate_MaleButton"),
        "female" | "f" => Ok("CharCreate_FemaleButton"),
        other => Err(format!("gender must be male or female, not {other:?}")),
    }
}

/// The Asgard have no gender choice (`genderViable` in CharacterCreate.lua
/// hides the buttons), so the flow skips the gender click for them.
pub fn archetype_has_gender(archetype: &str) -> bool {
    !archetype.eq_ignore_ascii_case("asgard")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetype_buttons_follow_the_alignment() {
        assert_eq!(
            Alignment::Praxis.archetype_button("soldier").unwrap(),
            "CharCreate_PraxisSoldierButton"
        );
        assert_eq!(
            Alignment::Sgu.archetype_button("Asgard").unwrap(),
            "CharCreate_SGUAsgardButton"
        );
        assert_eq!(
            Alignment::Praxis.archetype_button("Goa'uld").unwrap(),
            "CharCreate_PraxisGoauldButton"
        );
        assert_eq!(
            Alignment::Sgu.archetype_button("archaeologist").unwrap(),
            "CharCreate_SGUArcheologistButton"
        );
    }

    /// An archetype from the other faction has no button: refuse it up
    /// front instead of clicking a missing window.
    #[test]
    fn cross_faction_archetypes_are_refused() {
        assert!(Alignment::Sgu.archetype_button("Jaffa").is_err());
        assert!(Alignment::Praxis.archetype_button("Asgard").is_err());
        assert!(Alignment::Praxis.archetype_button("Wizard").is_err());
    }

    #[test]
    fn alignment_and_gender_parse() {
        assert_eq!(
            Alignment::parse("SGU").unwrap().button(),
            "CharCreate_SGUButton"
        );
        assert_eq!(
            Alignment::parse("praxis").unwrap().button(),
            "CharCreate_PraxisButton"
        );
        assert!(Alignment::parse("goauld").is_err());
        assert_eq!(gender_button("F").unwrap(), "CharCreate_FemaleButton");
        assert!(gender_button("x").is_err());
        assert!(!archetype_has_gender("Asgard"));
        assert!(archetype_has_gender("Soldier"));
    }

    #[test]
    fn char_container_is_one_based_and_capped() {
        assert_eq!(char_container(1).unwrap(), "CharSelect_CharContainer_1");
        assert_eq!(char_container(8).unwrap(), "CharSelect_CharContainer_8");
        assert!(char_container(0).is_err());
        assert!(char_container(9).is_err());
    }

    /// `CharSelectWin` is nil in the world: the visibility check must guard
    /// the nil or every in-world poll raises a Lua error.
    #[test]
    fn visible_guards_a_nil_global() {
        assert_eq!(
            visible("CharSelectWin"),
            "(_G[\"CharSelectWin\"] ~= nil and _G[\"CharSelectWin\"]:isVisible())"
        );
    }

    #[test]
    fn lua_quote_escapes_quotes_and_backslashes() {
        assert_eq!(lua_quote(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(lua_quote("x\ny"), "\"x\\ny\"");
    }
}
