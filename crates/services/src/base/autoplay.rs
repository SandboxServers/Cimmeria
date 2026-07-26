//! Server-side autoplay: auto-select a character at world entry instead of
//! waiting for the client's `playCharacter` (`0xC4`).
//!
//! # What this is for
//!
//! Verifying a server change normally requires a human launching the game
//! client and clicking through login and character select. Autoplay removes
//! the human from the *server-controlled* half of that: once a session has
//! authenticated and been handed its character list, the server synthesises
//! the character-select input itself and drives the ordinary world-entry
//! path.
//!
//! # What this is NOT
//!
//! Autoplay skips **client input only**. It is not an authentication bypass
//! and not a parallel world-entry flow:
//!
//! - It runs off the same [`ConnectedClientState`] the normal login flow
//!   creates, so the session has already passed credential validation and
//!   Mercury session establishment. Autoplay cannot manufacture a session.
//! - It calls the same `handle_play_character` a real `0xC4` would, with the
//!   same arguments. Everything downstream (RESET_ENTITIES, ENABLE_ENTITIES,
//!   `onClientMapLoad`, `mapLoaded`) is byte-identical to a human-driven
//!   entry.
//! - The configured character id is validated against the *authenticated
//!   account's own* character list before use. A configured id belonging to
//!   another account is refused. This is the load-bearing authorization
//!   check: without it, a config value would select the entity a session
//!   enters the world as, which is a privilege-escalation shape.
//!
//! # Why two gates
//!
//! [`AutoplaySettings::is_armed`] requires **both** an explicit character id
//! and `developer_mode`. A single environment variable leaking into a
//! production process therefore cannot arm auto-world-entry.
//!
//! The decision logic is kept pure in [`decide`] so it can be unit-tested
//! without a socket, a database, or a live session; the process-global in
//! [`settings`] only supplies the configured values at the call site.

use std::sync::OnceLock;

use cimmeria_common::ServerConfig;

use crate::mercury::types::CharacterInfo;

/// Resolved autoplay configuration, snapshotted once at base-service start.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutoplaySettings {
    /// Character to auto-select. `None` disables autoplay.
    pub player_id: Option<i32>,

    /// Optional world guard — refuse to auto-enter unless the character's
    /// persisted `world_location` matches. `None` skips the check.
    pub expected_world: Option<String>,

    /// Mirror of `ServerConfig::developer_mode`; autoplay's second gate.
    pub developer_mode: bool,
}

impl AutoplaySettings {
    /// Snapshot the autoplay-relevant fields out of the server config.
    pub fn from_config(config: &ServerConfig) -> Self {
        Self {
            player_id: config.autoplay_player_id,
            expected_world: config.autoplay_expected_world.clone(),
            developer_mode: config.developer_mode,
        }
    }

    /// True only when both opt-in gates are satisfied. "Armed" means autoplay
    /// will *evaluate* a session; it does not mean it will engage — the
    /// per-session ownership and world checks in [`decide`] still apply.
    pub fn is_armed(&self) -> bool {
        self.player_id.is_some() && self.developer_mode
    }
}

/// Process-global autoplay settings.
///
/// A global rather than a threaded parameter because the call site sits five
/// frames below the config owner (recv loop → connect loop → encrypted
/// dispatch → account arms → enable_entities), and several of those
/// signatures are already at the project's `too_many_arguments` threshold.
/// This mirrors the established `cimmeria_discord` singleton pattern, which
/// exists for the same reason. The decision logic itself stays pure and takes
/// settings by reference, so tests never touch this.
static SETTINGS: OnceLock<AutoplaySettings> = OnceLock::new();

/// Install the process-wide autoplay settings. First call wins; later calls
/// are ignored (multiple `BaseService::new` calls in a test process must not
/// fight over the value).
///
/// Emits a loud WARN when autoplay is armed. Auto-world-entry changes what a
/// login does, so an operator who armed it by accident should see it in the
/// first screen of startup logs rather than discovering it from behaviour.
pub fn init(settings: AutoplaySettings) {
    let armed = settings.is_armed();
    let player_id = settings.player_id;
    let expected_world = settings.expected_world.clone();

    if SETTINGS.set(settings).is_err() {
        return;
    }

    if armed {
        tracing::warn!(
            autoplay_player_id = ?player_id,
            autoplay_expected_world = ?expected_world,
            "AUTOPLAY ARMED: sessions will auto-enter the world without client \
             character-select input. This is a developer/CI bootstrap aid and \
             must not be enabled in production."
        );
    }
}

