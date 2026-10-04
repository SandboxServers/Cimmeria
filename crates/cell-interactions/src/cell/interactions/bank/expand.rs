//! Buying vault space at the Banker: the cell half of BV-05 (D-BV02).
//!
//! The vault grows from 40 to 100 slots in steps of 10, each step bought at
//! a Banker for the naquadah price in `resources.bank_expansion_price`.
//! `sgw_player.bank_slots` and the cash belong to the base, and the cell has
//! no database pool, so the flow is two round trips:
//!
//! 1. **Quote.** A personal vault opens (Banker click or GM `.bank`), and
//!    [`request_expansion_quote`] asks the base whether the vault can grow.
//!    Below the ceiling the base answers with `OfferExpansion`, and
//!    [`offer_vault_expansion`] records the size and the price it was
//!    offered at on the vault session and shows the one-button Expand dialog
//!    ([`VAULT_EXPAND_DIALOG_ID`]), plus a chat line with the price.
//! 2. **Purchase.** The dialog's reply reaches [`answer_vault_expansion`]
//!    after the #479 offered-dialog gate. It takes the recorded offer
//!    (one-shot) and a **fresh** vault verdict ([`vault_access`], the rule
//!    every bank move takes), and forwards both to the base, which buys in
//!    one statement or refuses with a reason and a chat line.
//!
//! The dialog button is not an authority check: `dialogButtonChoice`
//! carries only ids the client chose. What decides a purchase is the fresh
//! verdict (session open, same space, Banker in range, or a GM session),
//! then the offered size, the cash and the ceiling, all on the server.
//!
//! Every event logs under the `bank` target with the D-BV19 correlators.
//! The purchase refusals (`expand_rejected`) are logged by the base, which
//! can read the cash and `bank_slots` for them; the cell logs only what the
//! base cannot see.

use tokio::sync::mpsc;

use cimmeria_entity::cell_entity::{ExpansionOffer, VaultScope};
use cimmeria_wire::cell::messages::{BankCellToBase, ExpandTrigger};
use cimmeria_wire::cell::vault::{
    VAULT_EXPAND_BUTTON_ID, VAULT_EXPAND_DIALOG_ID, VAULT_EXPAND_DIALOG_SERVED, VAULT_EXPAND_STEP,
};

use super::rejection::send_bank_feedback;
use super::vault_access;
use crate::cell::interactions::send_dialog_display;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// A personal vault just opened for `entity_id`: ask the base whether to
/// offer the next expansion. `speaker_id` is the Banker, or the player's
/// own entity for a GM `.bank` session.
pub(super) async fn request_expansion_quote(
    entity_id: u32,
    speaker_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let Some(player_id) = id.player_id else {
        // An entity with no character (a test fixture, a bot) cannot own a
        // vault row, so there is nothing to quote.
        tracing::debug!(
            target: "bank",
            event = "expand_quote_skipped",
            account_id = id.account_id,
            account_name = id.account_name,
            entity_id,
            entity_name = id.player_name,
            reason = "no_player_id",
            "expand_quote_skipped: the entity has no character id, no expansion offer"
        );
        return;
    };
    let msg = CellToBaseMsg::Bank(BankCellToBase::ExpansionQuote {
        entity_id,
        account_id: id.account_id,
        player_id,
        speaker_id,
    });
    if let Err(e) = tx.send(msg).await {
        tracing::warn!(
            target: "bank",
            event = "expand_quote_send_failed",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id,
            player_name = id.player_name,
            entity_id,
            entity_name = id.player_name,
            reason = "base_channel_closed",
            error = %e,
            "expand_quote_send_failed: the quote could not reach the base -- the vault is \
             open but no Expand dialog will be offered"
        );
    }
}

