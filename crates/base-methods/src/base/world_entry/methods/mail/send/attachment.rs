//! What a send attaches (SS-M2): the checks that need no database, and the
//! refusal every attachment gate answers with.
//!
//! The client runs the same checks before it sends (SS-E1 M-Q2: COD needs an
//! item and a price above zero; the cash spinner stops at the balance minus
//! postage), so a refusal here means a modified client or a stale UI. It is
//! still answered, with a result code and a feedback line, like any other.

use crate::cell::mail::codes::{flags::MAIL_COD, MailResult};
use crate::cell::messages::MailSend;

/// Postage for a send with cash or an item attached, in naquadah (D-SS02:
/// `GateMailMod.attachmentCost = 25`, `GateMail.lua:10`). A sink: it is
/// debited from the sender and credited to nobody. Project policy recovered
/// from the client's UI, not a server constant.
pub(in super::super) const POSTAGE: i64 = 25;

/// The item a send asks to attach: an inventory instance id (SS-E1 M-Q2)
/// and how many of its stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct ItemRequest {
    pub(in super::super) item_id: i32,
    pub(in super::super) quantity: i32,
}

/// A validated attachment. `cash` is a gift, or with `cod` the price the
/// recipient pays (D-SS09); never negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct Attachment {
    pub(in super::super) cash: i32,
    pub(in super::super) cod: bool,
    pub(in super::super) item: Option<ItemRequest>,
}

impl Attachment {
    /// What the sender pays now: postage, plus the gift cash. A COD price is
    /// not the sender's money; the recipient pays it later (SS-M3). In `i64`
    /// so `i32::MAX` cash plus postage cannot wrap.
    pub(in super::super) fn sender_cost(&self) -> i64 {
        let gift = if self.cod { 0 } else { i64::from(self.cash) };
        POSTAGE + gift
    }

    /// The mail row's `flags`.
    pub(in super::super) fn mail_flags(&self) -> i32 {
        if self.cod {
            MAIL_COD
        } else {
            0
        }
    }
}

/// Why an attachment was refused: the result code, a stable `reason` log
/// value and the feedback line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct AttachmentRefusal {
    pub(in super::super) result: MailResult,
    pub(in super::super) reason: &'static str,
    pub(in super::super) text: &'static str,
}

impl AttachmentRefusal {
    pub(in super::super) const fn new(
        result: MailResult,
        reason: &'static str,
        text: &'static str,
    ) -> Self {
        Self {
            result,
            reason,
            text,
        }
    }
}

/// The item is not in the sender's main bag, not theirs, or gone.
pub(in super::super) const ITEM_NOT_FOUND: AttachmentRefusal = AttachmentRefusal::new(
    MailResult::ItemNotAvailable,
    "item_not_owned",
    "The attached item is no longer in your bags. The message was not sent.",
);
/// Equipped, bandolier and mission items stay put (the crafting bag is a
/// mail source since 2026-09-27; `escrow::MAILABLE_CONTAINERS`).
pub(in super::super) const ITEM_NOT_IN_MAIN_BAG: AttachmentRefusal = AttachmentRefusal::new(
    MailResult::ItemNotAvailable,
    "item_not_in_main_bag",
    "Only items in your main bag or crafting bag can be sent by gate-mail. \
     The message was not sent.",
);
/// A vault item (personal 17, auction 18, team 19, command 20). Owner
/// decision 2026-09-27 (Bank campaign): vendors, trade, crafting and mail
/// see only the backpack.
pub(in super::super) const ITEM_IN_VAULT: AttachmentRefusal = AttachmentRefusal::new(
    MailResult::ItemNotAvailable,
    "item_in_vault",
    "Items in a vault cannot be sent by gate-mail. Move it to your backpack first. \
     The message was not sent.",
);
/// A buyback item (16): still the vendor's until bought back.
pub(in super::super) const ITEM_IN_BUYBACK: AttachmentRefusal = AttachmentRefusal::new(
    MailResult::ItemNotAvailable,
    "item_in_buyback",
    "Items on a vendor's buyback list cannot be sent by gate-mail. \
     The message was not sent.",
);
pub(in super::super) const ITEM_BOUND: AttachmentRefusal = AttachmentRefusal::new(
    MailResult::ItemNotAvailable,
    "item_bound",
    "Bound items cannot be sent by gate-mail. The message was not sent.",
);
pub(in super::super) const ITEM_QUANTITY_EXCEEDS_STACK: AttachmentRefusal = AttachmentRefusal::new(
    MailResult::ItemNotAvailable,
    "item_quantity_exceeds_stack",
    "You do not have that many of the attached item. The message was not sent.",
);
/// D-SS02: cash plus postage is more than the sender holds.
pub(in super::super) const NOT_ENOUGH_CASH: AttachmentRefusal = AttachmentRefusal::new(
    MailResult::NotEnoughCash,
    "not_enough_cash",
    "You do not have enough naquadah for the attached naquadah plus 25 postage. \
     The message was not sent.",
);

/// Check the attachment fields of `send` that need no database. `Ok(None)`
/// is a text-only mail.
pub(in super::super) fn validate(send: &MailSend) -> Result<Option<Attachment>, AttachmentRefusal> {
    if !send.has_attachment() {
        return Ok(None);
    }
    if send.cash < 0 {
        return Err(AttachmentRefusal::new(
            MailResult::NoRecipients,
            "negative_cash",
            "The attached naquadah cannot be negative. The message was not sent.",
        ));
    }
    if send.item_id == 0 && send.item_quantity != 0 {
        return Err(AttachmentRefusal::new(
            MailResult::NoRecipients,
            "item_quantity_without_item",
            "Your gate-mail message could not be read. It was not sent.",
        ));
    }
    if send.item_id != 0 && send.item_quantity <= 0 {
        return Err(AttachmentRefusal::new(
            MailResult::ItemNotAvailable,
            "invalid_item_quantity",
            "The attached item quantity is not valid. The message was not sent.",
        ));
    }
    // D-SS09, both directions, as the client itself refuses them.
    if send.cod && send.item_id == 0 {
        return Err(AttachmentRefusal::new(
            MailResult::ItemNotAvailable,
            "cod_without_item",
            "A COD gate-mail message needs an attached item. It was not sent.",
        ));
    }
    if send.cod && send.cash <= 0 {
        return Err(AttachmentRefusal::new(
            MailResult::NoRecipients,
            "cod_without_price",
            "A COD gate-mail message needs a price above zero. It was not sent.",
        ));
    }
    Ok(Some(Attachment {
        cash: send.cash,
        cod: send.cod,
        item: (send.item_id != 0).then_some(ItemRequest {
            item_id: send.item_id,
            quantity: send.item_quantity,
        }),
    }))
}
