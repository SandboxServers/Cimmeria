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
//! `.bankdump [player]` (BV-04) lists a character's personal vault
//! (container 17) to the GM, read-only. The cell has no DB pool, so it
//! forwards the request to the base (`BankCellToBase::GmDump`), which reads
//! the vault, answers on the feedback channel and logs the `gm_action`
//! event. With no name it lists the GM's own vault; a name is matched
//! exactly by the base, so an offline character works too.
//!
//! `.bankexpand` (BV-05) buys one +10 step of the GM's own vault through
//! the Banker dialog's purchase path, with an open vault session required.
//!
//! `.orgvaultexpand [team|command] [from_slots]` (BV-09, D-BV28) quotes, or
//! with the current size buys, one +10 step of the GM's Team vault, paid
//! from the Team treasury. The Team's leader only, checked by the base
//! under the organization lock. There is no client UI for it: the Expand
//! dialog is quarantined (#943). No vault session is needed: the purchase
//! moves no item and the leader check is the authorization.
//!
//! New in the Rust server; the legacy python console had no equivalent.

use cimmeria_entity::cell_entity::VaultScope;
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::{BankCellToBase, BankSubject, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// `.bank`: open the personal vault on the caller, then confirm on the
/// feedback channel.
#[tracing::instrument(name = "bank.console_open", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn open(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let text = if crate::cell::interactions::open_vault_gm(caller_id, tx, space_mgr).await {
        "bank: personal vault opened (GM session, no Banker; any interact closes it)"
    } else {
        "bank: could not open the vault -- this entity is not in a space"
    };
    send_gm_feedback(caller_id, text, tx).await;
}

/// `.bankdump [player]`: hand the read to the base, which answers the GM.
///
/// The only refusals decided here are the ones the base cannot see: a bare
/// `.bankdump` from an entity with no character, and a dead base channel.
/// Each logs `gm_action action=bankdump result=refused` with its reason.
#[tracing::instrument(name = "bank.console_dump", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn dump(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    let subject = match (args.first(), id.player_id) {
        (Some(name), _) => BankSubject::Name((*name).to_string()),
        (None, Some(player_id)) => BankSubject::Player(player_id),
        (None, None) => {
            tracing::info!(
                target: "bank",
                event = "gm_action",
                action = "bankdump",
                result = "refused",
                reason = "caller_not_player",
                account_id = id.account_id,
                account_name = id.account_name,
                entity_id = caller_id,
                entity_name = id.player_name,
                "gm_action: bankdump refused",
            );
            send_gm_feedback(
                caller_id,
                "bankdump: you have no character id; name a player instead",
                tx,
            )
            .await;
            return;
        }
    };
    let target_player_id = match &subject {
        BankSubject::Player(id) => Some(*id),
        BankSubject::Name(_) => None,
    };
    // The subject's name for the failure row: the caller's own, or the name
    // the GM typed. Owned because `subject` moves into the message.
    let target_player_name = match &subject {
        BankSubject::Player(_) => id.player_name.map(str::to_owned),
        BankSubject::Name(name) => Some(name.clone()),
    };
    let msg = CellToBaseMsg::Bank(BankCellToBase::GmDump {
        entity_id: caller_id,
        account_id: id.account_id,
        player_id: id.player_id,
        subject,
    });
    if let Err(e) = tx.send(msg).await {
        tracing::warn!(
            target: "bank",
            event = "gm_action",
            action = "bankdump",
            result = "refused",
            reason = "base_channel_closed",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id = caller_id,
            entity_name = id.player_name,
            target_player_id,
            target_player_name = target_player_name.as_deref(),
            error = %e,
            "gm_action: bankdump could not reach the base",
        );
    }
}

/// Is `text` a `.bankdump` line? Matched on the command word only.
pub(crate) fn is_bankdump_command(text: &str) -> bool {
    text.strip_prefix('.')
        .and_then(|body| body.split_whitespace().next())
        .is_some_and(|name| name.eq_ignore_ascii_case("bankdump"))
}

/// A non-GM typed `.bankdump`: `gm_action result=refused reason=not_gm` on
/// the `bank` target. The caller still sends the generic "is a GM command"
/// line, and the text is never broadcast.
pub(crate) fn log_non_gm_bankdump(entity_id: u32, space_mgr: &SpaceManager) {
    let id = space_mgr.player_identity(entity_id);
    tracing::info!(
        target: "bank",
        event = "gm_action",
        action = "bankdump",
        result = "refused",
        reason = "not_gm",
        account_id = id.account_id,
        account_name = id.account_name,
        player_id = id.player_id,
        player_name = id.player_name,
        entity_id,
        entity_name = id.player_name,
        "gm_action: bankdump refused",
    );
}

