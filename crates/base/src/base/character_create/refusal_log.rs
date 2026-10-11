//! The `character_create_failed` row: exactly one line for every refused
//! `createCharacter`, carrying the stable `reason` of the call site (TG-DB-08).
//! The level follows the reason, through [`Reason::level`], so a reason can't
//! be misspelled into the wrong level: it is an enum, not a string.

use std::fmt::Display;
use std::net::SocketAddr;

use tracing::Level;

use super::fail_code::moniker;

/// Why a `createCharacter` was refused: the closed `reason` list of
/// `docs/architecture/negative-logging-convention.md` § TG-DB-08.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reason {
    NameParseFailed,
    NameRejected,
    ExtraNameRejected,
    MalformedArgs,
    InvalidSkinTint,
    UnknownCharDef,
    NoDbPool,
    QueryFailed,
    InvalidVisualGroup,
    ForcedGroupChoice,
    InvalidChoice,
    MissingOptionalChoice,
    /// The start profiles could not be read (`start_profile.rs`).
    StartProfilesLoadFailed,
    /// The CharDef has no start profile row.
    NoStartProfile,
    /// The start profile has a problem; carries `ProfileProblem::reason()`
    /// (`empty_world`, `origin_position`, ...), spelled by the resources crate.
    StartProfileProblem(&'static str),
    /// The profile's world is not in `resources.worlds`.
    StartWorldUnknown,
    /// The cell announced no space for the profile's world.
    StartWorldNotLoaded,
    /// The profile's `world_id` lookup failed.
    DbError,
    /// The starter kit could not be loaded, placed or recorded. The helper
    /// logged the precise ERROR (`cause`); this row is the client-refusal
    /// summary, so it is WARN and carries `kit_failure`.
    StarterKitFailed,
    NameTaken,
    DbWriteFailed,
}

impl Reason {
    /// The `reason` field value.
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::NameParseFailed => "name_parse_failed",
            Self::NameRejected => "name_rejected",
            Self::ExtraNameRejected => "extra_name_rejected",
            Self::MalformedArgs => "malformed_args",
            Self::InvalidSkinTint => "invalid_skin_tint",
            Self::UnknownCharDef => "unknown_char_def",
            Self::NoDbPool => "no_db_pool",
            Self::QueryFailed => "query_failed",
            Self::InvalidVisualGroup => "invalid_visual_group",
            Self::ForcedGroupChoice => "forced_group_choice",
            Self::InvalidChoice => "invalid_choice",
            Self::MissingOptionalChoice => "missing_optional_choice",
            Self::StartProfilesLoadFailed => "start_profiles_load_failed",
            Self::NoStartProfile => "no_start_profile",
            Self::StartProfileProblem(problem) => problem,
            Self::StartWorldUnknown => "start_world_unknown",
            Self::StartWorldNotLoaded => "start_world_not_loaded",
            Self::DbError => "db_error",
            Self::StarterKitFailed => "starter_kit_failed",
            Self::NameTaken => "name_taken",
            Self::DbWriteFailed => "db_write_failed",
        }
    }

    /// The level of the row: what each site logged at before TG-DB-08. A
    /// bad name or skin tint and a taken name are the player's doing (INFO);
    /// a database failure and every start-profile refusal (lock L3: a
    /// profile that can't be used is a content bug) are ERROR; the rest WARN.
    pub(super) fn level(self) -> Level {
        match self {
            Self::NameRejected
            | Self::ExtraNameRejected
            | Self::InvalidSkinTint
            | Self::NameTaken => Level::INFO,
            Self::QueryFailed
            | Self::DbWriteFailed
            | Self::StartProfilesLoadFailed
            | Self::NoStartProfile
            | Self::StartProfileProblem(_)
            | Self::StartWorldUnknown
            | Self::StartWorldNotLoaded
            | Self::DbError => Level::ERROR,
            Self::NameParseFailed
            | Self::MalformedArgs
            | Self::UnknownCharDef
            | Self::NoDbPool
            | Self::InvalidVisualGroup
            | Self::ForcedGroupChoice
            | Self::InvalidChoice
            | Self::MissingOptionalChoice
            | Self::StarterKitFailed => Level::WARN,
        }
    }
}

