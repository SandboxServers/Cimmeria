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
pub(super) async fn open(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    crate::cell::interactions::open_vault_gm(caller_id, tx, space_mgr).await;
    let text = if space_mgr
        .get_entity(caller_id)
        .is_some_and(|e| e.vault_session.is_some())
    {
        "bank: personal vault opened (GM session, no Banker; any interact closes it)"
    } else {
        "bank: could not open the vault -- this entity is not in a space"
    };
    send_gm_feedback(caller_id, text, tx).await;
}

/// Is `text` a `.bank` line? Matched on the command word only, so
/// `.bank now` counts and `.banker` does not.
pub(crate) fn is_bank_command(text: &str) -> bool {
    text.strip_prefix('.')
        .and_then(|body| body.split_whitespace().next())
        .is_some_and(|name| name.eq_ignore_ascii_case("bank"))
}

/// A non-GM typed `.bank`: say why nothing opened, and log it. The line is
/// consumed, never broadcast.
pub(crate) async fn refuse_non_gm(entity_id: u32, tx: &mpsc::Sender<CellToBaseMsg>) {
    tracing::info!(
        target: "bank",
        event = "vault_open_rejected",
        entity_id,
        reason = "not_gm",
        "vault_open_rejected: .bank needs GM access"
    );
    send_gm_feedback(
        entity_id,
        ".bank needs GM access. Visit a Banker to open your vault.",
        tx,
    )
    .await;
}
