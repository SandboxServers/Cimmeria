//! Whether a player is present at an NPC event: the one gate for the
//! "nobody saw it" WARNs on the NPC path (DA-F2).
//!
//! An NPC method or attack sequence that reaches no witness is only a bug
//! when a player should have seen it. Two NPCs fighting with no player
//! around drop every `onStatUpdate`, `onStateFieldUpdate` and `onSequence`
//! by design: nobody is there to draw them. The Debug Area arena logged
//! about 55 such WARNs in its first 30 minutes on the colo (release
//! v2026-10-05.1), none of them a fault. Owner decision: "NPC vs NPC
//! warnings without players present can just go to null. Agree that if
//! issue is happening with player present throw warning."
//!
//! [`SpaceManager::player_present`] is that rule. Its callers are the
//! no-witness WARNs themselves (`abilities.wire event=wire_npc_no_witnesses`
//! and `abilities.sequence outcome=no_witnesses`), which only run once the
//! witness list came back empty. A player is present when:
//!
//! - **In range.** A player in the entity's space has the entity within that
//!   player's AoI radius. This is the case the WARN exists for: a player
//!   stands in range, but the witness list says nobody sees the NPC (a stale
//!   or broken witness set, #838's unrendered guard). It is the shooter
//!   alone that counts, because the witness list that came back empty is
//!   the shooter's: a player in range of only the event's counterpart (an
//!   attack's target) cannot see the shooter, so an empty list is correct
//!   (the Debug Area arena on the colo, 2026-10-05, a player 145 m from the
//!   guards and past 150 m from the Soldier shooting them). It walks
//!   `Space::players`, the same set `get_witnesses_of` has just walked, so
//!   it adds no new scan shape to the hot path.
//! - **Involved.** The entity itself, the counterpart, or anyone on the
//!   entity's threat list is player side ([`SpaceManager::is_player_side`]):
//!   a player, or an entity a player owns (a pet counts as its owner, a
//!   deployable, a GM's lab dummy). This covers a player fighting the NPC
//!   from beyond their own AoI.

use super::SpaceManager;
use cimmeria_entity::cell_entity::PetState;

impl SpaceManager {
    /// Whether `entity_id` is a player, or an entity a player owns: a pet
    /// (its owner), a deployable or a lab dummy. Unknown ids are not.
    pub fn is_player_side(&self, entity_id: u32) -> bool {
        if self.pets.is_pet(entity_id) || self.deployables.get(entity_id).is_some() {
            return true;
        }
        self.get_entity(entity_id).is_some_and(|e| {
            e.is_player
                || e.extensions.contains::<PetState>()
                || e.extensions.contains::<super::LabDummy>()
        })
    }

    /// Whether some player in `entity_id`'s space has it within the
    /// player's own AoI radius: the distance the AoI tick admits a witness
    /// at, read from the same `Space::players` set `get_witnesses_of` walks.
    pub fn player_in_aoi_of(&self, entity_id: u32) -> bool {
        let Some(space) = self
            .entity_space
            .get(&entity_id)
            .and_then(|sid| self.spaces.get(sid))
        else {
            return false;
        };
        let Some(pos) = space.entities.get(&entity_id).map(|e| e.position) else {
            return false;
        };
        space.players.iter().any(|pid| {
            *pid != entity_id
                && space
                    .entities
                    .get(pid)
                    .is_some_and(|p| p.position.distance_to(&pos) <= p.aoi_radius)
        })
    }