/// Who asked for a character, the account they asked from, and what they
/// asked for once it has been parsed. Built once in `create_character`; the
/// handler fills `player_name` and `char_def_id` as soon as it reads them, so
/// every later refusal names what was refused.
pub(super) struct Refusal<'a> {
    pub(super) addr: SocketAddr,
    pub(super) account_id: u32,
    pub(super) account_name: Option<&'a str>,
    /// The requested character name, once parsed (the old name, taken-name
    /// and commit rows carried it).
    pub(super) player_name: Option<String>,
    /// The requested CharDef, once parsed.
    pub(super) char_def_id: Option<i32>,
}

/// The per-site detail a refusal row carries beyond the request itself.
/// Every field is optional and is left off the row when absent.
#[derive(Debug, Default)]
pub(super) struct Detail<'d> {
    /// Why the name rule refused a name (`validate_character_name`).
    pub(super) rule: Option<&'d str>,
    /// The rejected extra name.
    pub(super) extra_name: Option<&'d str>,
    pub(super) skin_tint_color_id: Option<i32>,
    pub(super) vg_id: Option<i32>,
    pub(super) choice_id: Option<i32>,
    /// The row id the insert returned, for refusals after the insert.
    pub(super) player_id: Option<i32>,
    /// The start profile refused, and its world.
    pub(super) profile_id: Option<&'d str>,
    pub(super) world: Option<&'d str>,
    /// Every problem of a refused start profile, logged as `detail`.
    pub(super) profile_problems: Option<&'d str>,
    /// The starter-kit helper's own reason (`KitFailure`): `db_error`,
    /// `unknown_item`, `no_valid_container`, `all_valid_containers_full`.
    pub(super) kit_failure: Option<&'d str>,
    /// The `event` of the helper's ERROR row that has the precise cause.
    pub(super) cause: Option<&'static str>,
}

impl<'d> Detail<'d> {
    /// A starter-kit refusal: `cause` is the helper's event, `kit_failure`
    /// its reason, `player_id` the row that rolls back (if inserted).
    pub(super) fn kit(cause: &'static str, kit_failure: &'d str, player_id: Option<i32>) -> Self {
        Self {
            player_id,
            kit_failure: Some(kit_failure),
            cause: Some(cause),
            ..Self::default()
        }
    }
}

