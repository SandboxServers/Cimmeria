//! `chatIgnore` (SGWPlayer base method 0xC5): add a name to, or remove it
//! from, the caller's own contact-list Ignore list (SS-C1, D-SS15).
//!
//! `chatIgnore(WSTRING aPlayerName, UINT8 aFlag)` per `Communicator.def`:
//! flag 1 ignores, 0 stops ignoring. The edit goes through the contact-list
//! member ops, so the client's contact-list window gets the same
//! `onContactListAddMembers` / `onContactListRemoveMembers` echo as a UI
//! edit, and the member ops reload the Ignore cache on the session and the
//! cell. The list is always the caller's: the list id comes from
//! `ensure_system_lists(player_id)` with the session's `player_id`, never
//! from the payload (CAT-L-07).
//!
//! An add stores the canonical name of a real character (D-SS13 against
//! `sgw_player`, offline characters included), refuses the caller's own
//! character, and stops at [`MAX_IGNORE_LIST_MEMBERS`] (CAT-L-04's cap). A
//! remove matches the typed name against the list itself, so a name whose
//! character was since deleted can still be removed. Every outcome, refusals
//! included, is one feedback line.
//!
//! Each call spends a token from the chat bucket (D-SS14) before anything
//! else, so `chatIgnore` cannot be used to hammer the database; an
//! over-limit call gets the usual "too quickly" line at most once per 5 s.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_base_session::base::session_identity::session_identity;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::super::contact_list::handlers::{announce_added_members, handle_remove_members};
use super::super::contact_list::ignore::{
    add_ignore_entry, ensure_ignore_list, load_ignore_names, match_name, resolve_character,
    CharacterLookup, IgnoreAdd, NameMatch, MAX_IGNORE_LIST_MEMBERS,
};
use super::super::feedback::{send_feedback_line, FeedbackCtx};
use super::super::rate_limit::limits::CHAT_EXEMPT_ACCESS_LEVEL;
use super::super::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use super::super::ConnectedClientState;
use cimmeria_entity::organization::org_text::{validate, TextField};

/// Longest prefix of a typed name echoed back or logged.
const SHOWN_NAME_CHARS: usize = 64;

pub(super) const IGNORE_NO_TARGET_TEXT: &str = "Name a player to ignore.";
pub(super) const IGNORE_SELF_TEXT: &str = "You cannot ignore yourself.";
pub(super) const IGNORE_UNAVAILABLE_TEXT: &str =
    "Your Ignore list cannot be changed right now. Try again later.";
pub(super) const IGNORE_BAD_REQUEST_TEXT: &str = "That Ignore request could not be read.";
pub(super) const IGNORE_BAD_NAME_TEXT: &str = "That is not a valid character name.";

pub(super) fn ignore_full_text() -> String {
    format!("Your Ignore list is full ({MAX_IGNORE_LIST_MEMBERS} names). Remove someone first.")
}

fn shown(name: &str) -> String {
    name.chars().take(SHOWN_NAME_CHARS).collect()
}

/// The caller, from server session state.
#[derive(Debug, Clone, Copy)]
struct Caller {
    player_id: i32,
    account_id: u32,
    entity_id: u32,
    /// The session's names, for the log lines (Rule 6).
    identity: PlayerIdentity,
}

