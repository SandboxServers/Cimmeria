//! Mesh-name safety net: kinds of mover that are never baked, whatever
//! the Kismet evidence says.
//!
//! The classifier's real evidence is Kismet ([`super::kismet_evidence`]).
//! These rules sit in front of it for the three kinds of `InterpActor`
//! whose cooked pose is the wrong answer often enough that one mistake
//! costs more than every correct inclusion gains:
//!
//! - **Doors.** A door's cooked pose is closed. The nine
//!   Castle_CellBlock cell doors no Matinee opens could be baked
//!   correctly, but that is an absence of evidence, and a door moved by
//!   something this classifier does not read would seal a doorway.
//! - **Stargate parts.** Chevrons, chevron lights and the inner-ring
//!   spinner are driven by the dialing sequence. In the 2009 client
//!   every gate's parts sit in its map's persistent level, which the
//!   extractor does not read (Login_Map is the one exception), so the
//!   rule guards against a re-cook rather than a shipped actor.
//! - **Security-camera heads.** Every one NA40 measured sweeps ±45° under
//!   Matinee.
//!
//! Matching is a case-insensitive substring test on the resolved
//! `StaticMesh` object name, so `SGC_small_door_00`, `SGC_Door03`,
//! `EM-Door_Prison00` and `CA-CastleEntrance_Door00` all hit `door`.

/// Why a mesh name alone rules an actor out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameRule {
    Door,
    StargatePart,
    CameraHead,
}

impl NameRule {
    /// Stable identifier for the decision log.
    pub fn label(self) -> &'static str {
        match self {
            NameRule::Door => "name:door",
            NameRule::StargatePart => "name:stargate-part",
            NameRule::CameraHead => "name:camera-head",
        }
    }
}

/// Lower-case substrings and the rule each one triggers, checked in
/// order.
pub const NAME_RULES: &[(&str, NameRule)] = &[
    ("door", NameRule::Door),
    ("stargate", NameRule::StargatePart),
    ("chevron", NameRule::StargatePart),
    ("securitycam", NameRule::CameraHead),
];

/// The first [`NAME_RULES`] entry `mesh_name` matches.
pub fn name_rule(mesh_name: &str) -> Option<NameRule> {
    let lower = mesh_name.to_ascii_lowercase();
    NAME_RULES
        .iter()
        .find(|(needle, _)| lower.contains(needle))
        .map(|(_, rule)| *rule)
}