    /// The DA-F2 rule: whether a player is present at an event on
    /// `entity_id` whose other party (an attack's target) is `counterpart`.
    ///
    /// True when a player has `entity_id` within their AoI radius
    /// ([`Self::player_in_aoi_of`]; the counterpart's range is not tested,
    /// since the witness list that came back empty is `entity_id`'s), or
    /// when `entity_id`, `counterpart` or anyone on `entity_id`'s threat list
    /// is [player side](Self::is_player_side). A no-witness WARN on the NPC
    /// path is written only when this is true; an NPC-only event with no
    /// player anywhere near writes nothing.
    pub fn player_present(&self, entity_id: u32, counterpart: Option<u32>) -> bool {
        if self.is_player_side(entity_id)
            || self.player_in_aoi_of(entity_id)
            || counterpart.is_some_and(|id| self.is_player_side(id))
        {
            return true;
        }
        self.get_entity(entity_id).is_some_and(|e| {
            e.threat_list
                .keys()
                .any(|&threat| self.is_player_side(threat))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use cimmeria_entity::cell_entity::{MobAggression, PlayerIdentity};

    use super::*;
    use crate::cell::deployables::{DeployableState, PulseTotals};

    const NPC_A: u32 = 10;
    const NPC_B: u32 = 11;
    const PLAYER: u32 = 1;
    const PET: u32 = 12;
    const DEPLOYABLE: u32 = 13;
    const DUMMY: u32 = 14;

    /// Two NPCs at the origin and 20 u east, the player 500 u away (outside
    /// every AoI), and the player-owned entities next to the NPCs.
    fn mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(PLAYER, "Castle", [500.0, 0.0, 500.0], [0.0; 3])
            .unwrap();
        for (id, x) in [
            (NPC_A, 0.0),
            (NPC_B, 20.0),
            (PET, 5.0),
            (DEPLOYABLE, 6.0),
            (DUMMY, 7.0),
        ] {
            mgr.create_entity(id, "Castle", [x, 0.0, 0.0], [0.0; 3])
                .unwrap();
        }
        mgr.get_entity_mut(PLAYER).unwrap().is_player = true;
        mgr.connect_entity(PLAYER);
        mgr.pets.register(PLAYER, PET, mgr.player_identity(PLAYER));
        let now = Instant::now();
        mgr.deployables.register(
            DEPLOYABLE,
            DeployableState {
                owner: PLAYER,
                owner_identity: PlayerIdentity::UNKNOWN,
                ability_id: 1,
                template_id: 1,
                pulse_effect_id: 1,
                radius: 5.0,
                pulse_interval: Duration::from_secs(1),
                pulses_total: 1,
                spawned_at: now,
                next_pulse_at: now,
                totals: PulseTotals::default(),
            },
        );
        mgr.get_entity_mut(DUMMY)
            .unwrap()
            .extensions
            .insert(super::super::LabDummy {
                owner_id: PLAYER,
                owner_identity: PlayerIdentity::UNKNOWN,
                disposition: MobAggression::Hostile,
                expires_at: now + Duration::from_secs(600),
            });
        mgr
    }

    #[test]
    fn two_npcs_with_no_player_anywhere_are_not_present() {
        let mut mgr = mgr();
        mgr.get_entity_mut(NPC_A)
            .unwrap()
            .threat_list
            .insert(NPC_B, 5.0);
        assert!(!mgr.player_present(NPC_A, Some(NPC_B)));
        assert!(!mgr.player_present(NPC_A, None));
    }

    /// **Review probe (#1244).** A player 30 u from the NPC, inside their
    /// AoI, whose witness set never picked the NPC up (no AoI pass): the
    /// WARN's own fault case. Not involved in the fight, but present. Fails
    /// if the in-range clause is dropped.
    #[test]
    fn a_player_in_aoi_range_is_present_even_with_a_stale_witness_set() {
        let mut mgr = mgr();
        mgr.update_position_preserving_facing(PLAYER, [30.0, 0.0, 0.0], [0.0; 3]);
        assert!(mgr.get_witnesses_of(NPC_A).is_empty(), "fixture: stale");
        assert!(mgr.player_present(NPC_A, None));
    }

    /// **Regression guard (colo 2026-10-05, Debug Area arena).** A player in
    /// range of the counterpart only: NPC A (the target) 5 u inside the
    /// player's AoI, NPC B (the shooter) 15 u outside it. The player cannot
    /// see the shooter, so its empty witness list is correct and nothing is
    /// present. Fails if the counterpart's AoI range is tested again.
    #[test]
    fn a_player_in_range_of_only_the_counterpart_is_not_present() {
        let mut mgr = mgr();
        let r = mgr.get_entity(PLAYER).unwrap().aoi_radius;
        mgr.update_position_preserving_facing(PLAYER, [5.0 - r, 0.0, 0.0], [0.0; 3]);
        assert!(
            mgr.player_in_aoi_of(NPC_A) && !mgr.player_in_aoi_of(NPC_B),
            "fixture: the player sees A ({r} u AoI) and not B"
        );
        assert!(!mgr.player_present(NPC_B, Some(NPC_A)));
        // The other way round, the shooter in range, is present.
        assert!(mgr.player_present(NPC_A, Some(NPC_B)));
    }

    #[test]
    fn a_player_source_target_or_threat_entry_is_present() {
        let mut mgr = mgr();
        assert!(mgr.player_present(PLAYER, Some(NPC_A)));
        assert!(mgr.player_present(NPC_A, Some(PLAYER)));
        mgr.get_entity_mut(NPC_A)
            .unwrap()
            .threat_list
            .insert(PLAYER, 5.0);
        assert!(mgr.player_present(NPC_A, None));
    }

    /// Each player-owned kind counts as player side, as the target and on a
    /// threat list. Fails if any branch of `is_player_side` is dropped.
    #[test]
    fn a_pet_a_deployable_and_a_lab_dummy_count_as_their_owner() {
        let mut mgr = mgr();
        for owned in [PET, DEPLOYABLE, DUMMY] {
            assert!(mgr.is_player_side(owned), "{owned}");
            assert!(mgr.player_present(NPC_A, Some(owned)), "{owned} as target");
            assert!(mgr.player_present(owned, None), "{owned} as source");
            mgr.get_entity_mut(NPC_B).unwrap().threat_list.clear();
            mgr.get_entity_mut(NPC_B)
                .unwrap()
                .threat_list
                .insert(owned, 1.0);
            assert!(mgr.player_present(NPC_B, None), "{owned} on threat list");
        }
        assert!(!mgr.is_player_side(NPC_A));
    }
}
