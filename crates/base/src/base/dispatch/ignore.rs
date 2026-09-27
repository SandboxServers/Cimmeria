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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::super::contact_list::handlers::{handle_add_members, handle_remove_members};
use super::super::contact_list::ignore::{
    ensure_ignore_list, load_ignore_names, match_name, resolve_character, CharacterLookup,
    NameMatch, MAX_IGNORE_LIST_MEMBERS,
};
use super::super::feedback::{send_feedback_line, FeedbackCtx};
use super::super::ConnectedClientState;

/// Longest prefix of a typed name echoed back or logged.
const SHOWN_NAME_CHARS: usize = 64;

pub(super) const IGNORE_NO_TARGET_TEXT: &str = "Name a player to ignore.";
pub(super) const IGNORE_SELF_TEXT: &str = "You cannot ignore yourself.";
pub(super) const IGNORE_UNAVAILABLE_TEXT: &str =
    "Your Ignore list cannot be changed right now. Try again later.";
pub(super) const IGNORE_BAD_REQUEST_TEXT: &str = "That Ignore request could not be read.";

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
}

/// Handle `chatIgnore`. Every path ends in one feedback line.
#[tracing::instrument(name = "chat.ignore", level = "info", skip_all, fields(peer = %addr))]
pub(super) async fn handle_chat_ignore(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    let feedback = FeedbackCtx {
        transport,
        connected,
    };

    let caller = {
        let clients = connected.lock().unwrap();
        clients.get(&addr).and_then(|c| {
            Some(Caller {
                player_id: c.active_player_id?,
                account_id: c.account_id,
                entity_id: c.player_entity_id?,
            })
        })
    };
    let Some(caller) = caller else {
        tracing::debug!(
            target: "chat",
            event = "chat.ignore_refused",
            %addr,
            reason = "not_in_world",
            "chatIgnore from a session with no character in the world; dropped",
        );
        return;
    };

    let refuse = |reason: &'static str, name: &str| {
        tracing::debug!(
            target: "chat",
            event = "chat.ignore_refused",
            %addr,
            player_id = caller.player_id,
            account_id = caller.account_id,
            entity_id = caller.entity_id,
            target_name = %shown(name),
            reason,
            "chatIgnore refused, feedback sent",
        );
    };

    let Some((typed, flag)) = decode(payload) else {
        refuse("decode_failed", "");
        send_feedback_line(&feedback, addr, IGNORE_BAD_REQUEST_TEXT).await;
        return;
    };
    let add = match flag {
        1 => true,
        0 => false,
        _ => {
            refuse("bad_flag", &typed);
            send_feedback_line(&feedback, addr, IGNORE_BAD_REQUEST_TEXT).await;
            return;
        }
    };
    if typed.is_empty() {
        refuse("no_target", &typed);
        send_feedback_line(&feedback, addr, IGNORE_NO_TARGET_TEXT).await;
        return;
    }
    let Some(pool) = db_pool.as_deref() else {
        refuse("no_db_pool", &typed);
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
                account_id = caller.account_id,
                entity_id = caller.entity_id,
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
                refuse("unknown_character", &typed);
                let text = format!("No character named {} exists.", shown(&typed));
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
            Ok(CharacterLookup::Ambiguous) => {
                refuse("ambiguous", &typed);
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
                    account_id = caller.account_id,
                    entity_id = caller.entity_id,
                    reason = "db_error",
                    error = %e,
                    "chatIgnore: name lookup failed",
                );
                send_feedback_line(&feedback, addr, IGNORE_UNAVAILABLE_TEXT).await;
                return;
            }
        };
        if target_player_id == caller.player_id {
            refuse("self", &typed);
            send_feedback_line(&feedback, addr, IGNORE_SELF_TEXT).await;
            return;
        }
        if current.contains(&name) {
            refuse("already_ignored", &name);
            let text = format!("{name} is already on your Ignore list.");
            send_feedback_line(&feedback, addr, &text).await;
            return;
        }
        if current.len() >= MAX_IGNORE_LIST_MEMBERS {
            refuse("list_full", &name);
            send_feedback_line(&feedback, addr, &ignore_full_text()).await;
            return;
        }
        (name, Some(target_player_id))
    } else {
        match match_name(current.iter().map(String::as_str), &typed) {
            NameMatch::Found(name) => (name, None),
            NameMatch::NotFound => {
                refuse("not_ignored", &typed);
                let text = format!("{} is not on your Ignore list.", shown(&typed));
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
            NameMatch::Ambiguous => {
                refuse("ambiguous", &typed);
                let text = format!(
                    "More than one name on your Ignore list matches {}. Type the exact name.",
                    shown(&typed)
                );
                send_feedback_line(&feedback, addr, &text).await;
                return;
            }
        }
    };

    // The member ops echo CM 87/88 and, because this is the Ignore list,
    // reload the session and cell copies before returning.
    let changed = if add {
        handle_add_members(
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
        .await
    } else {
        handle_remove_members(
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
        .await
    };
    if changed.is_empty() {
        // The member op logged the DB outcome; the player still hears back.
        refuse("write_failed", &name);
        send_feedback_line(&feedback, addr, IGNORE_UNAVAILABLE_TEXT).await;
        return;
    }

    let before = current.len();
    let after = if add { before + 1 } else { before - 1 };
    tracing::info!(
        target: "chat",
        event = if add { "chat.ignore_added" } else { "chat.ignore_removed" },
        %addr,
        player_id = caller.player_id,
        account_id = caller.account_id,
        entity_id = caller.entity_id,
        target_player_id,
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
