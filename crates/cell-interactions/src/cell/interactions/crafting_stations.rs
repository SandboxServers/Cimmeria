//! Crafting stations in reach of a player (CR-05, D-CR05).
//!
//! A station is any entity whose `entity_flags` carry an
//! `ENTITYFLAG_Craft_*` bit; each bit makes it a station for one verb. A
//! player can use a station within [`MAX_INTERACT_DISTANCE`], the same
//! bound (and the same 3-D distance) `interact` enforces.
//!
//! Two callers share [`stations_in_range`]: the 1 Hz station tick, which
//! reports changes to the base for `onUpdateCraftingOptions`, and the
//! crafting forward, which computes `CraftRequest::allowed` fresh at request
//! time. The client does no distance check of its own (CR-E1 Q2), so the
//! forward's mask is the authoritative one.

use cimmeria_cell_catalog::crafting::CraftType;
use cimmeria_wire::crafting::StationSet;

use super::dispatch::MAX_INTERACT_DISTANCE;
use crate::cell::space_manager::SpaceManager;

/// Every `ENTITYFLAG_Craft_*` bit.
const ANY_STATION_FLAG: u64 = {
    let mut mask = 0u64;
    let mut i = 0;
    while i < CraftType::ALL.len() {
        mask |= CraftType::ALL[i].entity_flag() as u64;
        i += 1;
    }
    mask
};

/// The nearest station per verb among `candidates` (entity id, entity
/// flags, squared distance to the player), within [`MAX_INTERACT_DISTANCE`].
///
/// Equal distances resolve to the lower entity id, so the answer does not
/// depend on the order the spatial grid returns entities in (the client keeps
/// only one machine per section, and a flapping id would resend 140).
pub fn nearest_stations(candidates: impl IntoIterator<Item = (u32, u64, f32)>) -> StationSet {
    let max_sq = MAX_INTERACT_DISTANCE * MAX_INTERACT_DISTANCE;
    let mut best: [Option<(f32, u32)>; 4] = [None; 4];
    for (entity_id, flags, dist_sq) in candidates {
        if flags & ANY_STATION_FLAG == 0 || dist_sq > max_sq {
            continue;
        }
        for (slot, verb) in best.iter_mut().zip(CraftType::ALL) {
            if flags & verb.entity_flag() as u64 == 0 {
                continue;
            }
            let closer = match *slot {
                None => true,
                Some((d, id)) => dist_sq < d || (dist_sq == d && entity_id < id),
            };
            if closer {
                *slot = Some((dist_sq, entity_id));
            }
        }
    }
    best.map(|slot| slot.map(|(_, id)| id))
}

/// The `ECraftTypeFlags` mask `stations` grants: one bit per verb with a
/// station.
pub fn station_mask(stations: &StationSet) -> u8 {
    stations
        .iter()
        .zip(CraftType::ALL)
        .filter(|(station, _)| station.is_some())
        .fold(0, |mask, (_, verb)| mask | verb.bit())
}

/// The stations in reach of `entity_id` in its own space. All `None` when
/// the entity is unknown.
pub fn stations_in_range(space_mgr: &SpaceManager, entity_id: u32) -> StationSet {
    let Some(space) = space_mgr
        .get_entity_space_id(entity_id)
        .and_then(|space_id| space_mgr.spaces.get(&space_id))
    else {
        return [None; 4];
    };
    let Some(player) = space.entities.get(&entity_id) else {
        return [None; 4];
    };
    // The grid is cell-granular, so it over-returns; `nearest_stations`
    // applies the exact distance.
    let candidates = space
        .space
        .get_entities_in_range(&player.position, MAX_INTERACT_DISTANCE)
        .into_iter()
        .filter_map(|id| u32::try_from(id.0).ok())
        .filter(|&id| id != entity_id)
        .filter_map(|id| {
            let e = space.entities.get(&id)?;
            Some((
                id,
                e.entity_flags,
                player.position.distance_squared_to(&e.position),
            ))
        });
    nearest_stations(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_cell_catalog::crafting::{
        ENTITYFLAG_CRAFT_ALLOYING, ENTITYFLAG_CRAFT_CRAFT, ENTITYFLAG_CRAFT_RESEARCH,
        ENTITYFLAG_CRAFT_REV_ENG,
    };

    const ALL_VERBS: u64 = (ENTITYFLAG_CRAFT_CRAFT
        | ENTITYFLAG_CRAFT_RESEARCH
        | ENTITYFLAG_CRAFT_REV_ENG
        | ENTITYFLAG_CRAFT_ALLOYING) as u64;

    /// A candidate `x` units from the player: the squared distance.
    fn at(x: f32) -> f32 {
        x * x
    }

    #[test]
    fn station_in_range_fills_every_section_its_flags_name() {
        let got = nearest_stations([(900, ALL_VERBS, at(4.0))]);
        assert_eq!(got, [Some(900); 4]);
        assert_eq!(station_mask(&got), 0x0F);
    }

    /// Exactly `MAX_INTERACT_DISTANCE` is in range (the interact gate is
    /// `>`), just past it is not.
    #[test]
    fn range_boundary_matches_interact() {
        assert_eq!(
            nearest_stations([(900, ALL_VERBS, at(MAX_INTERACT_DISTANCE))]),
            [Some(900); 4]
        );
        assert_eq!(
            nearest_stations([(900, ALL_VERBS, at(MAX_INTERACT_DISTANCE + 0.01))]),
            [None; 4]
        );
    }

    #[test]
    fn each_verb_takes_its_own_nearest_station() {
        let got = nearest_stations([
            (901, ENTITYFLAG_CRAFT_CRAFT as u64, at(3.0)),
            (902, ALL_VERBS, at(1.0)),
            (903, ENTITYFLAG_CRAFT_ALLOYING as u64, at(0.5)),
            (904, ENTITYFLAG_CRAFT_RESEARCH as u64, at(2.0)),
        ]);
        assert_eq!(got, [Some(902), Some(902), Some(902), Some(903)]);
    }

    #[test]
    fn entities_without_craft_flags_are_not_stations() {
        // 1 | 2 | 1024: other EEntityFlags bits.
        assert_eq!(nearest_stations([(900, 1027, at(1.0))]), [None; 4]);
        assert_eq!(station_mask(&[None; 4]), 0);
    }

    #[test]
    fn a_research_only_station_grants_only_research() {
        let got = nearest_stations([(900, ENTITYFLAG_CRAFT_RESEARCH as u64, at(1.0))]);
        assert_eq!(got, [None, Some(900), None, None]);
        assert_eq!(station_mask(&got), CraftType::Research.bit());
    }

    /// Ties go to the lower id whatever order the grid yields.
    #[test]
    fn equal_distance_picks_the_lower_id_in_any_order() {
        let a = (905, ALL_VERBS, at(2.0));
        let b = (904, ALL_VERBS, at(-2.0));
        assert_eq!(nearest_stations([a, b]), [Some(904); 4]);
        assert_eq!(nearest_stations([b, a]), [Some(904); 4]);
    }
}
