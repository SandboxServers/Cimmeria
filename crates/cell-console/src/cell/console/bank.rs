//! `.bank` — open the GM's own personal vault anywhere (bank-vault BV-02,
//! D-BV03).
//!
//! UAT needs to reach the bank without walking to a Banker. The session it
//! opens has no Banker (`banker_id: None`), so bank moves skip the
//! proximity check; any later `interact` ends it, like a Banker session.
//! The client's `isBankingOverride` property is not used: nothing reads it
//! (audit A-12).
//!
//! GM-only, like every `.`-command. A non-GM who types `.bank` gets a
//! refusal line instead of having it broadcast as chat
//! ([`refuse_non_gm`], called from the chat interceptor): a player who
//! reads about the command should be told why nothing opened.
//!
//! New in the Rust server; the legacy python console had no equivalent.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.bank`: open the personal vault on the caller, then confirm on the
/// feedback channel.
#[tracing::instrument(name = "bank.console_open", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn open(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // A failure has already been refused, with its own line and log.
    if crate::cell::interactions::open_vault_gm(caller_id, tx, space_mgr).await {
        send_gm_feedback(
            caller_id,
            "bank: personal vault opened (GM session, no Banker; any interact closes it)",
            tx,
        )
        .await;
    }
}

/// Is `text` a `.bank` line? Matched on the command word only, so
/// `.bank now` counts and `.banker` does not.
pub(crate) fn is_bank_command(text: &str) -> bool {
    text.strip_prefix('.')
        .and_then(|body| body.split_whitespace().next())
        .is_some_and(|name| name.eq_ignore_ascii_case("bank"))
}

/// A non-GM typed `.bank`: `vault_open_rejected reason=not_gm` and a line
/// saying why nothing opened. The chat line is consumed, never broadcast.
#[tracing::instrument(
    name = "bank.console_open",
    level = "info",
    skip_all,
    fields(entity_id)
)]
pub(crate) async fn refuse_non_gm(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    crate::cell::interactions::reject_vault_open(
        entity_id,
        crate::cell::interactions::VaultOpenReject::NotGm,
        None,
        None,
        "",
        tx,
        space_mgr,
    )
    .await;
}
