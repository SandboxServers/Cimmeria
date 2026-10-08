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
//! A refused player is sent to their start profile's world and point
//! (`cimmeria_resources::base::start_profiles`, Class Start v6 CS-02): the
//! profile of their alignment and archetype, so a Free Jaffa goes to
//! Dakara_E1, an SGU human to SGC_W1 and Praxis to the Castle_CellBlock
//! stasis room. They are told why in chat once the world has loaded, and
//! logged at WARN with player and world names. With no start profile loaded
//! the entry is refused instead (lock L3: never a guessed world). Movement
//! inside a world never changes the world, so it needs no check.

use cimmeria_resources::base::start_profiles::{self, StartProfiles};

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

/// What the GM-only rule says about one entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GmOnlyDecision {
    /// Not a GM-only world, or a GM.
    Allowed,
    /// Refused; go to the start profile's home instead.
    Redirect(GmOnlyRedirect),
    /// Refused, and no start profile names a home (the boot load failed).
    /// The caller refuses the entry outright, logged at ERROR.
    NoHome { refused_world: &'static str },
}

/// A start profile's world and point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StartHome {
    pub world: &'static str,
    pub position: [f32; 3],
}

/// Whether `world` is GM-only.
pub fn is_gm_only_world(world: &str) -> bool {
    GM_ONLY_WORLDS.contains(&world)
}

/// Whether a player at `access_level` may enter `world`: a GM-only world
/// and a level below [`GM_ACCESS_LEVEL`] is refused, and the player goes to
/// the home of their `alignment` and `archetype` (`sgw_player` ordinals)
/// from the process start profiles.
pub fn gm_only_redirect(
    world: &str,
    access_level: u32,
    alignment: i32,
    archetype: i32,
) -> GmOnlyDecision {
    gm_only_redirect_in(
        start_profiles::installed().as_deref(),
        world,
        access_level,
        alignment,
        archetype,
    )
}

/// [`gm_only_redirect`] against an explicit profile set (`None`: none
/// loaded).
pub fn gm_only_redirect_in(
    profiles: Option<&StartProfiles>,
    world: &str,
    access_level: u32,
    alignment: i32,
    archetype: i32,
) -> GmOnlyDecision {
    if access_level >= GM_ACCESS_LEVEL {
        return GmOnlyDecision::Allowed;
    }
    let Some(refused_world) = GM_ONLY_WORLDS.iter().copied().find(|w| *w == world) else {
        return GmOnlyDecision::Allowed;
    };
    match profiles.and_then(|p| home_in(p, alignment, archetype)) {
        Some(home) => GmOnlyDecision::Redirect(GmOnlyRedirect {
            refused_world,
            world: home.world,
            position: home.position,
        }),
        None => GmOnlyDecision::NoHome { refused_world },
    }
}

/// The start of a character of `alignment` and `archetype`, from the
/// process start profiles: Praxis to the Castle_CellBlock stasis room, SGU
/// humans (and the Asgard holding state) to SGC_W1, Free Jaffa to Dakara_E1.
/// `None` when no profile is loaded or none matches.
pub fn home_for(alignment: i32, archetype: i32) -> Option<StartHome> {
    home_in(
        start_profiles::installed().as_deref()?,
        alignment,
        archetype,
    )
}

/// [`home_for`] against an explicit profile set.
pub fn home_in(profiles: &StartProfiles, alignment: i32, archetype: i32) -> Option<StartHome> {
    let p = profiles.home_for(alignment, archetype)?;
    Some(StartHome {
        world: cimmeria_entity::name_intern::intern(&p.world)?,
        position: p.position,
    })
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
    use cimmeria_resources::base::start_profiles::fixture;

    const PRAXIS: i32 = 1;
    const SGU: i32 = 2;

    fn redirect(world: &str, level: u32, alignment: i32, archetype: i32) -> GmOnlyDecision {
        gm_only_redirect_in(Some(&fixture::seeded()), world, level, alignment, archetype)
    }

    /// A refused player goes to their own start profile's home. The Free
    /// Jaffa case is the CS-02 change: the old alignment-only rule sent an
    /// SGU Shol'va to SGC_W1.
    #[test]
    fn a_player_is_refused_a_gm_only_world_and_sent_to_their_profile_start() {
        let GmOnlyDecision::Redirect(r) = redirect("DebugArea", 0, PRAXIS, 2) else {
            panic!("a player is refused");
        };
        assert_eq!(r.refused_world, "DebugArea");
        assert_eq!(r.world, "Castle_CellBlock");
        assert_eq!(r.position, fixture::CELLBLOCK_START);
        let GmOnlyDecision::Redirect(r) = redirect("DebugArea", 1, SGU, 1) else {
            panic!("a moderator is refused");
        };
        assert_eq!((r.world, r.position), ("SGC_W1", fixture::SGC_W1_START));
        let GmOnlyDecision::Redirect(r) = redirect("DebugArea", 0, SGU, 7) else {
            panic!("a Free Jaffa is refused");
        };
        assert_eq!(
            (r.world, r.position),
            ("Dakara_E1", fixture::DAKARA_E1_START)
        );
    }

    /// Lock L3: with no start profile loaded, the entry is refused outright
    /// rather than guessed into Castle_CellBlock.
    #[test]
    fn no_loaded_profile_refuses_instead_of_guessing() {
        assert_eq!(
            gm_only_redirect_in(None, "DebugArea", 0, PRAXIS, 1),
            GmOnlyDecision::NoHome {
                refused_world: "DebugArea"
            }
        );
        assert_eq!(
            gm_only_redirect_in(None, "Harset", 0, PRAXIS, 1),
            GmOnlyDecision::Allowed,
            "the profiles are only read for a refusal"
        );
    }

    #[test]
    fn home_in_follows_the_profile() {
        let profiles = fixture::seeded();
        let home = |a, b| home_in(&profiles, a, b).map(|h| h.world);
        assert_eq!(home(SGU, 7), Some("Dakara_E1"));
        assert_eq!(home(SGU, 5), Some("SGC_W1"));
        assert_eq!(home(PRAXIS, 6), Some("Castle_CellBlock"));
        assert_eq!(home(PRAXIS, 8), Some("Castle_CellBlock"));
    }

    #[test]
    fn a_redirect_owes_the_player_one_line() {
        // A player id no other test uses: the store is process-wide.
        const PLAYER: i32 = 0x7DA0_0001;
        let GmOnlyDecision::Redirect(r) = redirect("DebugArea", 0, PRAXIS, 1) else {
            panic!("refused");
        };
        assert_eq!(take_redirect_line(PLAYER), None);
        note_gm_only_redirect("login", PLAYER, Some("Tester"), Some(1), None, 0, &r);
        assert_eq!(take_redirect_line(PLAYER), Some(REDIRECT_LINE));
        assert_eq!(take_redirect_line(PLAYER), None, "once");
    }

    #[test]
    fn a_gm_or_any_other_world_passes() {
        for level in [2, 3, 4, 99] {
            assert_eq!(
                redirect("DebugArea", level, PRAXIS, 1),
                GmOnlyDecision::Allowed,
                "level {level}"
            );
        }
        for world in ["Castle_CellBlock", "SGC_W1", "Harset", "debugarea", ""] {
            assert_eq!(
                redirect(world, 0, PRAXIS, 1),
                GmOnlyDecision::Allowed,
                "{world}"
            );
        }
    }
}