/// Current autoplay settings, or the disabled default when [`init`] was never
/// called (the case for every unit test and every library consumer that isn't
/// the server binary).
pub(crate) fn settings() -> &'static AutoplaySettings {
    static DISABLED: OnceLock<AutoplaySettings> = OnceLock::new();
    SETTINGS
        .get()
        .unwrap_or_else(|| DISABLED.get_or_init(AutoplaySettings::default))
}

/// Outcome of evaluating autoplay for one authenticated session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AutoplayDecision {
    /// Autoplay is not armed — the normal client-driven flow applies. This is
    /// the default and the only variant reachable from a shipped config.
    Disabled,

    /// Autoplay is armed but must not engage for this session. `reason` is a
    /// stable, queryable string for the refusal log.
    Refused { reason: &'static str },

    /// Auto-select `player_id` and drive the normal world-entry path.
    Engage { player_id: i32 },
}

/// Decide whether to auto-select a character for a session that has just been
/// sent its character list.
///
/// `characters` MUST be the list queried for the *authenticated* account —
/// that is what makes the ownership check meaningful. Passing another
/// account's list would defeat the authorization gate.
pub(crate) fn decide(
    settings: &AutoplaySettings,
    characters: &[CharacterInfo],
) -> AutoplayDecision {
    // Gate 1: an explicit character id. Checked first so a server with
    // autoplay unconfigured reports `Disabled` rather than a refusal reason,
    // keeping the refusal logs meaningful.
    let want = match settings.player_id {
        Some(id) => id,
        None => return AutoplayDecision::Disabled,
    };

    // Gate 2: developer mode. Deliberately a refusal rather than `Disabled`:
    // a configured id with developer_mode off is a misconfiguration the
    // operator should see, not silence.
    if !settings.developer_mode {
        return AutoplayDecision::Refused {
            reason: "developer_mode_required",
        };
    }

    // Authorization: the configured character must belong to THIS
    // authenticated account. `characters` came from
    // `query_character_list(db_pool, account_id)`, so membership in it is
    // proof of ownership. Without this check a config value would decide
    // which entity a session enters the world as.
    let character = match characters.iter().find(|c| c.player_id == want) {
        Some(c) => c,
        None => {
            return AutoplayDecision::Refused {
                reason: "character_not_owned_by_account",
            }
        }
    };

    // World guard. Compared against the character's persisted location —
    // autoplay refuses to enter the wrong map rather than relocating the
    // character into the expected one.
    if let Some(expected) = &settings.expected_world {
        if !character.world_location.eq_ignore_ascii_case(expected) {
            return AutoplayDecision::Refused {
                reason: "world_location_mismatch",
            };
        }
    }

    AutoplayDecision::Engage { player_id: want }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal character fixture. Only `player_id` and `world_location`
    /// participate in the autoplay decision; the rest is filler.
    fn character(player_id: i32, world_location: &str) -> CharacterInfo {
        CharacterInfo {
            player_id,
            name: format!("Char{player_id}"),
            extra_name: String::new(),
            alignment: 0,
            level: 1,
            gender: 0,
            world_location: world_location.to_string(),
            archetype: 0,
            title: 0,
            player_type: 0,
            playable: 1,
        }
    }

    fn armed(player_id: i32) -> AutoplaySettings {
        AutoplaySettings {
            player_id: Some(player_id),
            expected_world: None,
            developer_mode: true,
        }
    }

    // ── Inert-when-disabled guards ───────────────────────────────────────

    /// The headline safety property: a default (shipped) config never
    /// auto-enters the world, even when the account owns characters that
    /// would otherwise be selectable.
    #[test]
    fn decide_is_disabled_for_default_settings() {
        let characters = vec![character(1, "Castle_CellBlock"), character(2, "Agnos")];
        assert_eq!(
            decide(&AutoplaySettings::default(), &characters),
            AutoplayDecision::Disabled
        );
    }

    /// `settings()` must report disabled when `init` was never called. This
    /// is what makes autoplay inert for every consumer that isn't the server
    /// binary, including the entire unit-test suite.
    #[test]
    fn uninitialised_settings_are_disabled() {
        // NOTE: this asserts the *disabled default*, not the global's
        // post-init value — another test in this binary may legitimately have
        // called `init`. `is_armed()` on the default is the invariant that
        // matters, and it holds regardless of ordering.
        assert!(!AutoplaySettings::default().is_armed());
        assert_eq!(
            decide(&AutoplaySettings::default(), &[character(1, "Agnos")]),
            AutoplayDecision::Disabled
        );
    }

    /// A configured id alone must not arm autoplay — `developer_mode` is an
    /// independent gate. Reverting the two-gate rule to a single `player_id`
    /// check makes this fail.
    #[test]
    fn decide_refuses_configured_id_without_developer_mode() {
        let settings = AutoplaySettings {
            player_id: Some(7),
            expected_world: None,
            developer_mode: false,
        };
        assert_eq!(
            decide(&settings, &[character(7, "Castle_CellBlock")]),
            AutoplayDecision::Refused {
                reason: "developer_mode_required"
            }
        );
    }

    #[test]
    fn is_armed_requires_both_gates() {
        assert!(!AutoplaySettings::default().is_armed());
        assert!(!AutoplaySettings {
            player_id: Some(7),
            expected_world: None,
            developer_mode: false,
        }
        .is_armed());
        assert!(!AutoplaySettings {
            player_id: None,
            expected_world: None,
            developer_mode: true,
        }
        .is_armed());
        assert!(armed(7).is_armed());
    }

    // ── Authorization guards ─────────────────────────────────────────────

    /// The privilege-escalation shape: the configured id names a character
    /// this account does not own. Autoplay must refuse rather than enter the
    /// world as another account's character. Deleting the ownership check in
    /// `decide` makes this fail.
    #[test]
    fn decide_refuses_character_not_owned_by_the_authenticated_account() {
        // The account owns 1 and 2; autoplay is configured for 99.
        let characters = vec![character(1, "Castle_CellBlock"), character(2, "Agnos")];
        assert_eq!(
            decide(&armed(99), &characters),
            AutoplayDecision::Refused {
                reason: "character_not_owned_by_account"
            }
        );
    }

    /// An account with no characters at all must refuse, not panic or engage
    /// on a phantom id. This is the fresh-account / creation-screen shape.
    #[test]
    fn decide_refuses_when_account_has_no_characters() {
        assert_eq!(
            decide(&armed(1), &[]),
            AutoplayDecision::Refused {
                reason: "character_not_owned_by_account"
            }
        );
    }

    // ── World-guard behaviour ────────────────────────────────────────────

    #[test]
    fn decide_engages_when_world_guard_matches() {
        let settings = AutoplaySettings {
            player_id: Some(4),
            expected_world: Some("Castle_CellBlock".to_string()),
            developer_mode: true,
        };
        assert_eq!(
            decide(&settings, &[character(4, "Castle_CellBlock")]),
            AutoplayDecision::Engage { player_id: 4 }
        );
    }

    /// The world guard must refuse rather than relocate. Pins the documented
    /// "guard, not a teleport" semantic: a character saved in the wrong world
    /// yields a refusal, never an `Engage` that would silently enter the
    /// wrong map.
    #[test]
    fn decide_refuses_when_character_is_in_a_different_world() {
        let settings = AutoplaySettings {
            player_id: Some(4),
            expected_world: Some("Castle_CellBlock".to_string()),
            developer_mode: true,
        };
        assert_eq!(
            decide(&settings, &[character(4, "Agnos")]),
            AutoplayDecision::Refused {
                reason: "world_location_mismatch"
            }
        );
    }

    /// World names reach us from a DB column whose casing isn't guaranteed to
    /// match an operator-typed env var; the guard compares case-insensitively
    /// so a casing difference isn't mistaken for a wrong-world refusal.
    #[test]
    fn decide_world_guard_is_case_insensitive() {
        let settings = AutoplaySettings {
            player_id: Some(4),
            expected_world: Some("castle_cellblock".to_string()),
            developer_mode: true,
        };
        assert_eq!(
            decide(&settings, &[character(4, "Castle_CellBlock")]),
            AutoplayDecision::Engage { player_id: 4 }
        );
    }

    /// With no world guard configured, any owned character engages — the
    /// guard is opt-in on top of autoplay, not a second mandatory gate.
    #[test]
    fn decide_engages_without_world_guard_regardless_of_location() {
        assert_eq!(
            decide(&armed(2), &[character(1, "Agnos"), character(2, "Othala")]),
            AutoplayDecision::Engage { player_id: 2 }
        );
    }

    /// Selection must key on the configured id, not on list position — an
    /// off-by-one that grabbed `characters[0]` would enter the world as the
    /// wrong character on a multi-character account.
    #[test]
    fn decide_selects_by_id_not_by_list_position() {
        let characters = vec![
            character(10, "Agnos"),
            character(20, "Castle_CellBlock"),
            character(30, "Othala"),
        ];
        assert_eq!(
            decide(&armed(30), &characters),
            AutoplayDecision::Engage { player_id: 30 }
        );
    }
}