/// `.bankexpand`: buy one vault step for the GM through the purchase path
/// the Banker's dialog uses (BV-05). The Expand dialog is quarantined
/// (#943), so this is the only way to expand a vault until it is served.
/// The result line comes from the base, after the purchase commits or is
/// refused.
pub(super) async fn expand(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    crate::cell::interactions::gm_expand_vault(caller_id, tx, space_mgr).await;
}

/// The usage line for a `.orgvaultexpand` it cannot parse.
pub(crate) const ORG_EXPAND_USAGE: &str =
    "orgvaultexpand: usage .orgvaultexpand [team|command] [from_slots]. Nothing was charged.";

/// `.orgvaultexpand [team|command] [from_slots]`: parse the arguments and
/// hand the quote or the purchase to the base, which answers the GM.
///
/// Refused here, with WARN `expand_rejected` (`scope`, `trigger=gm_console`)
/// and a line: arguments it cannot parse (`bad_args`), an entity with no
/// character (`player_missing`), and a dead base channel
/// (`base_channel_closed`).
#[tracing::instrument(name = "bank.console_org_expand", level = "info", skip_all, fields(entity_id = caller_id))]
pub(super) async fn org_expand(
    caller_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(caller_id);
    let refuse = |reason: &'static str, scope: Option<VaultScope>| {
        tracing::warn!(
            target: "bank",
            event = "expand_rejected",
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            entity_id = caller_id,
            entity_name = id.player_name,
            scope = scope.map(VaultScope::as_str),
            reason,
            trigger = "gm_console",
            "expand_rejected: .orgvaultexpand refused on the cell -- nothing bought",
        );
    };
    let Some((scope, from_slots)) = parse_org_expand_args(args) else {
        refuse("bad_args", None);
        send_gm_feedback(caller_id, ORG_EXPAND_USAGE, tx).await;
        return;
    };
    let Some(player_id) = id.player_id else {
        refuse("player_missing", Some(scope));
        send_gm_feedback(
            caller_id,
            "orgvaultexpand: you have no character id. Nothing was charged.",
            tx,
        )
        .await;
        return;
    };
    let msg = CellToBaseMsg::Bank(BankCellToBase::OrgVaultExpand {
        entity_id: caller_id,
        account_id: id.account_id,
        player_id,
        scope,
        from_slots,
    });
    if tx.send(msg).await.is_err() {
        // No feedback line: it would go down the same closed channel.
        refuse("base_channel_closed", Some(scope));
    }
}

/// `[team|command] [from_slots]`, in either order: the scope defaults to
/// `Team`, the size to none (a quote). `None` for anything else.
pub(crate) fn parse_org_expand_args(args: &[&str]) -> Option<(VaultScope, Option<i16>)> {
    let (mut scope, mut from) = (None, None);
    for arg in args {
        if arg.eq_ignore_ascii_case("team") && scope.is_none() {
            scope = Some(VaultScope::Team);
        } else if arg.eq_ignore_ascii_case("command") && scope.is_none() {
            scope = Some(VaultScope::Command);
        } else if let (Ok(n), None) = (arg.parse::<i16>(), from) {
            from = Some(n);
        } else {
            return None;
        }
    }
    Some((scope.unwrap_or(VaultScope::Team), from))
}

/// Is `text` a `.orgvaultexpand` line? Matched on the command word only.
pub(crate) fn is_orgvaultexpand_command(text: &str) -> bool {
    text.strip_prefix('.')
        .and_then(|body| body.split_whitespace().next())
        .is_some_and(|name| name.eq_ignore_ascii_case("orgvaultexpand"))
}

/// A non-GM typed `.orgvaultexpand`: WARN `expand_rejected reason=not_gm
/// scope=team` and a line. The chat line is consumed, never broadcast.
pub(crate) async fn refuse_non_gm_org_expand(
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
        scope = VaultScope::Team.as_str(),
        reason = "not_gm",
        trigger = "gm_console",
        "expand_rejected: .orgvaultexpand from a player without GM access -- nothing bought",
    );
    send_gm_feedback(
        entity_id,
        ".orgvaultexpand needs GM access. Nothing was charged.",
        tx,
    )
    .await;
}

/// Is `text` a `.bankexpand` line? Matched on the command word only.
pub(crate) fn is_bankexpand_command(text: &str) -> bool {
    text.strip_prefix('.')
        .and_then(|body| body.split_whitespace().next())
        .is_some_and(|name| name.eq_ignore_ascii_case("bankexpand"))
}

/// A non-GM typed `.bankexpand`: the bank-specific refusal.
pub(crate) async fn refuse_non_gm_expand(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    crate::cell::interactions::refuse_non_gm_expand(entity_id, tx, space_mgr).await;
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
        tx,
        space_mgr,
    )
    .await;
}