impl Refusal<'_> {
    /// Log the refusal with `reason`, the `onCharacterCreateFailed` code sent
    /// to the client, and the underlying `error`, when there is one.
    pub(super) fn log(&self, reason: Reason, error_code: i32, error: Option<&dyn Display>) {
        self.log_detail(reason, error_code, error, Detail::default());
    }

    /// [`Refusal::log`] with the call site's [`Detail`].
    pub(super) fn log_detail(
        &self,
        reason: Reason,
        error_code: i32,
        error: Option<&dyn Display>,
        detail: Detail<'_>,
    ) {
        let level = reason.level();
        let reason = reason.as_str();
        let error = error.map(tracing::field::display);
        let addr = self.addr;
        let account_id = self.account_id;
        let account_name = self.account_name;
        let player_name = self.player_name.as_deref();
        let char_def_id = self.char_def_id;
        let error_name = moniker(error_code);
        let Detail {
            rule,
            extra_name,
            skin_tint_color_id,
            vg_id,
            choice_id,
            player_id,
            profile_id,
            world,
            profile_problems,
            kit_failure,
            cause,
        } = detail;
        match level {
            Level::ERROR => tracing::error!(
                event = "character_create_failed",
                reason,
                %addr,
                account_id,
                account_name,
                player_id,
                player_name,
                extra_name,
                rule,
                char_def_id, // nt:id-only CharDef rows carry no name column to pair
                profile_id, // nt:id-only profile key has no display name
                world,
                detail = profile_problems,
                vg_id, // nt:id-only visual groups carry no name column to pair
                choice_id, // nt:id-only visual choices carry no name column to pair
                skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
                kit_failure,
                cause,
                error_code,
                error_name,
                error,
                "character creation refused"
            ),
            Level::INFO => tracing::info!(
                event = "character_create_failed",
                reason,
                %addr,
                account_id,
                account_name,
                player_id,
                player_name,
                extra_name,
                rule,
                char_def_id, // nt:id-only CharDef rows carry no name column to pair
                profile_id, // nt:id-only profile key has no display name
                world,
                detail = profile_problems,
                vg_id, // nt:id-only visual groups carry no name column to pair
                choice_id, // nt:id-only visual choices carry no name column to pair
                skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
                kit_failure,
                cause,
                error_code,
                error_name,
                error,
                "character creation refused"
            ),
            _ => tracing::warn!(
                event = "character_create_failed",
                reason,
                %addr,
                account_id,
                account_name,
                player_id,
                player_name,
                extra_name,
                rule,
                char_def_id, // nt:id-only CharDef rows carry no name column to pair
                profile_id, // nt:id-only profile key has no display name
                world,
                detail = profile_problems,
                vg_id, // nt:id-only visual groups carry no name column to pair
                choice_id, // nt:id-only visual choices carry no name column to pair
                skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
                kit_failure,
                cause,
                error_code,
                error_name,
                error,
                "character creation refused"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Regression guard (TG-DB-08 review).** Every reason has the spelling
    /// and level the convention doc's table gives it. Written out as
    /// literals, so changing a level or a spelling in `Reason` fails here,
    /// and every variant must appear, so a new reason needs a row.
    #[test]
    fn every_reason_has_its_documented_spelling_and_level() {
        let table = [
            (Reason::NameParseFailed, "name_parse_failed", Level::WARN),
            (Reason::NameRejected, "name_rejected", Level::INFO),
            (
                Reason::ExtraNameRejected,
                "extra_name_rejected",
                Level::INFO,
            ),
            (Reason::MalformedArgs, "malformed_args", Level::WARN),
            (Reason::InvalidSkinTint, "invalid_skin_tint", Level::INFO),
            (Reason::UnknownCharDef, "unknown_char_def", Level::WARN),
            (Reason::NoDbPool, "no_db_pool", Level::WARN),
            (Reason::QueryFailed, "query_failed", Level::ERROR),
            (
                Reason::InvalidVisualGroup,
                "invalid_visual_group",
                Level::WARN,
            ),
            (
                Reason::ForcedGroupChoice,
                "forced_group_choice",
                Level::WARN,
            ),
            (Reason::InvalidChoice, "invalid_choice", Level::WARN),
            (
                Reason::MissingOptionalChoice,
                "missing_optional_choice",
                Level::WARN,
            ),
            (
                Reason::StartProfilesLoadFailed,
                "start_profiles_load_failed",
                Level::ERROR,
            ),
            (Reason::NoStartProfile, "no_start_profile", Level::ERROR),
            (
                Reason::StartProfileProblem("empty_world"),
                "empty_world",
                Level::ERROR,
            ),
            (
                Reason::StartWorldUnknown,
                "start_world_unknown",
                Level::ERROR,
            ),
            (
                Reason::StartWorldNotLoaded,
                "start_world_not_loaded",
                Level::ERROR,
            ),
            (Reason::DbError, "db_error", Level::ERROR),
            (Reason::StarterKitFailed, "starter_kit_failed", Level::WARN),
            (Reason::NameTaken, "name_taken", Level::INFO),
            (Reason::DbWriteFailed, "db_write_failed", Level::ERROR),
        ];
        let mut covered = std::collections::BTreeSet::new();
        for (reason, spelling, level) in table {
            assert_eq!(reason.as_str(), spelling, "{reason:?}");
            assert_eq!(reason.level(), level, "{reason:?}");
            // Exhaustive: a new variant needs an arm here (it won't compile
            // otherwise), and the count below then needs its table row.
            covered.insert(match reason {
                Reason::NameParseFailed => 0,
                Reason::NameRejected => 1,
                Reason::ExtraNameRejected => 2,
                Reason::MalformedArgs => 3,
                Reason::InvalidSkinTint => 4,
                Reason::UnknownCharDef => 5,
                Reason::NoDbPool => 6,
                Reason::QueryFailed => 7,
                Reason::InvalidVisualGroup => 8,
                Reason::ForcedGroupChoice => 9,
                Reason::InvalidChoice => 10,
                Reason::MissingOptionalChoice => 11,
                Reason::StartProfilesLoadFailed => 12,
                Reason::NoStartProfile => 13,
                Reason::StartProfileProblem(_) => 14,
                Reason::StartWorldUnknown => 15,
                Reason::StartWorldNotLoaded => 16,
                Reason::DbError => 17,
                Reason::StarterKitFailed => 18,
                Reason::NameTaken => 19,
                Reason::DbWriteFailed => 20,
            });
        }
        assert_eq!(covered.len(), 21, "every Reason variant has a table row");
        // Every spelling is distinct.
        let mut spellings: Vec<&str> = table.iter().map(|(_, s, _)| *s).collect();
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), table.len());
    }
}
