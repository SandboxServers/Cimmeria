//! Per-world navmesh containment mode: the [`NavmeshMode`] enum, its
//! `resources.worlds.navmesh_mode` spelling and the fail-closed parser the
//! world loader uses.
//!
//! The one predicate every containment gate consults,
//! `SpaceManager::enforces_navmesh_containment`, lives with `SpaceManager`
//! (`cell/space_manager/navmesh_containment.rs`); this half is plain data,
//! so the DB loaders can own it.
//!
//! # Why a mode exists at all
//!
//! A `data/spaces/<world>.nav` mesh is used for two unrelated things:
//!
//! * **Information** — `find_path`, `line_of_sight`, `get_navmesh_height`,
//!   NPC wander validity, the `on_navmesh` field the `.bug` report and the
//!   spawn log print. A wrong answer here degrades an NPC or a diagnostic.
//! * **Containment** — a hard gate on where a *player* may be. A wrong
//!   answer here snaps the player back every packet, which reads in-game as
//!   an invisible wall with no message.
//!
//! A world with **no** mesh fails open in both roles, which is survivable.
//! A world with a **partial** mesh fails open as information and *closed* as
//! containment, which is worse: the holes become invisible walls. Harset is
//! the worked example — its plaza floor has no coverage from Z -200 to
//! Z -228 across the only walk to the Command Center door, so an ordinary
//! player cannot get there. No tester saw it because a GM is warn-only on
//! this layer, and every Harset tester so far was a GM.
//!
//! [`NavmeshMode::Advisory`] says "this mesh is information, not a gate".
//! An advisory world behaves for every containment purpose exactly as a
//! world with no mesh at all, while keeping every informational consumer.
//!
//! # Why it is explicit data and not a heuristic
//!
//! Containment is a server-authority gate. Auto-detecting a "bad" mesh and
//! flipping the mode at runtime would let a gate turn itself off from
//! conditions an attacker can influence (where entities stand, which polys
//! get queried). The mode is a seeded column, changed by a human editing
//! `db/resources/Worlds/Seed/worlds.sql`, and an unparseable value falls
//! back to [`NavmeshMode::Enforce`] — the strict side.
//!
//! Bounds, speed and teleport validation are untouched by the mode. An
//! advisory world still rejects NaN, out-of-AABB and teleport-sized moves.

/// How a world's navmesh may be used against player movement.
///
/// Persisted as `resources.worlds.navmesh_mode`, a `varchar(16)` with a
/// `CHECK` constraint and a `'enforce'` default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavmeshMode {
    /// The mesh is a containment gate. Today's behaviour for every world
    /// but Harset, and the default for a world that has no row, no stamp
    /// (DB down at startup), or an unrecognised column value.
    #[default]
    Enforce,
    /// The mesh is information only. Containment gates treat this world
    /// exactly like a world with no mesh; `find_path`, line of sight,
    /// height sampling and the `on_navmesh` diagnostics all still use it.
    Advisory,
}

impl NavmeshMode {
    /// The spelling used in `resources.worlds.navmesh_mode`.
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Enforce => "enforce",
            Self::Advisory => "advisory",
        }
    }
}

impl TryFrom<&str> for NavmeshMode {
    type Error = ();

    /// Exact, case-sensitive: the column has a `CHECK` constraint that only
    /// admits the two lowercase spellings, so anything else reached this
    /// process by a route that bypassed the schema and should not be
    /// silently normalised into a weaker gate.
    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        match raw {
            "enforce" => Ok(Self::Enforce),
            "advisory" => Ok(Self::Advisory),
            _ => Err(()),
        }
    }
}

/// Parse a `resources.worlds.navmesh_mode` value, falling back to
/// [`NavmeshMode::Enforce`] with a WARN.
///
/// Fail-*closed*, deliberately, and the opposite of how this module's
/// runtime predicate fails: an unreadable column value is a data defect,
/// and quietly demoting a world's containment gate because of one is how a
/// movement gate disappears without anyone deciding to remove it. The WARN
/// names the world so the row is findable.
pub(crate) fn mode_from_db_value(world: &str, raw: &str) -> NavmeshMode {
    NavmeshMode::try_from(raw).unwrap_or_else(|()| {
        tracing::warn!(
            target: "movement.navmesh",
            world_name = %world,
            raw_value = %raw,
            reason = "navmesh_mode_unrecognised",
            "navmesh: resources.worlds.navmesh_mode holds a value this build \
             does not know -- falling back to 'enforce' (containment stays on)"
        );
        NavmeshMode::Enforce
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two spellings the `CHECK` constraint admits round-trip, and
    /// nothing else parses. A future third mode that forgets an arm here
    /// would otherwise arrive as `Enforce` with no signal.
    #[test]
    fn only_the_two_db_spellings_parse() {
        assert_eq!(NavmeshMode::try_from("enforce"), Ok(NavmeshMode::Enforce));
        assert_eq!(NavmeshMode::try_from("advisory"), Ok(NavmeshMode::Advisory));
        assert_eq!(NavmeshMode::Enforce.as_db_str(), "enforce");
        assert_eq!(NavmeshMode::Advisory.as_db_str(), "advisory");
        for bad in ["Advisory", "ADVISORY", "advisory ", "", "off", "none"] {
            assert!(
                NavmeshMode::try_from(bad).is_err(),
                "{bad:?} must not parse -- a near-miss spelling silently \
                 becoming a mode is how a movement gate disappears",
            );
        }
    }

    /// An unreadable column value keeps containment **on**. The fallback
    /// direction is the whole safety argument for making the mode data.
    #[test]
    fn an_unrecognised_column_value_falls_back_to_enforce() {
        assert_eq!(
            mode_from_db_value("Harset", "Advisory"),
            NavmeshMode::Enforce,
        );
        assert_eq!(mode_from_db_value("Harset", ""), NavmeshMode::Enforce);
        assert_eq!(
            mode_from_db_value("Harset", "advisory"),
            NavmeshMode::Advisory,
        );
    }
}
