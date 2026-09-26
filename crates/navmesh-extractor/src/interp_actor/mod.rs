//! NA40 — which `InterpActor`s are baked into a `.nav` / `.occ`.
//!
//! `InterpActor` is UE3's Matinee-driven mover. NA36 made the walker
//! able to read it but kept it all-or-nothing behind a flag, because a
//! mover's cooked pose is its design-time pose: bake a door and the
//! doorway is sealed. This module replaces the flag with a decision per
//! actor, grounded in the chunk's own Kismet:
//!
//! - [`kismet_evidence`] follows every `SeqAct_Interp` in the chunk to
//!   the actors its groups drive, and reads their move-track keyframes.
//! - [`name_rules`] is a mesh-name safety net for doors, Stargate parts
//!   and camera heads, which are never baked.
//! - [`classify`] turns the two into an include / exclude / undecided
//!   [`Decision`] with the rule that fired.
//!
//! [`InterpActorMode::Classify`] is the default. [`InterpActorMode::Off`]
//! reproduces a pre-NA36 extraction (no `InterpActor` at all), which is
//! what every map not rebuilt since was built with.
//!
//! Evidence and numbers: `docs/engine/navmesh-build-pipeline.md` §12 and
//! `docs/analysis/npc-ai-restoration/worknotes/na40-static-interp-actors.md`.

pub mod classify;
pub mod kismet_evidence;
pub mod move_track;
pub mod name_rules;

use std::io::Write;

pub use classify::{classify, Decision, ExcludeRule, IncludeRule, UndecidedReason};
pub use kismet_evidence::{ActorMotion, GroupTracks, MatineeGroup, MotionEvidence};
pub use move_track::{MoveFrame, MoveTrack, MoveTrackSummary};
pub use name_rules::{name_rule, NameRule};

/// How the walker treats `InterpActor` exports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InterpActorMode {
    /// Not walked at all: the pre-NA36 extraction.
    Off,
    /// Walked, and each one baked only if [`classify`] includes it.
    #[default]
    Classify,
}

impl InterpActorMode {
    /// The CLI spelling.
    pub fn label(self) -> &'static str {
        match self {
            InterpActorMode::Off => "off",
            InterpActorMode::Classify => "classify",
        }
    }

    /// Parse the CLI spelling. Anything else is an error, not a default:
    /// these tools produce measurements.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "off" => Ok(InterpActorMode::Off),
            "classify" => Ok(InterpActorMode::Classify),
            other => Err(format!(
                "interp-actors wants `off` or `classify`, got {other:?}"
            )),
        }
    }
}

/// One classified actor, for the decision log.
#[derive(Debug, Clone, PartialEq)]
pub struct InterpActorRecord {
    /// Chunk file stem; filled in by the map-level caller.
    pub chunk: String,
    /// Export object name plus instance number, e.g. `InterpActor_12`.
    pub actor: String,
    /// 0-based export index in the chunk.
    pub export_index: usize,
    /// Resolved `StaticMesh` object name.
    pub mesh: String,
    /// Cooked `Location`, UE3 cm.
    pub location: [f32; 3],
    pub decision: Decision,
    /// The Kismet evidence in one line: each Matinee group as
    /// `group#seq_act`, then each other reference.
    pub evidence: String,
}

impl InterpActorRecord {
    /// Header of [`write_decision_log`]'s TSV.
    pub const TSV_HEADER: &'static str =
        "chunk\tactor\texport\tmesh\tue3_x\tue3_y\tue3_z\tbw_x\tbw_y\tbw_z\tverdict\trule\tevidence";

    fn tsv_row(&self) -> String {
        let [x, y, z] = self.location;
        // CA05 axis mapping: BigWorld (x, y, z) = UE3 (Y, Z, X) / 100.
        format!(
            "{}\t{}\t{}\t{}\t{x:.0}\t{y:.0}\t{z:.0}\t{:.2}\t{:.2}\t{:.2}\t{}\t{}\t{}",
            self.chunk,
            self.actor,
            self.export_index + 1,
            self.mesh,
            y / 100.0,
            z / 100.0,
            x / 100.0,
            self.decision.verdict(),
            self.decision.rule(),
            self.evidence,
        )
    }
}

/// Summarise an actor's evidence for the decision log.
pub fn evidence_line(motion: &ActorMotion) -> String {
    let mut parts: Vec<String> = motion
        .matinee
        .iter()
        .map(|g| {
            let tracks = match &g.tracks {
                None => "unreadable".to_string(),
                Some(t) => {
                    let mut names: Vec<String> = t
                        .moves
                        .iter()
                        .map(|m| format!("Move({} keys)", m.positions.len()))
                        .collect();
                    names.extend(t.other.iter().cloned());
                    names.join("+")
                }
            };
            format!("matinee {}#{} [{tracks}]", g.group, g.seq_act)
        })
        .collect();
    parts.extend(motion.other_refs.iter().map(|r| format!("ref {r}")));
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("; ")
    }
}

/// Write the decision log: one row per classified actor, sorted by
/// chunk and export so two runs diff cleanly.
pub fn write_decision_log<W: Write>(
    w: &mut W,
    records: &[InterpActorRecord],
) -> std::io::Result<()> {
    let mut sorted: Vec<&InterpActorRecord> = records.iter().collect();
    sorted.sort_by(|a, b| {
        a.chunk
            .cmp(&b.chunk)
            .then(a.export_index.cmp(&b.export_index))
    });
    writeln!(w, "{}", InterpActorRecord::TSV_HEADER)?;
    for r in sorted {
        writeln!(w, "{}", r.tsv_row())?;
    }
    Ok(())
}

/// Included / excluded / undecided counts over `records`.
pub fn tally(records: &[InterpActorRecord]) -> (usize, usize, usize) {
    records
        .iter()
        .fold((0, 0, 0), |(i, e, u), r| match r.decision {
            Decision::Include(_) => (i + 1, e, u),
            Decision::Exclude(_) => (i, e + 1, u),
            Decision::Undecided(_) => (i, e, u + 1),
        })
}

#[cfg(test)]
mod classify_tests;
#[cfg(test)]
mod evidence_tests;
