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

use crate::{Event, Named};

/// Who an event is about, as far as its payload says.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Identity<'a> {
    pub account_id: Option<u32>,
    pub account_name: Option<&'a str>,
    pub character: Option<&'a str>,
}

/// The account and character an event names, if any.
pub fn identity(event: &Event) -> Identity<'_> {
    // An account ID outside `u32` cannot be a real account; it reads as
    // no ID rather than wrapping onto someone else's.
    fn account_id(n: &Named) -> Option<u32> {
        n.id.and_then(|i| u32::try_from(i).ok())
    }
    fn id<'a>(account: Option<&'a Named>, character: Option<&'a Named>) -> Identity<'a> {
        Identity {
            account_id: account.and_then(account_id),
            account_name: account.and_then(Named::name),
            character: character.and_then(Named::name),
        }
    }
    match event {
        Event::PlayerLogin {
            account, character, ..
        }
        | Event::PlayerLogout {
            account, character, ..
        }
        | Event::PlayerDisconnect {
            account, character, ..
        }
        | Event::MercuryTimeout {
            account, character, ..
        } => id(Some(account), character.as_ref()),
        Event::PlayerAuthFailed { account_name, .. } => Identity {
            account_name: Some(account_name),
            ..Identity::default()
        },
        Event::PlayerWorldEntry {
            account, character, ..
        }
        | Event::PlayerWorldExit {
            account, character, ..
        }
        | Event::CharacterCreated {
            account, character, ..
        } => id(Some(account), Some(character)),
        Event::PlayerLevelUp { character, .. }
        | Event::PlayerDeath { character, .. }
        | Event::PlayerRespawn { character, .. }
        | Event::MissionAccepted { character, .. }
        | Event::MissionCompleted { character, .. }
        | Event::MissionFailed { character, .. }
        | Event::MissionRewardGranted { character, .. }
        | Event::LootGenerated { character, .. }
        | Event::ItemUsed { character, .. }
        | Event::MinigameResult { character, .. }
        | Event::Dialog { character, .. } => id(None, Some(character)),
        Event::Chat { speaker, .. } => id(None, Some(speaker)),
        Event::GmCommand { gm, .. }
        | Event::GmTeleport { gm, .. }
        | Event::GmSpawn { gm, .. }
        | Event::GmItemGrant { gm, .. } => id(None, Some(gm)),
        Event::NpcDeath { killer, .. } => id(None, killer.as_ref()),
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
