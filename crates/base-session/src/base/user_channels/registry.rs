//! [`UserChannelRegistry`]: the process-wide table of user chat channels,
//! keyed by their wire id (`CHAN_CHAT` and up).
//!
//! One channel per unique [`name_key`](cimmeria_entity::organization::org_text::name_key)
//! (case-folded, D-SS13 style): `chat` and `Chat` are the same channel, so a
//! player who mistypes case still lands in the same room as everyone else.
//! Membership is by entity id (the base's `SGWPlayer` entity for the
//! character, matching the legacy `ChatChannel.players[player.entityId]`),
//! never by address or account -- a character with two sessions cannot
//! happen, and this stays correct if that ever changes.
//!
//! Channels are process memory only, like [`super::super::mutes`]: nothing
//! here is persisted, and a server restart starts with no channels (every
//! client rejoins its saved ones and the three defaults on login, exactly
//! as it does after the very first login ever).

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

use cimmeria_entity::organization::limits::{MAX_CHANNELS_PER_PLAYER, MAX_USER_CHANNELS};
use cimmeria_wire::cell::chat::CHAN_CHAT;

/// One user channel: its wire id, its display name as first created, and
/// who is in it now.
#[derive(Debug, Clone)]
struct ChannelState {
    display_name: String,
    members: HashSet<u32>,
}

#[derive(Debug, Default)]
struct Inner {
    /// Wire id -> channel. The id is never below [`CHAN_CHAT`].
    channels: HashMap<u8, ChannelState>,
    /// Case-folded name -> wire id, the join-by-name index.
    by_key: HashMap<String, u8>,
}

/// The process-wide user-channel table.
#[derive(Debug, Default)]
pub struct UserChannelRegistry {
    inner: Mutex<Inner>,
}

/// Outcome of [`UserChannelRegistry::join`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinOutcome {
    /// Joined (and, if `created`, the channel did not exist before this
    /// call). `wire_id` is the full wire channel id; the caller derives the
    /// display id for `onChatJoined` as `wire_id - CHAN_CHAT`.
    Joined {
        wire_id: u8,
        display_name: String,
        created: bool,
    },
    /// The caller's entity was already a member of this channel.
    AlreadyMember { display_name: String },
    /// The entity already holds [`MAX_CHANNELS_PER_PLAYER`] channels.
    PlayerLimitReached,
    /// The server already holds [`MAX_USER_CHANNELS`] channels, or every
    /// wire id up to `u8::MAX` is taken (the wire ceiling: `onChatJoined`'s
    /// display id is one `UINT8`).
    ServerLimitReached,
}

/// Outcome of [`UserChannelRegistry::leave`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaveOutcome {
    /// Left. `deleted` is true when that was the last member and the
    /// channel (and its wire id) no longer exist.
    Left { display_name: String, deleted: bool },
    /// No channel has this wire id (never created, already deleted, or a
    /// display id past what the caller has ever seen joined).
    NotFound,
    /// The channel exists, but the caller's entity was not a member.
    NotMember,
}

