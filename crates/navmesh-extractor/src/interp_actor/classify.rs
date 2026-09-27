//! The per-actor decision: bake this `InterpActor` into the `.nav` /
//! `.occ`, leave it out, or leave it out and flag it.
//!
//! Rules, first match wins:
//!
//! 1. The mesh name hits a [`super::name_rules`] safety net (door,
//!    Stargate part, camera head) → **exclude**, whatever else is true.
//! 2. No Kismet in the chunk references the actor → **include**.
//!    Nothing in its level can move it, so its cooked pose is its only
//!    pose.
//! 3. A Kismet reference the classifier does not understand → an op
//!    other than a Matinee player or one of [`BENIGN_KISMET_OPS`], or a
//!    direct object property other than an event's `Originator` →
//!    **undecided**. An actor Kismet names only in those benign ways →
//!    **include**.
//! 4. A Matinee group whose tracks could not be read (including a move
//!    track with no position keys), or that carries a
//!    track that can change more than the actor's appearance
//!    ([`BENIGN_TRACKS`]) → **undecided**.
//! 5. Any move track that starts or ends away from the cooked pose →
//!    **exclude**: the actor spends time somewhere else (a door that
//!    opens and stays open).
//! 6. Any move track that rotates more than [`MAX_ROTATION_DEG`] →
//!    **exclude** (camera sweeps, radar dishes, fan rotors).
//! 7. Any move track that slides more than [`MAX_HORIZONTAL_CM`]
//!    sideways → **exclude**: a round trip that far sideways is a
//!    sliding panel, and its cooked pose closes whatever it slides off.
//! 8. Otherwise → **include**. Every track leaves from and returns to
//!    the cooked pose and only bobs or rises: ring-transport rings
//!    (up to 3.3 m for a few seconds per activation), floating lamps
//!    (17 cm) and idling vehicles (8 cm, 1°). The cooked pose is where
//!    the actor rests.
//!
//! Undecided actors are not baked, and the coverage report counts them
//! as an `InterpActor` collision risk.

use super::kismet_evidence::ActorMotion;
use super::name_rules::{name_rule, NameRule};

/// A key within this many cm of the cooked pose is "at rest".
pub const REST_TOLERANCE_CM: f32 = 1.0;
/// Rotation beyond this many degrees on any axis rules an actor out.
pub const MAX_ROTATION_DEG: f32 = 5.0;
/// Sideways travel beyond this many cm rules an actor out.
pub const MAX_HORIZONTAL_CM: f32 = 50.0;

/// Kismet ops that may name an actor without being able to move it or
/// change its collision. `SeqAct_PlaySound` plays its cue at the
/// target. In the 2009 client it is the only non-Matinee op that names
/// an `InterpActor` at all: 244 of them, every one also Matinee-driven
/// (street lamps and floating lights in Lucia and Tollana, Humvees in
/// Agnos and Lucia, radar dishes in Beta_Site_Evo_1, Castle and Lucia).
pub const BENIGN_KISMET_OPS: &[&str] = &["SeqAct_PlaySound"];

/// Matinee track classes that change how an actor looks or sounds but
/// not where its collision is.
pub const BENIGN_TRACKS: &[&str] = &[
    "InterpTrackEvent",
    "InterpTrackSound",
    "InterpTrackFloatMaterialParam",
    "InterpTrackVectorMaterialParam",
    "InterpTrackColorProp",
];

/// Why an actor is baked.
#[derive(Debug, Clone, PartialEq)]
pub enum IncludeRule {
    /// No Kismet references it.
    Unreferenced,
    /// Kismet names it only in ways that cannot move it: an event's
    /// `Originator`, or the target of a [`BENIGN_KISMET_OPS`] op.
    BenignReferencesOnly,
    /// Every Matinee move leaves from and returns to the cooked pose,
    /// without turning or sliding. `max_vertical_cm` is the largest
    /// rise or dip.
    RestAnchoredMatinee { max_vertical_cm: f32 },
}

/// Why an actor is left out on evidence.
#[derive(Debug, Clone, PartialEq)]
pub enum ExcludeRule {
    Name(NameRule),
    /// A move track starts or ends this far from the cooked pose.
    MatineeLeavesRest {
        offset_cm: f32,
    },
    /// A move track rotates this far.
    MatineeRotates {
        degrees: f32,
    },
    /// A move track slides this far sideways.
    MatineeSlides {
        horizontal_cm: f32,
    },
}

/// Why an actor is left out without evidence either way.
#[derive(Debug, Clone, PartialEq)]
pub enum UndecidedReason {
    /// A Kismet reference other than a Matinee group or an event
    /// originator, as `Class[LinkDesc]` or `Class.Property`.
    KismetReference(String),
    /// A Matinee group whose `InterpData` or group could not be read.
    MatineeGroupUnreadable(String),
    /// A track class outside [`BENIGN_TRACKS`] and `InterpTrackMove`.
    UnknownTrack(String),
}

/// The classifier's verdict for one actor.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    Include(IncludeRule),
    Exclude(ExcludeRule),
    Undecided(UndecidedReason),
}

impl Decision {
    /// Is the actor's geometry baked?
    pub fn is_included(&self) -> bool {
        matches!(self, Decision::Include(_))
    }

