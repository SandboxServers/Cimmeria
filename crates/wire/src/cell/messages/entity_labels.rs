//! Naming entity IDs from late client telemetry rows (named telemetry, NT-40).
//!
//! The telemetry ingest (`cimmeria-admin-api`, `routes/telemetry`) replays
//! rows the client recorded seconds or minutes earlier. An entity ID in such
//! a row is a recycled slot, so it can only be named by asking the cell who
//! held that slot at the row's time: `SpaceManager::entity_label_at`, which
//! reads the live entity and the departed-entity rings (NT-02). The cell
//! loop owns the `SpaceManager`, so the ingest sends one
//! [`BaseToCellMsg::EntityLabelsAt`](super::BaseToCellMsg::EntityLabelsAt)
//! per uploaded chunk with every ID it needs, and the cell answers between
//! ticks.

use std::time::SystemTime;

/// Most queries one [`BaseToCellMsg::EntityLabelsAt`](super::BaseToCellMsg::EntityLabelsAt)
/// answers. Each is a hash lookup plus a scan of one space's departed rings,
/// on the cell loop thread, so the batch is capped; queries past the cap get
/// `None`. A chunk names far fewer distinct entities than this.
pub const ENTITY_LABEL_QUERY_CAP: usize = 512;

/// One entity ID to name: whoever held `entity_id` in `space_id` at server
/// time `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityLabelQuery {
    pub space_id: u32,
    pub entity_id: u32,
    /// Server-clock time. The caller maps a client timestamp onto the server
    /// clock before asking; a raw client time would let clock skew, or the
    /// client, choose the occupant.
    pub at: SystemTime,
}

/// The reply: one label per query, in order. `None` when no entity held the
/// slot at that time (or none the rings still remember), when it had no
/// name, or when the query was past [`ENTITY_LABEL_QUERY_CAP`].
pub type EntityLabelsReply = Vec<Option<&'static str>>;