/// The base's `OfferExpansion`: record the offer on the vault session and,
/// while the dialog is served ([`VAULT_EXPAND_DIALOG_SERVED`]), show it
/// ([`show_expand_offer`]). While it is quarantined the offer is recorded
/// and DEBUG `expand_offer_suppressed reason=dialog_quarantined` is logged
/// once per open instead.
///
/// The offer is dropped (DEBUG `expand_offer_dropped`) when the entity is
/// gone or is another character, or when the personal vault session the
/// quote was asked for has ended or moved to another speaker in the
/// meantime: that player has walked away or clicked something else, and a
/// dialog now would offer a purchase the next check refuses.
pub async fn offer_vault_expansion(
    entity_id: u32,
    player_id: i32,
    speaker_id: u32,
    from_slots: i16,
    price: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let drop_reason = match space_mgr.get_entity_mut(entity_id) {
        None => Some("entity_missing"),
        Some(p) if p.player_id != Some(player_id) => Some("entity_is_another_player"),
        Some(p) => match p.vault_session.as_mut() {
            None => Some("no_vault_session"),
            Some(s) if s.scope != VaultScope::Personal => Some("vault_scope_mismatch"),
            // A GM session speaks through the player's own entity.
            Some(s) if s.banker_id.unwrap_or(entity_id) != speaker_id => Some("speaker_changed"),
            Some(s) => {
                s.expansion_offer = Some(ExpansionOffer { from_slots, price });
                None
            }
        },
    };
    if let Some(reason) = drop_reason {
        tracing::debug!(
            target: "bank",
            event = "expand_offer_dropped",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id,
            // The entity may now play another character: name the offer's.
            player_name = (id.player_id == Some(player_id))
                .then_some(id.player_name)
                .flatten(),
            entity_id,
            entity_name = id.player_name,
            speaker_id,
            speaker_name = space_mgr.entity_label(speaker_id),
            bank_slots = from_slots,
            reason,
            "expand_offer_dropped: the vault session changed before the offer arrived -- \
             no Expand dialog"
        );
        return;
    }
    if !VAULT_EXPAND_DIALOG_SERVED {
        // The client has no entry for 60110 while it is quarantined (#943),
        // so a display would show nothing. The offer is still recorded, so
        // lifting the quarantine needs no other change here.
        tracing::debug!(
            target: "bank",
            event = "expand_offer_suppressed",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id,
            player_name = id.player_name,
            entity_id,
            entity_name = id.player_name,
            speaker_id,
            speaker_name = space_mgr.entity_label(speaker_id),
            bank_slots = from_slots,
            price,
            reason = "dialog_quarantined",
            "expand_offer_suppressed: the Expand dialog is quarantined -- no dialog shown; \
             a GM buys with .bankexpand"
        );
        return;
    }
    show_expand_offer(
        entity_id,
        speaker_id,
        ExpansionOffer { from_slots, price },
        tx,
        space_mgr,
    )
    .await;
}

/// Show a recorded offer: `onDialogDisplay(speaker, 60110)`, which also
/// records the #479 offered-dialog gate, then the chat line with the price.
/// [`offer_vault_expansion`] calls it only while the dialog is served.
pub async fn show_expand_offer(
    entity_id: u32,
    speaker_id: u32,
    offer: ExpansionOffer,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let gm_override = speaker_id == entity_id;
    let ExpansionOffer { from_slots, price } = offer;
    tracing::debug!(
        target: "bank",
        event = "expand_offered",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id,
        entity_name = id.player_name,
        banker_id = (!gm_override).then_some(speaker_id),
        banker_name = (!gm_override)
            .then(|| space_mgr.entity_label(speaker_id))
            .flatten(),
        gm_override,
        bank_slots = from_slots,
        price,
        "expand_offered: Expand dialog shown"
    );
    send_dialog_display(
        entity_id,
        speaker_id as i32,
        VAULT_EXPAND_DIALOG_ID,
        tx,
        space_mgr,
    )
    .await;
    let text = format!(
        "Your vault has {from_slots} slots. {VAULT_EXPAND_STEP} more cost {price} naquadah: \
         press Expand vault in the Banker's dialog to buy them."
    );
    send_bank_feedback(entity_id, &text, tx, space_mgr).await;
}

