//! Naming entity IDs from late client telemetry rows (named telemetry, NT-40).
//!
//! The telemetry ingest (`cimmeria-admin-api`, `routes/telemetry`) replays
//! rows the client recorded seconds or minutes earlier. An entity ID in such
//! a row is a recycled slot, so it can only be named by asking the cell who
//! held that slot at the row's time: `SpaceManager::entity_label_at`, which
//! reads the live entity and the departed-entity rings (NT-02).
//!
//! **A channel of its own.** The ingest is reachable from the internet (the
//! upload routes are on the public login port), so its questions never ride
//! the base->cell gameplay channel: an [`EntityLabelsRequest`] goes on a
//! small dedicated channel ([`ENTITY_LABEL_CHANNEL_CAPACITY`]) that the cell
//! loop reads only when no gameplay message is waiting. A full channel makes
//! the ingest replay unnamed; it never waits.

use std::time::SystemTime;

/// Capacity of the ingest-to-cell label channel. The ingest also keeps at
/// most one request in flight, so this only absorbs requests whose sender
/// already gave up.
pub const ENTITY_LABEL_CHANNEL_CAPACITY: usize = 4;

/// Most queries one [`EntityLabelsRequest`] answers. Each is a hash lookup
/// on the cell loop thread; queries past the cap get `None`.
pub const ENTITY_LABEL_QUERY_CAP: usize = 128;

/// One entity ID to name: whoever held `entity_id` in `space_id` at server
/// time `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityLabelQuery {
    pub space_id: u32,
    pub entity_id: u32,
    /// Server-clock time, mapped from the client's timestamp by the caller.
    /// The space, the entity and the time all come from the client's row,
    /// so the answer names whoever the row claims, not a server observation.
    pub at: SystemTime,
}

/// The reply: one label per query, in order. `None` when no entity held the
/// slot at that time (or none the rings still remember), when it had no
/// name, or when the query was past [`ENTITY_LABEL_QUERY_CAP`].
pub type EntityLabelsReply = Vec<Option<&'static str>>;

/// A batch of [`EntityLabelQuery`]s and where to send the answer. The cell
/// skips a request whose `reply_tx` is already closed: the ingest stopped
/// waiting and replayed the chunk unnamed.
#[derive(Debug)]
pub struct EntityLabelsRequest {
    pub queries: Vec<EntityLabelQuery>,
    pub reply_tx: tokio::sync::oneshot::Sender<EntityLabelsReply>,
}
