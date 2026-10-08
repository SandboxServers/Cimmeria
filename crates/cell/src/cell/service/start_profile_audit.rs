//! Boot audit of the start profiles against this cell's worlds (Class Start
//! v6, CS-02, lock L3).
//!
//! Character creation refuses a profile whose world the cell cannot deliver
//! a player to, but only when someone tries to create one. This pass names
//! every such profile at startup, and every spawn point the world's
//! containment navmesh rejects, so a bad seed row is an ERROR in SigNoz on
//! boot rather than a player's failed creation.

use cimmeria_cell_world::cell::arrival::{check_arrival, ArrivalCheck};
use cimmeria_resources::base::start_profiles::{self, StartProfiles};

use super::super::space_manager::SpaceManager;

/// What is wrong with one profile on this cell.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Finding {
    /// The cell has no startup space and no instanced definition for it.
    WorldNotEnterable {
        char_def_id: i32,
        profile_id: String,
        world: String,
    },
    /// The world enforces its navmesh and the spawn point is off it.
    SpawnOffMesh {
        char_def_id: i32,
        profile_id: String,
        world: String,
        position: [f32; 3],
    },
}

/// Every finding for `profiles`, in char_def order. `enterable` and
/// `arrival` are the cell's answers, injected so the rule is testable
/// without a loaded world.
pub(super) fn findings(
    profiles: &StartProfiles,
    enterable: impl Fn(&str) -> bool,
    arrival: impl Fn(&str, [f32; 3]) -> ArrivalCheck,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for p in profiles.profiles() {
        if !enterable(&p.world) {
            out.push(Finding::WorldNotEnterable {
                char_def_id: p.char_def_id,
                profile_id: p.profile_id.clone(),
                world: p.world.clone(),
            });
        } else if arrival(&p.world, p.position) == ArrivalCheck::OffMesh {
            out.push(Finding::SpawnOffMesh {
                char_def_id: p.char_def_id,
                profile_id: p.profile_id.clone(),
                world: p.world.clone(),
                position: p.position,
            });
        }
    }
    out
}

/// Log every finding for the installed profiles at ERROR.
pub(super) fn audit_start_profiles(space_mgr: &SpaceManager) {
    let Some(profiles) = start_profiles::installed() else {
        // `load_at_boot` already logged why.
        return;
    };
    let found = findings(
        &profiles,
        |w| space_mgr.world_is_enterable(w),
        |w, p| check_arrival(space_mgr, w, p),
    );
    for f in &found {
        match f {
            Finding::WorldNotEnterable {
                char_def_id,
                profile_id,
                world,
            } => tracing::error!(
                event = "start_profile_invalid",
                reason = "start_world_not_enterable",
                char_def_id, // nt:id-only char_def rows carry the profile id below
                profile_id = %profile_id, // nt:id-only profile key has no display name
                world = %world,
                "start profile names a world this cell cannot deliver a player to; \
                 character creation refuses it"
            ),
            Finding::SpawnOffMesh {
                char_def_id,
                profile_id,
                world,
                position,
            } => tracing::error!(
                event = "start_profile_invalid",
                reason = "start_spawn_off_mesh",
                char_def_id, // nt:id-only char_def rows carry the profile id below
                profile_id = %profile_id, // nt:id-only profile key has no display name
                world = %world,
                ?position,
                "start profile's spawn point is off its world's navmesh"
            ),
        }
    }
    tracing::info!(
        event = "start_profiles_audited",
        profiles = profiles.profiles().len(),
        findings = found.len(),
        "Audited start profiles against this cell's worlds"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_resources::base::start_profiles::fixture;

    /// A profile whose world the cell cannot deliver to, and one whose point
    /// the mesh rejects, are each named once; the seed against the real
    /// worlds has neither.
    #[test]
    fn findings_name_the_unenterable_world_and_the_off_mesh_spawn() {
        let profiles = fixture::seeded();
        let all = |_: &str| true;
        let on = |_: &str, _: [f32; 3]| ArrivalCheck::Validated;
        assert_eq!(findings(&profiles, all, on), vec![]);

        let no_dakara = |w: &str| w != "Dakara_E1";
        let found = findings(&profiles, no_dakara, on);
        let ids: Vec<i32> = found
            .iter()
            .map(|f| match f {
                Finding::WorldNotEnterable { char_def_id, .. } => *char_def_id,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(ids, vec![8, 18], "the two Free Jaffa profiles");

        let off_in_sgc = |w: &str, _: [f32; 3]| {
            if w == "SGC_W1" {
                ArrivalCheck::OffMesh
            } else {
                ArrivalCheck::Unvalidated
            }
        };
        let off = findings(&profiles, all, off_in_sgc);
        assert_eq!(off.len(), 9, "8 SGU humans + the Asgard holding state");
        assert!(off
            .iter()
            .all(|f| matches!(f, Finding::SpawnOffMesh { world, .. } if world == "SGC_W1")));
    }
}