impl UserChannelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A poisoned lock only means a panic elsewhere while holding it;
        // every write here is one insert/remove pair kept in sync (see
        // `join`/`leave`), so keep serving reads rather than failing open
        // to "no channels exist" (which would silently drop every member).
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Join `key` (the case-folded [`name_key`] of `display_name`),
    /// creating the channel if no channel has that key yet.
    ///
    /// `display_name` is the normalised text to show and to send back in
    /// `onChatJoined` -- for a *new* channel it becomes that channel's
    /// name; for an *existing* one the original creator's spelling wins
    /// (matching the join-by-key semantics: the second "CHAT" joins the
    /// first "Chat", it does not rename it).
    ///
    /// [`name_key`]: cimmeria_entity::organization::org_text::name_key
    pub fn join(&self, display_name: &str, key: &str, entity_id: u32) -> JoinOutcome {
        let mut inner = self.lock();
        if let Some(&wire_id) = inner.by_key.get(key) {
            let existing_name = inner.channels[&wire_id].display_name.clone();
            if inner.channels[&wire_id].members.contains(&entity_id) {
                return JoinOutcome::AlreadyMember {
                    display_name: existing_name,
                };
            }
            if member_count(&inner.channels, entity_id) >= MAX_CHANNELS_PER_PLAYER {
                return JoinOutcome::PlayerLimitReached;
            }
            inner
                .channels
                .get_mut(&wire_id)
                .expect("by_key and channels stay in sync")
                .members
                .insert(entity_id);
            return JoinOutcome::Joined {
                wire_id,
                display_name: existing_name,
                created: false,
            };
        }

        if member_count(&inner.channels, entity_id) >= MAX_CHANNELS_PER_PLAYER {
            return JoinOutcome::PlayerLimitReached;
        }
        if inner.channels.len() >= MAX_USER_CHANNELS {
            return JoinOutcome::ServerLimitReached;
        }
        let Some(wire_id) = allocate_id(&inner.channels) else {
            return JoinOutcome::ServerLimitReached;
        };
        inner.channels.insert(
            wire_id,
            ChannelState {
                display_name: display_name.to_string(),
                members: HashSet::from([entity_id]),
            },
        );
        inner.by_key.insert(key.to_string(), wire_id);
        JoinOutcome::Joined {
            wire_id,
            display_name: display_name.to_string(),
            created: true,
        }
    }

    /// Leave the channel at `wire_id`. Deletes the channel (freeing its id
    /// for reuse) when that was the last member -- every user channel is
    /// created on demand by a join, so none needs to survive empty (unlike
    /// the legacy `CHANNEL_FLAG_KeepIfEmpty` defaults, which existed only
    /// because the legacy server pre-created them at boot, before anyone
    /// had joined).
    pub fn leave(&self, wire_id: u8, entity_id: u32) -> LeaveOutcome {
        let mut inner = self.lock();
        let Some(channel) = inner.channels.get_mut(&wire_id) else {
            return LeaveOutcome::NotFound;
        };
        if !channel.members.remove(&entity_id) {
            return LeaveOutcome::NotMember;
        }
        let display_name = channel.display_name.clone();
        let deleted = channel.members.is_empty();
        if deleted {
            inner.channels.remove(&wire_id);
            inner.by_key.retain(|_, id| *id != wire_id);
        }
        LeaveOutcome::Left {
            display_name,
            deleted,
        }
    }

    /// Remove `entity_id` from every channel it belongs to (a session
    /// teardown, never a client-initiated leave): no `onChatLeft` is owed,
    /// since there is no client left to tell. Deletes any channel this
    /// empties.
    pub fn leave_all(&self, entity_id: u32) {
        let mut inner = self.lock();
        let mut emptied = Vec::new();
        for (&wire_id, channel) in inner.channels.iter_mut() {
            if channel.members.remove(&entity_id) && channel.members.is_empty() {
                emptied.push(wire_id);
            }
        }
        for wire_id in emptied {
            inner.channels.remove(&wire_id);
            inner.by_key.retain(|_, id| *id != wire_id);
        }
    }

    /// The members of `wire_id`, if `entity_id` is one of them -- the
    /// server-authority check for `sendPlayerCommunication` on a user
    /// channel: a client cannot post to a channel it never joined, no
    /// matter what byte it sends. `None` for a channel that does not exist
    /// or that `entity_id` has not joined; callers must not distinguish
    /// the two in the refusal text (doing so would let a client probe
    /// which channel names exist).
    pub fn members_if_joined(&self, wire_id: u8, entity_id: u32) -> Option<Vec<u32>> {
        let inner = self.lock();
        let channel = inner.channels.get(&wire_id)?;
        if !channel.members.contains(&entity_id) {
            return None;
        }
        Some(channel.members.iter().copied().collect())
    }

    /// How many channels exist right now (tests and diagnostics).
    pub fn channel_count(&self) -> usize {
        self.lock().channels.len()
    }

    /// Whether a channel with this case-folded [`name_key`] exists right
    /// now (tests and diagnostics) -- unlike [`Self::channel_count`], this
    /// is stable under other tests concurrently joining and leaving
    /// unrelated channels on the shared process-wide registry.
    ///
    /// [`name_key`]: cimmeria_entity::organization::org_text::name_key
    pub fn contains_name(&self, key: &str) -> bool {
        self.lock().by_key.contains_key(key)
    }
}

fn member_count(channels: &HashMap<u8, ChannelState>, entity_id: u32) -> usize {
    channels
        .values()
        .filter(|c| c.members.contains(&entity_id))
        .count()
}

/// The lowest unused wire id from [`CHAN_CHAT`], or `None` if every id up
/// to `u8::MAX` is taken (the wire ceiling the doc comment on
/// [`JoinOutcome::ServerLimitReached`] explains). [`MAX_USER_CHANNELS`] is
/// checked before this runs, so this only ever scans a small, mostly-empty
/// range in practice.
fn allocate_id(channels: &HashMap<u8, ChannelState>) -> Option<u8> {
    (CHAN_CHAT..=u8::MAX).find(|id| !channels.contains_key(id))
}

static REGISTRY: LazyLock<UserChannelRegistry> = LazyLock::new(UserChannelRegistry::new);

/// The server's user-channel table: one per base process, shared by the
/// `chatJoin` / `chatLeave` / `sendPlayerCommunication` handlers and the
/// session-teardown cleanup.
pub fn user_channel_registry() -> &'static UserChannelRegistry {
    &REGISTRY
}

#[cfg(test)]
mod tests;
