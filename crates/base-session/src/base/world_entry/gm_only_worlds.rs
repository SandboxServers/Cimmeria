//! GM-only worlds (Debug Area D-DA4): which worlds a non-GM may not enter,
//! and where such a player goes instead.
//!
//! D-DA4 kept the Debug Area GM-only by giving it no stargate row, but
//! nothing on the server enforced it: a GM `.summon` brought a player there,
//! a demoted GM stayed there, and relog or respawn kept them there. The
//! Debug Area has 1-naquadah vendors and GM tooling a player must never
//! reach.
//!
//! Every way into a world ends in one of two base paths, and both ask
//! [`gm_only_redirect`] before the cell creates the entity:
//!
//! - **login** (`world_entry_db::query_world_entry`): the saved
//!   `world_location`, so a relog or a demoted GM's saved spot;
//! - **cross-world travel** (`gate_travel::handle_gate_travel`): every
//!   stargate, ring, content teleport, respawn into another world and GM
//!   `.summon` / `.gotolocation` across worlds.
//!
//! A refused player is sent to their faction's character-creation start
//! point (Praxis: the Castle_CellBlock stasis room; SGU: SGC_W1), told why
//! in chat once the world has loaded, and logged at WARN with player and
//! world names. Movement inside a world never changes the world, so it
//! needs no check.

/// Worlds only an account at GameMaster level or above may enter.
pub const GM_ONLY_WORLDS: &[&str] = &["DebugArea"];

/// The lowest `account.accesslevel` that may enter a GM-only world: 2,
/// `AccessLevel::GameMaster`, the same threshold as the cell's GM gate
/// (`cimmeria_cell_world::cell::dispatch::gm_gate::is_gm`).
pub const GM_ACCESS_LEVEL: u32 = 2;

/// The chat line a redirected player sees once the home world has loaded.
pub const REDIRECT_LINE: &str =
    "The Debug Area is for GMs only. You have been returned to your faction's starting point.";

/// Where a refused player goes instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GmOnlyRedirect {
    /// The refused world.
    pub refused_world: &'static str,
    /// The home world.
    pub world: &'static str,
    /// The home point.
    pub position: [f32; 3],
}

/// Whether `world` is GM-only.
pub fn is_gm_only_world(world: &str) -> bool {
    GM_ONLY_WORLDS.contains(&world)
}

/// `Some(redirect)` when a player at `access_level` may not enter `world`:
/// a GM-only world and a level below [`GM_ACCESS_LEVEL`]. `alignment` is
/// `sgw_player.alignment` (2 = SGU, anything else Praxis).
pub fn gm_only_redirect(world: &str, access_level: u32, alignment: i32) -> Option<GmOnlyRedirect> {
    if access_level >= GM_ACCESS_LEVEL {
        return None;
    }
    let refused_world = GM_ONLY_WORLDS.iter().copied().find(|w| *w == world)?;
    let (home, position) = home_for_alignment(alignment);
    Some(GmOnlyRedirect {
        refused_world,
        world: home,
        position,
    })
}

/// The character-creation start point of `alignment`: the Praxis start in
/// the Castle_CellBlock stasis room, or the SGU start in SGC_W1. The same
/// points `cimmeria_resources::base::chardef` hands a new character.
pub fn home_for_alignment(alignment: i32) -> (&'static str, [f32; 3]) {
    const SGU: i32 = 2;
    let world = if alignment == SGU {
        "SGC_W1"
    } else {
        "Castle_CellBlock"
    };
    let position = cimmeria_resources::base::chardef::starting_position(world)
        .unwrap_or([-334.231, 73.472, -228.026]);
    (world, position)
}

/// Characters redirected away from a GM-only world whose home world has
/// not loaded yet, keyed by `player_id`. The chat line cannot go out at
/// redirect time (the client is between worlds, with no player entity), so
/// `handle_on_client_ready` takes it with [`take_redirect_line`] once the
/// home world is up. A process-wide map rather than a session field: the
/// entry is written by the login query and the gate-travel handler, which
/// hold the player id but not always the session.
fn pending_lines() -> &'static std::sync::Mutex<std::collections::HashSet<i32>> {
    static PENDING: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<i32>>> =
        std::sync::OnceLock::new();
    PENDING.get_or_init(Default::default)
}

/// Log a refused entry (WARN, with player and world names) and queue the
/// player's chat line. `route` is `login` or `gate_travel`.
pub fn note_gm_only_redirect(
    route: &'static str,
    player_id: i32,
    player_name: Option<&str>,
    account_id: Option<u32>,
    account_name: Option<&str>,
    access_level: u32,
    redirect: &GmOnlyRedirect,
) {
    // Module-path target: exported by `OTEL_FILTER`'s base-session row.
    tracing::warn!(
        event = "gm_only_world_refused",
        route,
        player_id,
        player_name,
        account_id,
        account_name,
        access_level,
        refused_world = redirect.refused_world,
        world = redirect.world,
        "a non-GM was headed for a GM-only world; sent to the faction start instead"
    );
    if let Ok(mut set) = pending_lines().lock() {
        set.insert(player_id);
    }
}

/// The chat line owed to `player_id` for a GM-only redirect, once. `None`
/// when there is none.
pub fn take_redirect_line(player_id: i32) -> Option<&'static str> {
    let mut set = pending_lines().lock().ok()?;
    set.remove(&player_id).then_some(REDIRECT_LINE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_is_refused_a_gm_only_world_and_sent_home() {
        let r = gm_only_redirect("DebugArea", 0, 1).expect("a player is refused");
        assert_eq!(r.refused_world, "DebugArea");
        assert_eq!(r.world, "Castle_CellBlock");
        assert_eq!(r.position, [-334.231, 73.472, -228.026]);
        let r = gm_only_redirect("DebugArea", 1, 2).expect("a moderator is refused");
        assert_eq!(r.world, "SGC_W1");
    }

    #[test]
    fn a_redirect_owes_the_player_one_line() {
        // A player id no other test uses: the store is process-wide.
        const PLAYER: i32 = 0x7DA0_0001;
        let r = gm_only_redirect("DebugArea", 0, 1).unwrap();
        assert_eq!(take_redirect_line(PLAYER), None);
        note_gm_only_redirect("login", PLAYER, Some("Tester"), Some(1), None, 0, &r);
        assert_eq!(take_redirect_line(PLAYER), Some(REDIRECT_LINE));
        assert_eq!(take_redirect_line(PLAYER), None, "once");
    }

    #[test]
    fn a_gm_or_any_other_world_passes() {
        for level in [2, 3, 4, 99] {
            assert_eq!(
                gm_only_redirect("DebugArea", level, 1),
                None,
                "level {level}"
            );
        }
        for world in ["Castle_CellBlock", "SGC_W1", "Harset", "debugarea", ""] {
            assert_eq!(gm_only_redirect(world, 0, 1), None, "{world}");
        }
    }
}