    /// `include` / `exclude` / `undecided`.
    pub fn verdict(&self) -> &'static str {
        match self {
            Decision::Include(_) => "include",
            Decision::Exclude(_) => "exclude",
            Decision::Undecided(_) => "undecided",
        }
    }

    /// The rule that fired, with its measurement, for the decision log.
    pub fn rule(&self) -> String {
        match self {
            Decision::Include(IncludeRule::Unreferenced) => "kismet:unreferenced".into(),
            Decision::Include(IncludeRule::BenignReferencesOnly) => {
                "kismet:benign-refs-only".into()
            }
            Decision::Include(IncludeRule::RestAnchoredMatinee { max_vertical_cm }) => {
                format!("matinee:rest-anchored rise={max_vertical_cm:.0}cm")
            }
            Decision::Exclude(ExcludeRule::Name(rule)) => rule.label().into(),
            Decision::Exclude(ExcludeRule::MatineeLeavesRest { offset_cm }) => {
                format!("matinee:leaves-rest offset={offset_cm:.0}cm")
            }
            Decision::Exclude(ExcludeRule::MatineeRotates { degrees }) => {
                format!("matinee:rotates {degrees:.0}deg")
            }
            Decision::Exclude(ExcludeRule::MatineeSlides { horizontal_cm }) => {
                format!("matinee:slides {horizontal_cm:.0}cm")
            }
            Decision::Undecided(UndecidedReason::KismetReference(r)) => format!("kismet:ref {r}"),
            Decision::Undecided(UndecidedReason::MatineeGroupUnreadable(g)) => {
                format!("matinee:unreadable group={g}")
            }
            Decision::Undecided(UndecidedReason::UnknownTrack(t)) => format!("matinee:track {t}"),
        }
    }
}

/// Classify one `InterpActor`.
///
/// `mesh_name` is the resolved `StaticMesh` object name;
/// `cooked_location` is the actor's `Location` in UE3 cm, which a
/// world-frame move track is measured against.
pub fn classify(motion: &ActorMotion, mesh_name: &str, cooked_location: [f32; 3]) -> Decision {
    if let Some(rule) = name_rule(mesh_name) {
        return Decision::Exclude(ExcludeRule::Name(rule));
    }
    if motion.is_unreferenced() {
        return Decision::Include(IncludeRule::Unreferenced);
    }
    if let Some(r) = motion.other_refs.iter().find(|r| !is_benign_reference(r)) {
        return Decision::Undecided(UndecidedReason::KismetReference(r.clone()));
    }

    if motion.matinee.is_empty() {
        return Decision::Include(IncludeRule::BenignReferencesOnly);
    }

    let mut summaries = Vec::new();
    for group in &motion.matinee {
        let Some(tracks) = &group.tracks else {
            return Decision::Undecided(UndecidedReason::MatineeGroupUnreadable(format!(
                "{}#{}",
                group.group, group.seq_act
            )));
        };
        if let Some(t) = tracks
            .other
            .iter()
            .find(|t| !BENIGN_TRACKS.contains(&t.as_str()))
        {
            return Decision::Undecided(UndecidedReason::UnknownTrack(t.clone()));
        }
        // A move track with no position keys is what a curve the
        // parser could not read looks like. Summarising it would report
        // "never leaves rest", the one answer a bad read must not give.
        if tracks.moves.iter().any(|m| m.positions.is_empty()) {
            return Decision::Undecided(UndecidedReason::MatineeGroupUnreadable(format!(
                "{}#{} (move track without keys)",
                group.group, group.seq_act
            )));
        }
        summaries.extend(tracks.moves.iter().map(|m| m.summarize(cooked_location)));
    }

    let worst =
        |f: fn(&super::MoveTrackSummary) -> f32| summaries.iter().map(f).fold(0.0f32, f32::max);
    let leaves_rest = worst(|s| s.first_offset_cm.max(s.last_offset_cm));
    if leaves_rest > REST_TOLERANCE_CM {
        return Decision::Exclude(ExcludeRule::MatineeLeavesRest {
            offset_cm: leaves_rest,
        });
    }
    let degrees = worst(|s| s.max_rotation_deg);
    if degrees > MAX_ROTATION_DEG {
        return Decision::Exclude(ExcludeRule::MatineeRotates { degrees });
    }
    let horizontal_cm = worst(|s| s.max_horizontal_cm);
    if horizontal_cm > MAX_HORIZONTAL_CM {
        return Decision::Exclude(ExcludeRule::MatineeSlides { horizontal_cm });
    }
    Decision::Include(IncludeRule::RestAnchoredMatinee {
        max_vertical_cm: worst(|s| s.max_vertical_cm),
    })
}

/// `SeqEvent_*.Originator` (the actor is where an event fires from) or
/// a variable link from a [`BENIGN_KISMET_OPS`] op: neither says
/// anything about the actor moving.
fn is_benign_reference(reference: &str) -> bool {
    (reference.starts_with("SeqEvent_") && reference.ends_with(".Originator"))
        || BENIGN_KISMET_OPS
            .iter()
            .any(|op| reference.starts_with(&format!("{op}[")))
}
