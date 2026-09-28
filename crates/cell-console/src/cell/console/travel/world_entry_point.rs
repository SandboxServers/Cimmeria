//! A world's **entry point** — where a player lands the first time they go
//! there — for the travel commands that take a world without coordinates
//! (`.gotolocation <world>`, `.gotospace <spaceId>`).
//!
//! **Deliberate deviation — legacy has no equivalent.** Legacy
//! `gotoLocation` (`Player.py:344-365`) always required `x y z`. Owner
//! request 2026-09-27: naming the planet alone should drop the GM where a
//! player first arrives, so nobody has to look coordinates up.
//!
//! "Where a player first arrives" is answered from the same data the real
//! arrival paths use, in this order:
//!
//! 1. **Character-creation start** — `Castle_CellBlock` and `SGC_W1` are
//!    where new characters begin ([`starting_position`]). The historical
//!    CellBlocks (1201–1207) share the stock Cellblock's start.
//! 2. **Story arrival pad** — a world the story first delivers the player to
//!    by ring transport ([`STORY_ARRIVAL_PADS`]): Castle is reached from the
//!    Cellblock by mission 688's ring ceremony (chain 1109's
//!    `CrossWorldTeleport` lands on `Castle_ArmoryRingDropZone`), not
//!    through its gate.
//! 3. **Stargate arrival** — every other reachable world is first entered
//!    through its gate. The gate is the one the DHD picks (lowest
//!    `stargate_id` on the world), placed through [`validate_gate_arrival`],
//!    so the GM lands exactly where a gate traveller would, including the
//!    off-mesh respawner substitution.
//! 4. **First authored respawner** (lowest `respawner_id`, skipping the
//!    `(0,0,0)` placeholder rows) — gateless worlds such as the Harset
//!    interiors.
//!
//! A world with none of the three — or whose gate arrival is off-mesh with no
//! respawner to substitute — is refused with a line asking for coordinates
//! rather than guessing the origin.

use cimmeria_cell_world::cell::arrival::{check_arrival, validate_gate_arrival, ArrivalCheck};
use cimmeria_resources::base::chardef::starting_position;
use cimmeria_wire::mercury::world_data::historical_cellblocks::historical_cellblock;

use crate::cell::space_manager::SpaceManager;

/// Which rule produced an entry point — quoted in the GM feedback line so a
/// surprising landing spot says where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntrySource {
    CharacterStart,
    StoryRingPad,
    Stargate,
    Respawner,
}

impl EntrySource {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::CharacterStart => "new-character start",
            Self::StoryRingPad => "story ring pad",
            Self::Stargate => "stargate arrival",
            Self::Respawner => "respawner",
        }
    }
}

/// Resolve `world`'s entry point; `world` should already be the canonical
/// (`spaces.xml`) spelling, but every comparison is case-insensitive anyway
/// because the stargate and respawner tables carry the DB's spelling.
pub(crate) fn world_entry_point(
    space_mgr: &SpaceManager,
    world: &str,
) -> Option<([f32; 3], EntrySource)> {
    if let Some(pos) = starting_position(world) {
        return Some((pos, EntrySource::CharacterStart));
    }
    // The historical CellBlocks (1201–1207) are earlier builds of the same
    // cellblock, and a player goes in where a new character starts in the
    // stock one (owner, 2026-09-27). Exact match: `world` is canonical here.
    if historical_cellblock(world).is_some() {
        if let Some(pos) = starting_position("Castle_CellBlock") {
            return Some((pos, EntrySource::CharacterStart));
        }
    }

    if let Some(pos) = story_ring_pad(space_mgr, world) {
        return Some((pos, EntrySource::StoryRingPad));
    }

    let gate = space_mgr
        .stargates
        .iter()
        .filter(|(_, g)| g.world_name.eq_ignore_ascii_case(world))
        .min_by_key(|(id, _)| **id)
        .map(|(_, g)| g)
        // A gate whose arrival is the origin was never authored (Agnos's row
        // shipped all zeros) — the same sentinel reading respawners get.
        // Fall through to the respawner rule rather than land on it.
        .filter(|g| g.desired_arrival().0 != [0.0; 3]);
    if let Some(gate) = gate {
        let arrival = validate_gate_arrival(space_mgr, gate);
        if arrival.is_usable() {
            return Some((arrival.position, EntrySource::Stargate));
        }
        // `validate_gate_arrival` already tried every respawner the mesh
        // accepts, so a raw respawner here would be one the mesh rejects —
        // the silent-freeze shape `cell::arrival` exists to prevent, and the
        // subject may be a selected non-GM player. Refuse instead.
        tracing::warn!(
            world,
            reason = "gate_arrival_unusable",
            "console travel: the world's stargate arrival is off-mesh and no \
             respawner qualifies — no entry point"
        );
        return None;
    }

    space_mgr
        .respawners
        .iter()
        .filter(|r| r.world_name.eq_ignore_ascii_case(world))
        .filter(|r| r.pos != [0.0, 0.0, 0.0])
        .min_by_key(|r| r.respawner_id)
        .map(|r| (r.pos, EntrySource::Respawner))
}

/// `(world, ring tag)` for worlds whose story arrival is a ring pad.
///
/// Named by tag, not coordinates, so the position is read off the seeded
/// `ring_transport_regions` row the ring transport itself uses. A list
/// rather than a rule on purpose: "a pad with no outbound destinations"
/// would also match `CellblockRing3` and an unfinished Menfa Dark pad, and
/// neither is a story arrival.
const STORY_ARRIVAL_PADS: &[(&str, &str)] = &[
    // Mission 688's ring ceremony, chain 1109: `CrossWorldTeleport(Castle,
    // 466.365, 70.397, 991.466)` — region 34 (owner, 2026-09-27).
    ("Castle", "Castle_ArmoryRingDropZone"),
];

/// The story arrival pad for `world`, when it has one, is seeded and the
/// destination's navmesh (if it gates) accepts it. An unseeded or off-mesh
/// pad falls through to the stargate rule rather than stranding the subject.
fn story_ring_pad(space_mgr: &SpaceManager, world: &str) -> Option<[f32; 3]> {
    let (_, tag) = STORY_ARRIVAL_PADS
        .iter()
        .find(|(w, _)| w.eq_ignore_ascii_case(world))?;
    let Some(pad) = space_mgr
        .ring_regions
        .values()
        .find(|r| r.tag == *tag && r.world_name.eq_ignore_ascii_case(world))
    else {
        tracing::warn!(
            world,
            ring_tag = tag,
            reason = "story_pad_not_seeded",
            "console travel: the world's story arrival ring pad is not in \
             ring_transport_regions — falling back to the stargate"
        );
        return None;
    };
    let pos = [pad.x, pad.y, pad.z];
    if check_arrival(space_mgr, world, pos) == ArrivalCheck::OffMesh {
        tracing::warn!(
            world,
            ring_tag = tag,
            ?pos,
            reason = "story_pad_off_mesh",
            "console travel: the story arrival ring pad is off the navmesh — \
             falling back to the stargate"
        );
        return None;
    }
    Some(pos)
}
