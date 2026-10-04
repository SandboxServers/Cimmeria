//! The notes a cast leaves while it resolves, and the record that holds
//! them until its scope closes.
//!
//! Each note mirrors one AB-T3 row and carries the values that row logs:
//!
//! | Note | Row |
//! |---|---|
//! | [`Note::Fire`] | `ability_launched` (the fire half: the resolved target) |
//! | [`Note::Hit`] | `qr_rolled`, plus the target's pools around the hit |
//! | [`Note::Plan`] | `effect_planned` |
//! | [`Note::Nvp`] | `nvp_damage_resolved` |
//! | [`Note::Landing`] | the pools around `land_effects` for one recipient |
//! | [`Note::Ledger`] | `stat_buff_applied` |
//! | [`Note::Pulse`] | `pulse_ticked` |

use super::MAX_NOTES_PER_RECORD;

/// A target's HEALTH and FOCUS at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pools {
    pub health: i32,
    pub focus: i32,
}

/// One hit's roll and what it did to the target's pools.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitNote {
    pub target_id: u32,
    /// The hit's QR (`qr_rolled.qr`).
    pub qr: f64,
    /// The beta sample (`qr_rolled.roll`).
    pub roll: f64,
    pub result_code: u8,
    /// `qr_rolled.result`: `hit`, `miss`, `critical`...
    pub result: &'static str,
    /// No roll was taken (`EF_DontUseQR` on every effect).
    pub dont_use_qr: bool,
    pub before: Pools,
    /// After the NVP damage, the damage scripts, a god-mode restore and a
    /// duel clamp: what the target kept.
    pub after: Pools,
}

/// One `effect_planned` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanNote {
    pub target_id: u32,
    /// `None` for an unknown ability's generic swing.
    pub effect_id: Option<i32>,
    pub path: &'static str,
    pub reason: &'static str,
}

/// One `nvp_damage_resolved` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NvpNote {
    pub target_id: u32,
    pub effect_id: Option<i32>,
    pub health_base: i32,
    pub focus_base: i32,
    pub health_dealt: i32,
    pub focus_dealt: i32,
    pub absorbed: i32,
    pub before: Pools,
    pub after: Pools,
    /// `hit_roll` or `dont_use_qr`.
    pub reason: &'static str,
}

/// What `land_effects` did to one recipient's pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandingNote {
    pub recipient: u32,
    pub before: Pools,
    pub after: Pools,
}

/// One `stat_buff_applied` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LedgerNote {
    pub target_id: u32,
    pub effect_id: i32,
    /// `applied` or `replaced`.
    pub outcome: &'static str,
    pub duration_secs: f32,
    /// Held until removed (no expiry).
    pub held: bool,
}

/// One `pulse_ticked` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseNote {
    pub target_id: u32,
    pub effect_id: i32,
    pub path: &'static str,
    pub before: Pools,
    pub after: Pools,
    /// `remaining_before_decrement`.
    pub remaining: i32,
}

/// One decision of a cast.
#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    /// The cast fired at `target` (`None`: no entity target).
    Fire {
        target: Option<u32>,
        beneficial: bool,
    },
    Hit(HitNote),
    Plan(PlanNote),
    Nvp(NvpNote),
    Landing(LandingNote),
    Ledger(LedgerNote),
    Pulse(PulseNote),
}

/// Which toggle a record answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    /// A hostile cast: combat debug.
    Hostile,
    /// A player's beneficial cast (a heal, a buff): heal debug.
    Beneficial,
    /// A pulse with no fire in the same record (a DoT or HoT tick later
    /// on): combat or heal debug.
    Pulse,
}

/// Everything one cast noted, from its fire to the end of its scope.
#[derive(Debug, Clone, PartialEq)]
pub struct CastDebug {
    pub caster_id: u32,
    pub cast_id: Option<i32>,
    pub ability_id: i32,
    pub notes: Vec<Note>,
    /// Notes past [`MAX_NOTES_PER_RECORD`], counted, not kept.
    pub dropped_notes: u32,
}

impl CastDebug {
    pub fn new(caster_id: u32, cast_id: Option<i32>, ability_id: i32) -> Self {
        Self {
            caster_id,
            cast_id,
            ability_id,
            notes: Vec::new(),
            dropped_notes: 0,
        }
    }

    pub(crate) fn push(&mut self, note: Note) {
        if self.notes.len() >= MAX_NOTES_PER_RECORD {
            self.dropped_notes += 1;
        } else {
            self.notes.push(note);
        }
    }

    /// The toggle this record answers to.
    pub fn kind(&self) -> CastKind {
        for n in &self.notes {
            if let Note::Fire { beneficial, .. } = n {
                return if *beneficial {
                    CastKind::Beneficial
                } else {
                    CastKind::Hostile
                };
            }
        }
        if self.notes.iter().any(|n| matches!(n, Note::Pulse(_))) {
            CastKind::Pulse
        } else {
            CastKind::Hostile
        }
    }

    /// Every entity the cast touched (fire target, hit targets, landing
    /// recipients, ledger and pulse targets), caster excluded, no repeats.
    pub fn touched(&self) -> Vec<u32> {
        let mut out = Vec::new();
        let mut add = |id: u32| {
            if id != self.caster_id && !out.contains(&id) {
                out.push(id);
            }
        };
        for n in &self.notes {
            match n {
                Note::Fire { target, .. } => {
                    if let Some(t) = target {
                        add(*t);
                    }
                }
                Note::Hit(h) => add(h.target_id),
                Note::Plan(p) => add(p.target_id),
                Note::Nvp(v) => add(v.target_id),
                Note::Landing(l) => add(l.recipient),
                Note::Ledger(l) => add(l.target_id),
                Note::Pulse(p) => add(p.target_id),
            }
        }
        out
    }

    /// Whether `entity_id` cast it or was touched by it.
    pub fn involves(&self, entity_id: u32) -> bool {
        self.caster_id == entity_id || self.touched().contains(&entity_id)
    }
}
