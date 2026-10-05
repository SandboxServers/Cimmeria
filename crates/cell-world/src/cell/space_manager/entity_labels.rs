//! Names for runtime entity IDs (instrumentation-discipline Rule 6, NT-02).
//!
//! An entity ID is a recycled slot, so it is never named from the static
//! NameBook. A live entity is named from itself: the character name for a
//! player, the player-facing `name_id` text for an NPC. A departed one is
//! named from the [`departed ring`](super::departed_ring), checked against
//! its lifetime so a reused slot is never named after the wrong occupant.

use std::time::SystemTime;

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::name_intern::intern_opt;

use crate::cell::messages::{
    EntityLabelQuery, EntityLabelsReply, EntityLabelsRequest, ENTITY_LABEL_QUERY_CAP,
};

use super::departed_ring::DepartedEntity;
use super::SpaceManager;

/// The names a log line about one entity carries next to its ID.
///
/// For an NPC that is all three fields (D-NT5): `entity_name` is the
/// player-facing name, `template_id` + `template_name` the designer's. For a
/// player only `entity_name` (the character name) is set. Emit each as an
/// `Option` field so an unknown one is left out:
///
/// ```ignore
/// let n = space_mgr.entity_names(entity_id);
/// tracing::warn!(
///     entity_id,
///     entity_name = n.entity_name,
///     template_id = n.template_id,
///     template_name = n.template_name,
///     "..."
/// );
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityNames {
    pub entity_name: Option<&'static str>,
    pub template_id: Option<i32>,
    pub template_name: Option<&'static str>,
}

impl EntityNames {
    /// The names of `e`, resolved now. For an NPC this costs a NameBook read
    /// and an interner lookup, so call it inside the branch that logs.
    pub fn of(e: &CellEntity) -> Self {
        if is_player(e) {
            return Self {
                entity_name: player_label(e),
                ..Self::default()
            };
        }
        let book = cimmeria_names::book();
        Self {
            entity_name: intern_opt(npc_display_name(&book, e)),
            template_id: e.template_id,
            template_name: intern_opt(e.template_id.and_then(|t| book.template(t))),
        }
    }
}

/// An NPC's player-facing name: its own `name_id` text, else its template's.
fn npc_display_name<'b>(book: &'b cimmeria_names::NameBook, e: &CellEntity) -> Option<&'b str> {
    e.name_id
        .and_then(|n| book.text(n))
        .or_else(|| e.template_id.and_then(|t| book.template_display(t)))
}

/// A player entity, as opposed to an NPC: it belongs to a session.
fn is_player(e: &CellEntity) -> bool {
    e.is_player
        || e.player_id.is_some()
        || e.log_names.player_name.is_some()
        || e.character_name.is_some()
}

/// A player's name: the one stamped for logs at birth (already interned),
/// else the game's `character_name`.
fn player_label(e: &CellEntity) -> Option<&'static str> {
    e.log_names
        .player_name
        .or_else(|| intern_opt(e.character_name.as_deref()))
}

/// The label of a live entity: see [`SpaceManager::entity_label`].
fn label_of(e: &CellEntity) -> Option<&'static str> {
    EntityNames::of(e).entity_name
}

impl SpaceManager {
    /// The name of a live entity: the character name for a player, the
    /// `name_id` text for an NPC. `None` when the entity is gone or has no
    /// name; never `""`. NPC log lines also want `template_id` +
    /// `template_name`: use [`Self::entity_names`] for those.
    ///
    /// Takes an `Option<u32>` too, for the log fields whose entity is
    /// optional (`pet_id`, `owner_id`): `None` names nothing.
    pub fn entity_label(&self, entity_id: impl Into<Option<u32>>) -> Option<&str> {
        entity_id
            .into()
            .and_then(|id| self.get_entity(id))
            .and_then(label_of)
    }

    /// [`EntityNames`] for a live entity; all `None` when it is gone. A
    /// function that destroys the entity snapshots this at its top.
    pub fn entity_names(&self, entity_id: u32) -> EntityNames {
        self.get_entity(entity_id)
            .map(EntityNames::of)
            .unwrap_or_default()
    }