/// The player answered the Expand dialog. Only the authored button
/// ([`VAULT_EXPAND_BUTTON_ID`]) buys: a close sends `-1`, and any other id
/// is not a press of the button the player was shown, so neither is ever a
/// purchase (DEBUG `expand_dismissed`). A press goes to the base with the
/// recorded offer and a fresh verdict, and the base decides.
#[tracing::instrument(
    name = "bank.expand",
    level = "info",
    skip_all,
    fields(entity_id, button_id)
)]
pub async fn answer_vault_expansion(
    entity_id: u32,
    button_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if button_id != VAULT_EXPAND_BUTTON_ID {
        let id = space_mgr.player_identity(entity_id);
        tracing::debug!(
            target: "bank",
            event = "expand_dismissed",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id,
            entity_name = id.player_name,
            button_id, // nt:id-only an authored dialog button ordinal; no name table holds it
            reason = if button_id == -1 { "closed" } else { "unexpected_button" },
            "expand_dismissed: the Expand dialog was answered without its button -- nothing bought"
        );
        return;
    }
    request_expansion(entity_id, ExpandTrigger::Dialog, tx, space_mgr).await;
}

/// GM `.bankexpand`: buy one step through the same purchase path as the
/// dialog, with no offer (the base quotes the current step itself). The
/// caller has passed the GM gate on the server-side `access_level`. It
/// needs an open personal vault session, and a GM `.bank` session skips the
/// proximity check exactly as it does for moves.
#[tracing::instrument(
    name = "bank.console_expand",
    level = "info",
    skip_all,
    fields(entity_id)
)]
pub async fn gm_expand_vault(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    request_expansion(entity_id, ExpandTrigger::GmConsole, tx, space_mgr).await;
}

/// Forward one purchase request with a fresh verdict. The dialog takes the
/// session's offer (one-shot); the console sends none.
async fn request_expansion(
    entity_id: u32,
    trigger: ExpandTrigger,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let Some(player_id) = id.player_id else {
        tracing::warn!(
            target: "bank",
            event = "expand_rejected",
            account_id = id.account_id,
            account_name = id.account_name,
            entity_id,
            entity_name = id.player_name,
            reason = "player_missing",
            trigger = trigger.as_str(),
            "expand_rejected: the entity has no character id -- nothing bought"
        );
        send_bank_feedback(
            entity_id,
            "Your vault could not be expanded. Nothing was charged.",
            tx,
            space_mgr,
        )
        .await;
        return;
    };
    // The verdict first, then the take: both read the session as it is now.
    let vault = vault_access(entity_id, space_mgr);
    let offer = match trigger {
        ExpandTrigger::Dialog => space_mgr
            .get_entity_mut(entity_id)
            .and_then(|p| p.vault_session.as_mut())
            .and_then(|s| s.expansion_offer.take()),
        ExpandTrigger::GmConsole => None,
    };
    let msg = CellToBaseMsg::Bank(BankCellToBase::Expand {
        entity_id,
        account_id: id.account_id,
        player_id,
        offer,
        vault,
        trigger,
    });
    if let Err(e) = tx.send(msg).await {
        // No feedback line: it would go down the same closed channel.
        tracing::warn!(
            target: "bank",
            event = "expand_rejected",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id,
            player_name = id.player_name,
            entity_id,
            entity_name = id.player_name,
            reason = "base_channel_closed",
            trigger = trigger.as_str(),
            bank_slots = offer.map(|o| o.from_slots),
            price = offer.map(|o| o.price),
            error = %e,
            "expand_rejected: the purchase could not reach the base -- nothing bought, and \
             the player cannot be told"
        );
    }
}

/// A non-GM typed `.bankexpand`: WARN `expand_rejected reason=not_gm
/// trigger=gm_console` and a line saying why. The chat line is consumed,
/// never broadcast.
pub async fn refuse_non_gm_expand(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    tracing::warn!(
        target: "bank",
        event = "expand_rejected",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id,
        entity_name = id.player_name,
        reason = "not_gm",
        trigger = ExpandTrigger::GmConsole.as_str(),
        "expand_rejected: .bankexpand from a player without GM access -- nothing bought"
    );
    send_bank_feedback(
        entity_id,
        ".bankexpand needs GM access. Nothing was charged.",
        tx,
        space_mgr,
    )
    .await;
}
