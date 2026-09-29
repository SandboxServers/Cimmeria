//! Muted accounts: events a lab or test account causes never reach Discord.
//!
//! `[discord] muted_accounts = ["lab"]` names accounts by login name (any
//! case) or by numeric account id. An event is muted when it names a muted
//! account directly (`account_id` / `account_name`, or an `account_id` field
//! on a harvested tracing event), or when it names a character that a muted
//! account was seen playing: many events (level-ups, deaths, missions,
//! chat) carry only the character name, so the characters are learned from
//! the events that carry both, such as login and world entry.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::Event;

/// Who an event is about, as far as its payload says.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Identity<'a> {
    pub account_id: Option<u32>,
    pub account_name: Option<&'a str>,
    pub character: Option<&'a str>,
}

/// The account and character an event names, if any.
pub fn identity(event: &Event) -> Identity<'_> {
    fn id<'a>(
        account_id: Option<u32>,
        account_name: Option<&'a String>,
        character: Option<&'a String>,
    ) -> Identity<'a> {
        Identity {
            account_id,
            account_name: account_name.map(String::as_str),
            character: character.map(String::as_str),
        }
    }
    match event {
        Event::PlayerLogin {
            account_id,
            account_name,
            character_name,
            ..
        }
        | Event::PlayerLogout {
            account_id,
            account_name,
            character_name,
            ..
        } => id(
            Some(*account_id),
            account_name.as_ref(),
            character_name.as_ref(),
        ),
        Event::PlayerDisconnect {
            account_id,
            account_name,
            character_name,
            ..
        } => id(*account_id, account_name.as_ref(), character_name.as_ref()),
        Event::PlayerAuthFailed { account_name, .. } => id(None, Some(account_name), None),
        Event::PlayerWorldEntry {
            account_id,
            account_name,
            character_name,
            ..
        }
        | Event::PlayerWorldExit {
            account_id,
            account_name,
            character_name,
            ..
        }
        | Event::CharacterCreated {
            account_id,
            account_name,
            character_name,
            ..
        } => id(
            Some(*account_id),
            account_name.as_ref(),
            Some(character_name),
        ),
        Event::PlayerLevelUp { character_name, .. }
        | Event::PlayerDeath { character_name, .. }
        | Event::PlayerRespawn { character_name, .. }
        | Event::MissionAccepted { character_name, .. }
        | Event::MissionCompleted { character_name, .. }
        | Event::MissionFailed { character_name, .. }
        | Event::MissionRewardGranted { character_name, .. }
        | Event::LootGenerated { character_name, .. }
        | Event::ItemUsed { character_name, .. }
        | Event::MinigameResult { character_name, .. }
        | Event::Dialog { character_name, .. } => id(None, None, Some(character_name)),
        Event::Chat { speaker, .. } => id(None, None, Some(speaker)),
        Event::GmCommand { gm_name, .. }
        | Event::GmTeleport { gm_name, .. }
        | Event::GmSpawn { gm_name, .. }
        | Event::GmItemGrant { gm_name, .. } => id(None, None, Some(gm_name)),
        Event::NpcDeath { killer, .. } => id(None, None, killer.as_ref()),
        Event::MercuryTimeout { account_id, .. } => id(*account_id, None, None),
        Event::TracingEvent { fields, .. } => Identity {
            account_id: fields
                .iter()
                .find(|(k, _)| k == "account_id")
                .and_then(|(_, v)| v.parse().ok()),
            ..Identity::default()
        },
        _ => Identity::default(),
    }
}

fn learned() -> &'static Mutex<HashSet<String>> {
    static L: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(HashSet::new()))
}

fn account_matches(muted: &[String], who: &Identity<'_>) -> bool {
    muted.iter().any(|m| match m.parse::<u32>() {
        Ok(id) => who.account_id == Some(id),
        Err(_) => who.account_name.is_some_and(|n| n.eq_ignore_ascii_case(m)),
    })
}

/// Whether `event` belongs to a muted account. Learns the character of a
/// muted account from events that name both.
pub fn is_muted(muted: &[String], event: &Event) -> bool {
    if muted.is_empty() {
        return false;
    }
    let who = identity(event);
    if account_matches(muted, &who) {
        if let (Some(c), Ok(mut l)) = (who.character, learned().lock()) {
            l.insert(c.to_ascii_lowercase());
        }
        return true;
    }
    who.character.is_some_and(|c| {
        learned()
            .lock()
            .is_ok_and(|l| l.contains(&c.to_ascii_lowercase()))
    })
}

#[cfg(test)]
mod tests;
