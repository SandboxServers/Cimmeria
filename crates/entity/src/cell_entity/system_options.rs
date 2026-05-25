//! Per-session client option state — populated by the
//! `updateSystemOptions` cell method (player method index 93, wire signature
//! `ARRAY <of> NameValuePair`).
//!
//! Defaults mirror `SGWGame/Content/XML/SystemOptions.xml`:
//!
//! ```text
//! <option name='autoReload'         defaultBool='true'  serverSynch='true' />
//! <option name='reloadOnActivate'   defaultBool='false' serverSynch='true' />
//! ```
//!
//! Persistence is DB-backed via `sgw_player.auto_reload` and
//! `sgw_player.reload_on_activate`. The values are hydrated into the
//! cell entity at world-entry through `InitPlayerState`, and any live
//! toggle from `updateSystemOptions` (player method 93) fires a
//! `CellToBaseMsg::SystemOptionsUpdate` so the next login sees the
//! same values. The server-synched semantics from the original game
//! are preserved without involving the account layer — `sgw_player` is
//! per-character, which is the granularity the in-game checkbox panel
//! actually offers.

/// Single source of truth for the two reload-related client options.
///
/// Extending the struct to cover more of the ~100 options in
/// `SystemOptions.xml` is mechanical — add a field, default it from the
/// XML's `defaultBool`/`defaultInt`/`defaultFloat`/`defaultString`, and
/// add a match arm in [`SystemOptions::apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemOptions {
    /// `autoReload` — auto-reload weapon when the active clip hits zero.
    /// XML default: `true`.
    pub auto_reload: bool,

    /// `reloadOnActivate` — reload the active weapon on bandolier
    /// activate / weapon equip if the clip is below max.
    /// XML default: `false`.
    pub reload_on_activate: bool,
}

impl Default for SystemOptions {
    fn default() -> Self {
        Self {
            auto_reload: true,
            reload_on_activate: false,
        }
    }
}

impl SystemOptions {
    /// Apply a single `(name, value)` pair from the wire. Returns `true`
    /// if the option name was recognized (so the caller can log unknowns
    /// distinctly without us silently swallowing a typo).
    ///
    /// The value field is a `WSTRING` on the wire; the C++ client serialises
    /// every option type — bool, int, float, string — as its string
    /// representation, then SGW.exe parses per `optType`. We mirror that:
    /// booleans accept `"1"`, `"true"`, `"True"`, `"TRUE"` as truthy and
    /// anything else (including the empty string) as false.
    pub fn apply(&mut self, name: &str, value: &str) -> bool {
        match name {
            "autoReload" => {
                self.auto_reload = parse_bool(value);
                true
            }
            "reloadOnActivate" => {
                self.reload_on_activate = parse_bool(value);
                true
            }
            _ => false,
        }
    }
}

/// Parse a wire bool. Matches the original client's permissive parser —
/// `"1"` is the on-the-wire shape the client actually sends today, the
/// other tokens are belt-and-braces for any future SetSystemOption code
/// path that writes the literal `true`.
fn parse_bool(s: &str) -> bool {
    matches!(s, "1" | "true" | "True" | "TRUE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_xml() {
        let opts = SystemOptions::default();
        // SystemOptions.xml line 441: <option name='autoReload' defaultBool='true' />
        assert!(opts.auto_reload);
        // SystemOptions.xml line 451: <option name='reloadOnActivate' defaultBool='false' />
        assert!(!opts.reload_on_activate);
    }

    #[test]
    fn apply_recognises_auto_reload() {
        let mut opts = SystemOptions::default();
        assert!(opts.apply("autoReload", "0"));
        assert!(!opts.auto_reload);
        assert!(opts.apply("autoReload", "1"));
        assert!(opts.auto_reload);
    }

    #[test]
    fn apply_recognises_reload_on_activate() {
        let mut opts = SystemOptions::default();
        assert!(opts.apply("reloadOnActivate", "1"));
        assert!(opts.reload_on_activate);
        assert!(opts.apply("reloadOnActivate", "0"));
        assert!(!opts.reload_on_activate);
    }

    #[test]
    fn apply_returns_false_on_unknown_option() {
        let mut opts = SystemOptions::default();
        let before = opts.clone();
        assert!(!opts.apply("masterVolume", "0.5"));
        // Unknown name must not change anything.
        assert_eq!(opts, before);
    }

    #[test]
    fn apply_accepts_alternate_truthy_tokens() {
        // The wire form the 2009 client emits is "1"/"0", but the C++
        // option layer also accepts "true"/"True"/"TRUE" — see
        // SystemOptions.xml parser. Don't regress that.
        let mut opts = SystemOptions {
            auto_reload: false,
            ..SystemOptions::default()
        };
        for token in ["true", "True", "TRUE"] {
            opts.apply("autoReload", token);
            assert!(opts.auto_reload, "token {token:?} should be truthy");
            opts.auto_reload = false;
        }
    }

    #[test]
    fn apply_treats_empty_string_as_false() {
        let mut opts = SystemOptions {
            auto_reload: true,
            ..SystemOptions::default()
        };
        opts.apply("autoReload", "");
        assert!(!opts.auto_reload);
    }
}