/// Handle `chatIgnore`. Every path ends in one feedback line, except a rate
/// limited call after the first in its 5 s notify window.
#[tracing::instrument(name = "chat.ignore", level = "info", skip_all, fields(peer = %addr))]
#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_chat_ignore(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
    now: Instant,
) {
    let feedback = FeedbackCtx {
        transport,
        connected,
    };

    let (caller, decision) = {
        let mut clients = connected.lock().unwrap();
        match clients.get_mut(&addr) {
            Some(c) => {
                let caller =
                    c.active_player_id
                        .zip(c.player_entity_id)
                        .map(|(player_id, entity_id)| Caller {
                            player_id,
                            account_id: c.account_id,
                            entity_id,
                            identity: session_identity(c),
                        });
                let decision = if caller.is_none() || c.access_level >= CHAT_EXEMPT_ACCESS_LEVEL {
                    RateDecision::Allowed
                } else {
                    c.rate_limits.check(RateCategory::Chat, now)
                };
                if let RateDecision::Limited { notify } = decision {
                    let actor = RateActor {
                        addr,
                        player_id: c.active_player_id,
                        account_id: c.account_id,
                        entity_id: c.player_entity_id,
                    };
                    log_exceeded(RateCategory::Chat, actor, notify, &c.rate_limits, now);
                }
                (caller, decision)
            }
            None => (None, RateDecision::Allowed),
        }
    };
    if let RateDecision::Limited { notify } = decision {
        if notify {
            send_feedback_line(&feedback, addr, RateCategory::Chat.feedback_text()).await;
        }
        return;
    }
    let Some(caller) = caller else {
        let (ident, entity_id) = connected
            .lock()
            .unwrap()
            .get(&addr)
            .map_or((PlayerIdentity::UNKNOWN, None), |c| {
                (session_identity(c), c.player_entity_id)
            });
        tracing::debug!(
            target: "chat",
            event = "chat.ignore_refused",
            %addr,
            account_id = ident.account_id,
            account_name = ident.account_name,
            player_id = ident.player_id,
            player_name = ident.player_name,
            entity_id,
            entity_name = ident.player_name,
            reason = "not_in_world",
            "chatIgnore from a session with no character in the world; dropped",
        );
        return;
    };

    // `target_player_id` is the other character when it is known (resolved
    // on an add, looked up on a remove), per instrumentation rule 5.
    let refuse = |reason: &'static str, name: &str, target_player_id: Option<i32>| {
        tracing::debug!(
            target: "chat",
            event = "chat.ignore_refused",
            %addr,
            player_id = caller.player_id,
            player_name = caller.identity.player_name,
            account_id = caller.account_id,
            account_name = caller.identity.account_name,
            entity_id = caller.entity_id,
            entity_name = caller.identity.player_name,
            target_player_id,
            target_player_name = target_player_id.map(|_| name),
            target_name = %shown(name),
            reason,
            "chatIgnore refused, feedback sent",
        );
    };

    let Some((typed, flag)) = decode(payload) else {
        refuse("decode_failed", "", None);
        send_feedback_line(&feedback, addr, IGNORE_BAD_REQUEST_TEXT).await;
        return;
    };
    let add = match flag {
        1 => true,
        0 => false,
        _ => {
            refuse("bad_flag", &typed, None);
            send_feedback_line(&feedback, addr, IGNORE_BAD_REQUEST_TEXT).await;
            return;
        }
    };
    if typed.is_empty() {
        refuse("no_target", &typed, None);
        send_feedback_line(&feedback, addr, IGNORE_NO_TARGET_TEXT).await;
        return;
    }
    // A character name: at most 64 UTF-16 units (`sgw_player.player_name` is
    // varchar(64)) and no control, bidi or format characters, the same
    // bound a gate-mail recipient name gets (D-SS12). `read_wstring` has
    // already checked the declared length against the packet before
    // allocating, so this is the semantic bound, before any database work.
    if let Err(reject) = validate(TextField::MailRecipient, &typed) {
        refuse(reject.reason(), &typed, None);
        send_feedback_line(&feedback, addr, IGNORE_BAD_NAME_TEXT).await;
        return;
    }
    let Some(pool) = db_pool.as_deref() else {
        refuse("no_db_pool", &typed, None);
        send_feedback_line(&feedback, addr, IGNORE_UNAVAILABLE_TEXT).await;
        return;
    };

    let loaded = match ensure_ignore_list(pool, caller.player_id).await {
        Ok(list_id) => match load_ignore_names(pool, caller.player_id).await {
            Ok(names) => Ok((list_id, names)),
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    };
    let (list_id, current) = match loaded {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(
                target: "chat",
                event = "chat.ignore_refused",
                %addr,
                player_id = caller.player_id,
                player_name = caller.identity.player_name,
                account_id = caller.account_id,
                account_name = caller.identity.account_name,
                entity_id = caller.entity_id,
                entity_name = caller.identity.player_name,
                reason = "db_error",
                error = %e,
                "chatIgnore: could not load the Ignore list",
            );
            send_feedback_line(&feedback, addr, IGNORE_UNAVAILABLE_TEXT).await;
            return;
        }
    };

    let (name, target_player_id) = if add {
        let (target_player_id, name) = match resolve_character(pool, &typed).await {
            Ok(CharacterLookup::Found { player_id, name }) => (player_id, name),
            Ok(CharacterLookup::NotFound) => {
                refuse("unknown_character", &typed, None);
                let text = format!("No character named {} exists.", shown(&typed));
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
            Ok(CharacterLookup::Ambiguous) => {
                refuse("ambiguous", &typed, None);
                let text = format!(
                    "More than one character is named {}. Type the exact name.",
                    shown(&typed)
                );
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
            Err(e) => {
                tracing::error!(
                    target: "chat",
                    event = "chat.ignore_refused",
                    %addr,
                    player_id = caller.player_id,
                    player_name = caller.identity.player_name,
                    account_id = caller.account_id,
                    account_name = caller.identity.account_name,
                    entity_id = caller.entity_id,
                    entity_name = caller.identity.player_name,
                    reason = "db_error",
                    error = %e,
                    "chatIgnore: name lookup failed",
                );
                send_feedback_line(&feedback, addr, IGNORE_UNAVAILABLE_TEXT).await;
                return;
            }
        };
        if target_player_id == caller.player_id {
            refuse("self", &typed, Some(target_player_id));
            send_feedback_line(&feedback, addr, IGNORE_SELF_TEXT).await;
            return;
        }
        // The duplicate and cap checks are not made here, on the snapshot:
        // `add_ignore_entry` makes them in the database under the list's row
        // lock, so overlapping adds cannot both pass them (PR #893 review).
        (name, Some(target_player_id))
    } else {
        match match_name(current.iter().map(String::as_str), &typed) {
            NameMatch::Found(name) => {
                // For the log only: the character the entry names, if it
                // still exists. A lookup failure just leaves it unknown.
                let target_player_id = match resolve_character(pool, &name).await {
                    Ok(CharacterLookup::Found { player_id, .. }) => Some(player_id),
                    _ => None,
                };
                (name, target_player_id)
            }
            NameMatch::NotFound => {
                refuse("not_ignored", &typed, None);
                let text = format!("{} is not on your Ignore list.", shown(&typed));
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
            NameMatch::Ambiguous => {
                refuse("ambiguous", &typed, None);
                let text = format!(
                    "More than one name on your Ignore list matches {}. Type the exact name.",
                    shown(&typed)
                );
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
        }
    };

    // On an add, `add_ignore_entry` makes the duplicate and cap checks and
    // the insert in one locked transaction; `announce_added_members` then
    // echoes CM 87 and reloads the session and cell copies. A remove goes
    // through the member op, which echoes CM 88 and reloads them too.
    let (before, after) = if add {
        match add_ignore_entry(pool, caller.player_id, &name).await {
            Ok(IgnoreAdd::Added { list_id, before }) => {
                announce_added_members(
                    caller.entity_id,
                    caller.player_id,
                    list_id,
                    std::slice::from_ref(&name),
                    db_pool,
                    transport,
                    connected,
                    entity_to_addr,
                    cell_tx,
                )
                .await;
                (before, before + 1)
            }
            Ok(IgnoreAdd::Duplicate(stored)) => {
                refuse("already_ignored", &name, target_player_id);
                let text = format!("{stored} is already on your Ignore list.");
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
            Ok(IgnoreAdd::Full) => {
                refuse("list_full", &name, target_player_id);
                send_feedback_line(&feedback, addr, &ignore_full_text()).await;
                return;
            }
            Err(e) => {
                tracing::error!(
                    target: "chat",
                    event = "chat.ignore_refused",
                    %addr,
                    player_id = caller.player_id,
                    player_name = caller.identity.player_name,
                    account_id = caller.account_id,
                    account_name = caller.identity.account_name,
                    entity_id = caller.entity_id,
                    entity_name = caller.identity.player_name,
                    target_player_id,
                    target_player_name = target_player_id.map(|_| name.as_str()),
                    reason = "db_error",
                    error = %e,
                    "chatIgnore: the Ignore insert failed",
                );
                send_feedback_line(&feedback, addr, IGNORE_UNAVAILABLE_TEXT).await;
                return;
            }
        }
    } else {
        let removed = handle_remove_members(
            caller.entity_id,
            caller.player_id,
            list_id,
            vec![name.clone()],
            db_pool,
            transport,
            connected,
            entity_to_addr,
            cell_tx,
        )
        .await;
        if removed.is_empty() {
            // The member op logged the DB outcome; the player still hears back.
            refuse("write_failed", &name, target_player_id);
            send_feedback_line(&feedback, addr, IGNORE_UNAVAILABLE_TEXT).await;
            return;
        }
        (current.len(), current.len().saturating_sub(1))
    };

    tracing::info!(
        target: "chat",
        event = if add { "chat.ignore_added" } else { "chat.ignore_removed" },
        %addr,
        player_id = caller.player_id,
        player_name = caller.identity.player_name,
        account_id = caller.account_id,
        account_name = caller.identity.account_name,
        entity_id = caller.entity_id,
        entity_name = caller.identity.player_name,
        target_player_id,
        target_player_name = target_player_id.map(|_| name.as_str()),
        before,
        after,
        "Ignore list changed by chatIgnore",
    );
    let text = if add {
        format!("You are now ignoring {name}.")
    } else {
        format!("You are no longer ignoring {name}.")
    };
    send_feedback_line(&feedback, addr, &text).await;
}

/// `WSTRING aPlayerName, UINT8 aFlag`. `None` when either is missing.
fn decode(payload: &[u8]) -> Option<(String, u8)> {
    let (name, consumed) = read_wstring(payload, 0).ok()?;
    let flag = *payload.get(consumed)?;
    Some((name, flag))
}
