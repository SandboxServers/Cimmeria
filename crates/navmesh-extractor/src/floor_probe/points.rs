//! The shipped Castle (World 8) probe point set.
//!
//! Split out of `mod.rs` because it is data, not logic: a list of world
//! coordinates with a provenance note each, which grows whenever a
//! playtest produces another known-walkable spot. Keeping it next to
//! the probe algebra made both harder to read.

use super::{Confidence, ProbePoint};

/// The Castle (World 8) probe set.
///
/// HIGH points come from live telemetry captured during the 2026-09-18
/// colo playtest — a player avatar stood on each. MEDIUM points are
/// `db/resources/Worlds/Seed/spawnlist.sql` rows for `world_id = 8`,
/// whose heights were reconstructed from map landmarks in
/// `docs/analysis/castle-rebuild/worknotes/ca05.md`; the room is
/// evidence-backed but the exact spot inside it is not.
pub fn castle_probe_points() -> Vec<ProbePoint> {
    use Confidence::{High, Medium};
    vec![
        ProbePoint::new(
            "Zuritska_cell",
            High,
            [268.0, 66.79, 1042.59],
            "playtest telemetry; also spawnlist 238 Castle_Zuritska_Cell",
        ),
        ProbePoint::new(
            "Romney_corridor_end",
            High,
            [244.0, 66.79, 1036.0],
            "playtest telemetry; also spawnlist 240 Castle_Romney",
        ),
        ProbePoint::new(
            "Level5_comms_room",
            High,
            [271.7, 55.2, 858.0],
            "playtest telemetry; also spawnlist 239 Castle_Zuritska_Comms",
        ),
        ProbePoint::new(
            "Comms_terminal",
            Medium,
            [271.7, 55.2, 855.0],
            "spawnlist 246 Castle_CommsTerminal",
        ),
        ProbePoint::new(
            "Checkpoint_Alpha_DHD",
            Medium,
            [806.27, 55.10, 517.24],
            "spawnlist 2 Castle_DHD",
        ),
        ProbePoint::new(
            "Checkpoint_Alpha_ColMarsh",
            Medium,
            [810.73, 55.20, 515.01],
            "spawnlist 118 Castle_ColMarsh",
        ),
        ProbePoint::new(
            "Interior_Coppleman",
            Medium,
            [352.69, 70.27, 952.32],
            "spawnlist 87 Castle_Coppleman",
        ),
        ProbePoint::new(
            "Interior_SgtGerschon",
            Medium,
            [429.64, 70.11, 996.56],
            "spawnlist 112 Castle_SgtGerschon",
        ),
        ProbePoint::new(
            "Bunker_Muelbach",
            Medium,
            [1008.0, 48.0, 414.0],
            "spawnlist 241 Castle_Muelbach (bunker floor y = 48.00)",
        ),
        ProbePoint::new(
            "Exterior_CheckpointBravo",
            Medium,
            [962.41, 24.80, 469.57],
            "spawnlist 101 Castle_NidGuard9 — OUTDOOR control point; \
             expected to stand on Terrain, which the StaticMesh path does not decode",
        ),
        ProbePoint::new(
            "Exterior_NidGuard16",
            Medium,
            [758.17, 29.88, 421.44],
            "spawnlist 110 Castle_NidGuard16 — second OUTDOOR control point",
        ),
        ProbePoint::new(
            "Interior_AccessPanel",
            Medium,
            [330.49, 41.18, 653.11],
            "spawnlist 92 Castle_AccessPanel",
        ),
    ]
}
