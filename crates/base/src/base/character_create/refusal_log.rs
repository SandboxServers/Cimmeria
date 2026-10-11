//! The `character_create_failed` row: one line for every refused
//! `createCharacter`, carrying the stable `reason` of the call site (TG-DB-08).
//! The level follows the reason, as the handler's own lines did before.

use std::fmt::Display;
use std::net::SocketAddr;

use tracing::Level;

use super::fail_code::moniker;

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
}

impl Refusal<'_> {
    /// Log the refusal with `reason` (a snake_case name from the closed list
    /// for the call site), the `onCharacterCreateFailed` code sent to the
    /// client, and the underlying `error`, when there is one.
    pub(super) fn log(&self, reason: &'static str, error_code: i32, error: Option<&dyn Display>) {
        self.log_detail(reason, error_code, error, Detail::default());
    }

    /// [`Refusal::log`] with the call site's [`Detail`].
    pub(super) fn log_detail(
        &self,
        reason: &'static str,
        error_code: i32,
        error: Option<&dyn Display>,
        detail: Detail<'_>,
    ) {
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
        } = detail;
        match refusal_level(reason) {
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
                vg_id, // nt:id-only visual groups carry no name column to pair
                choice_id, // nt:id-only visual choices carry no name column to pair
                skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
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
                vg_id, // nt:id-only visual groups carry no name column to pair
                choice_id, // nt:id-only visual choices carry no name column to pair
                skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
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
                vg_id, // nt:id-only visual groups carry no name column to pair
                choice_id, // nt:id-only visual choices carry no name column to pair
                skin_tint_color_id, // nt:id-only palette index 0-15, not a named row
                error_code,
                error_name,
                error,
                "character creation refused"
            ),
        }
    }
}

/// The level each refusal logged at before TG-DB-08: the name and skin-tint
/// refusals are INFO, a taken name is INFO, a database failure is ERROR, and
/// everything else is WARN.
fn refusal_level(reason: &str) -> Level {
    match reason {
        "name_rejected" | "extra_name_rejected" | "invalid_skin_tint" | "name_taken" => Level::INFO,
        "query_failed" | "db_write_failed" => Level::ERROR,
        _ => Level::WARN,
    }
}