    /// The label of whichever entity held `entity_id` in `space_id` at
    /// server time `at`.
    ///
    /// Lifetimes are half-open, `[created_at, destroyed_at)`. The live
    /// entity answers when `at` is at or after its creation; a departed one
    /// answers when `at` falls inside its lifetime and it is still in its
    /// ring (10 minutes or 4,096 rows per space, players and NPCs in
    /// separate rings). `None` otherwise: a slot is never named after an
    /// occupant who didn't hold it at `at`.
    ///
    /// **`at` must be a server-clock time.** Lifetimes are stamped with the
    /// server's wall clock, and occupants of one slot can be seconds apart,
    /// so a client clock's skew would name the wrong one, and a client
    /// could pick its timestamp to choose the name. A caller with a client
    /// row (NT-40) maps the row onto the server clock first, from the
    /// server's receive time or a per-session clock offset, and never
    /// passes the client's time raw.
    pub fn entity_label_at(
        &self,
        space_id: u32,
        entity_id: u32,
        at: SystemTime,
    ) -> Option<&'static str> {
        if let Some(e) = self
            .spaces
            .get(&space_id)
            .and_then(|s| s.entities.get(&entity_id))
        {
            if e.created_at <= at {
                return label_of(e);
            }
        }
        self.departed
            .alive_at(space_id, entity_id, at)
            .and_then(|d| d.label)
    }

    /// Answer an [`EntityLabelsRequest`] from the telemetry ingest (NT-40),
    /// unless the ingest already stopped waiting for it: then the answer
    /// would go nowhere, so none is computed. Returns whether it answered.
    pub fn answer_entity_labels(&self, req: EntityLabelsRequest) -> bool {
        if req.reply_tx.is_closed() {
            return false;
        }
        req.reply_tx
            .send(self.entity_labels_at(&req.queries))
            .is_ok()
    }

    /// One [`Self::entity_label_at`] per query, in order. Queries past
    /// [`ENTITY_LABEL_QUERY_CAP`] get `None`, so one telemetry chunk can't
    /// stall the tick.
    pub fn entity_labels_at(&self, queries: &[EntityLabelQuery]) -> EntityLabelsReply {
        queries
            .iter()
            .enumerate()
            .map(|(i, q)| {
                (i < ENTITY_LABEL_QUERY_CAP)
                    .then(|| self.entity_label_at(q.space_id, q.entity_id, q.at))
                    .flatten()
            })
            .collect()
    }

    /// What the ring needs about a live entity, taken before it is
    /// removed. `None` when the entity isn't in a space.
    pub(crate) fn departure_snapshot(&self, entity_id: u32) -> Option<PendingDeparture> {
        let space_id = *self.entity_space.get(&entity_id)?;
        let e = self.spaces.get(&space_id)?.entities.get(&entity_id)?;
        Some(PendingDeparture::of(space_id, entity_id, e))
    }

    /// Record a departure in the ring, stamped with the ring's clock.
    pub(crate) fn record_departure(&mut self, pending: PendingDeparture) {
        let destroyed_at = self.departed.now();
        self.departed.push(
            pending.space_id,
            DepartedEntity {
                entity_id: pending.entity_id,
                player: pending.player,
                label: pending.names.entity_name,
                template_id: pending.names.template_id,
                created_at: pending.created_at,
                destroyed_at,
            },
        );
    }
}

/// A departure snapshotted before teardown and recorded after it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PendingDeparture {
    space_id: u32,
    entity_id: u32,
    player: bool,
    names: EntityNames,
    created_at: SystemTime,
}

impl PendingDeparture {
    pub(crate) fn of(space_id: u32, entity_id: u32, e: &CellEntity) -> Self {
        Self {
            space_id,
            entity_id,
            player: is_player(e),
            names: EntityNames::of(e),
            created_at: e.created_at,
        }
    }

    /// The names snapshotted, for the teardown log line.
    pub(crate) fn names(&self) -> EntityNames {
        self.names
    }
}
