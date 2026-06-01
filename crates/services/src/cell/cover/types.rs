//! Cover-system types: nodes, set metadata, height/quality enums, the
//! `Cover` service handle that aggregates a loaded spatial index + the
//! reservation table.

use cimmeria_common::{EntityId, Vector3};
use std::sync::Arc;
use std::sync::Mutex;

use super::reservation::CoverReservations;
use super::spatial::CoverIndex;

/// Cover height enum mirroring the `resources."ECoverHeight"` SQL type.
///
/// Binary-confirmed heights via direct memory read of
/// `DAT_018f41c8/cc/d0/d4` in SGW.exe (see
/// `docs/reverse-engineering/findings/cover-system.md`):
/// - `Low`  → 0.71 m (crouch-defeatable)
/// - `Mid`  → 1.07 m (default cover)
/// - `High` → 1.52 m (tall cover, full standing block)
/// - `Los`  → 2.52 m (fully line-of-sight blocking)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CoverHeight {
    Low = 0,
    Mid = 1,
    High = 2,
    Los = 3,
}

impl CoverHeight {
    /// Approximate cover height in BigWorld meters.
    pub fn meters(self) -> f32 {
        match self {
            Self::Low => 0.71,
            Self::Mid => 1.07,
            Self::High => 1.52,
            Self::Los => 2.52,
        }
    }

    /// Parse from the SQL enum string. Returns `None` for unrecognised values.
    pub fn from_sql_name(s: &str) -> Option<Self> {
        match s {
            "HEIGHT_Low" => Some(Self::Low),
            "HEIGHT_Mid" => Some(Self::Mid),
            "HEIGHT_High" => Some(Self::High),
            "HEIGHT_LOS" => Some(Self::Los),
            _ => None,
        }
    }
}

/// Cover quality enum mirroring `resources."ECoverQuality"`.
///
/// `None` is a real enum value in the source data — it tags nodes the
/// level designer marked as "placeholder / known-bad / do not select".
/// The scorer should bias against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CoverQuality {
    Good = 0,
    Better = 1,
    Best = 2,
    None_ = 3,
}

impl CoverQuality {
    /// Scoring weight contribution for the `aCoverWeight` term.
    /// Returns 1.0 for Best, 0.66 for Better, 0.33 for Good, 0.0 for None.
    pub fn score_factor(self) -> f32 {
        match self {
            Self::Best => 1.0,
            Self::Better => 0.66,
            Self::Good => 0.33,
            Self::None_ => 0.0,
        }
    }

    /// Parse from the SQL enum string.
    pub fn from_sql_name(s: &str) -> Option<Self> {
        match s {
            "QUALITY_Good" => Some(Self::Good),
            "QUALITY_Better" => Some(Self::Better),
            "QUALITY_Best" => Some(Self::Best),
            "QUALITY_None" => Some(Self::None_),
            _ => None,
        }
    }
}

/// Identifier for a specific cover slot — globally unique across the process.
///
/// The reservation table is keyed on this. A node currently maps 1:1 to a
/// slot (the in-memory `CoverNodePrefabData` does not yet model multi-slot
/// per-node — the on-disk format's `tail` bytes may encode slot data but
/// remain unconfirmed). The combination `(chunk_id, node_id)` is the
/// natural key — be aware that `node_id` is sequential per chunk and
/// stable only for as long as the extractor's input pak ordering stays
/// the same; do NOT persist this key in saved-game data without also
/// persisting the `chunk_name` for a re-extract-safe lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoverSlotKey {
    pub chunk_id: i32,
    pub node_id: i32,
}

impl CoverSlotKey {
    pub fn new(chunk_id: i32, node_id: i32) -> Self {
        Self { chunk_id, node_id }
    }
}

/// A single cover node loaded from `resources.cover_nodes`. Position is
/// in BigWorld meters; `orient` is the node's defensive facing in
/// radians. The cover is considered to defend against threats whose
/// vector-from-node falls within ±π/2 of `orient`.
#[derive(Debug, Clone)]
pub struct CoverNode {
    pub chunk_id: i32,
    pub node_id: i32,
    pub pos: Vector3,
    pub orient: f32,
    pub height: CoverHeight,
    pub quality: CoverQuality,
    /// Raw 4-byte tail from the on-disk record. Semantics unconfirmed;
    /// preserved for future RE. Most-likely candidates: secondary lean
    /// angle f32, or width+flags packed bytes.
    pub tail: [u8; 4],
}

impl CoverNode {
    pub fn key(&self) -> CoverSlotKey {
        CoverSlotKey::new(self.chunk_id, self.node_id)
    }
}

/// Metadata about a cover set (the parent grouping of cover nodes —
/// one row per chunk in `resources.cover_sets`).
#[derive(Debug, Clone)]
pub struct CoverSetMeta {
    pub chunk_id: i32,
    pub chunk_name: String,
    pub primary_author: String,
    pub has_variant: bool,
    pub src_pak: String,
}

/// The cover service handle. Carried on the `SpaceManager` alongside the
/// navmesh + entity tables.
///
/// `index` is the spatial index of all loaded cover nodes (rebuilt at
/// startup; never mutated at runtime — node positions are fixed). The
/// `reservations` map is hot — mutated on every `reserve_cover_slot` /
/// `release_cover_slot` from the NPC AI tick.
#[derive(Debug, Clone)]
pub struct Cover {
    /// Per-chunk set metadata, indexed by `chunk_id`.
    pub sets: Arc<Vec<CoverSetMeta>>,
    /// Spatial index over every loaded cover node.
    pub index: Arc<CoverIndex>,
    /// Live reservation table.
    pub reservations: Arc<Mutex<CoverReservations>>,
}

impl Cover {
    /// Build an empty cover service. Used in tests and as the
    /// no-cover-data fallback when the DB tables are empty (which
    /// keeps the cell service functional in unit-test contexts).
    pub fn empty() -> Self {
        Self {
            sets: Arc::new(Vec::new()),
            index: Arc::new(CoverIndex::empty()),
            reservations: Arc::new(Mutex::new(CoverReservations::default())),
        }
    }

    /// Build from loaded data. Takes ownership; intended for cell-service
    /// startup.
    pub fn from_loaded(sets: Vec<CoverSetMeta>, nodes: Vec<CoverNode>) -> Self {
        Self {
            sets: Arc::new(sets),
            index: Arc::new(CoverIndex::build(nodes)),
            reservations: Arc::new(Mutex::new(CoverReservations::default())),
        }
    }

    /// Total node count loaded (post-build).
    pub fn node_count(&self) -> usize {
        self.index.node_count()
    }

    /// Total cover-set count loaded.
    pub fn set_count(&self) -> usize {
        self.sets.len()
    }

    /// Release the cover slot currently held by `entity_id`, if any.
    /// Idempotent — returns the freed slot if there was one.
    ///
    /// Recovers gracefully from a poisoned reservations mutex: emits a
    /// `tracing::warn!` and continues with the recovered inner state
    /// rather than panicking the cell process. A poisoned cover table
    /// is recoverable — the worst case is some stale reservations get
    /// retired one tick later than they should.
    pub fn release_for_entity(&self, entity_id: EntityId) -> Option<CoverSlotKey> {
        let mut r = match self.reservations.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::warn!(
                    entity_id = entity_id.0,
                    "cover reservations mutex poisoned during release_for_entity — recovering"
                );
                poisoned.into_inner()
            }
        };
        r.release_for_entity(entity_id)
    }
}
