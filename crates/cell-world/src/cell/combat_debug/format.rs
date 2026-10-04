//! The one formatter: a [`CastDebug`] record to its debug lines.
//!
//! The same text goes to the `abilities.debug` row and to the client, so the
//! in-game trace and SigNoz never disagree. Every line starts with
//! `[CD #<cast_id>]` (`[CD]` for a note made outside any cast), the join key
//! to the cast's AB-T rows.
//!
//! - **Simple lines**, one per outcome: each hit (its QR roll and result and
//!   the target's pools), each landing of a beneficial or routed effect,
//!   each pulse; or one `fired, nothing resolved` line when the cast reached
//!   no one.
//! - **Verbose lines**, added for a verbose watcher: every `effect_planned`
//!   path and reason, every NVP entry with its pools, every ledger entry and
//!   each pulse's path.
//!
//! A line longer than the chat cap ([`MAX_LINE_UNITS`], the D-SS12 255
//! UTF-16 units) is split by [`split_line`].

use cimmeria_entity::organization::limits::MAX_CHAT_TEXT_UNITS;

use super::record::{CastDebug, Note, Pools};
use crate::cell::space_manager::SpaceManager;

/// The longest line sent: the server's chat-text cap (D-SS12). The client's
/// chat window has no `MaxTextLength` of its own (chat-wire-formats C-Q3),
/// so this is the server's rule, kept for the debug lines too.
pub const MAX_LINE_UNITS: usize = MAX_CHAT_TEXT_UNITS;

/// What a continuation line of a split starts with.
const CONTINUATION: &str = "  ... ";

/// A record's lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lines {
    pub simple: Vec<String>,
    pub verbose: Vec<String>,
}

/// `Name(id)` for a named entity, `entity <id>` otherwise.
pub fn entity_label(mgr: &SpaceManager, entity_id: u32) -> String {
    let name = mgr.get_entity(entity_id).and_then(|e| {
        e.character_name
            .as_deref()
            .or(e.npc_name.as_deref())
            .filter(|n| !n.is_empty())
    });
    match name {
        Some(n) => format!("{n}({entity_id})"),
        None => format!("entity {entity_id}"),
    }
}

/// `HP 100->77 (-23), FP 50->50 (+0)`.
pub fn pools_text(before: Pools, after: Pools) -> String {
    format!(
        "HP {}->{} ({:+}), FP {}->{} ({:+})",
        before.health,
        after.health,
        after.health - before.health,
        before.focus,
        after.focus,
        after.focus - before.focus
    )
}

fn prefix(rec: &CastDebug) -> String {
    match rec.cast_id {
        Some(id) => format!("[CD #{id}]"),
        None => "[CD]".to_string(),
    }
}

fn effect_text(effect_id: Option<i32>) -> String {
    effect_id.map_or_else(|| "eff -".to_string(), |e| format!("eff {e}"))
}

/// Build the record's lines (module docs). The labels are read now, at the
/// end of the cast, so an entity that left mid-cast shows as `entity <id>`.
pub fn format_record(mgr: &SpaceManager, rec: &CastDebug) -> Lines {
    let p = prefix(rec);
    let ability = super::commands::ability_label(mgr, rec.ability_id);
    let caster = entity_label(mgr, rec.caster_id);
    let label = |id: u32| entity_label(mgr, id);
    let mut out = Lines::default();
    let mut fire_target: Option<Option<u32>> = None;
    for note in &rec.notes {
        match note {
            Note::Fire { target, .. } => fire_target = Some(*target),
            Note::Hit(h) => {
                let result = if h.dont_use_qr {
                    format!("{} (no roll)", h.result)
                } else {
                    format!("{}, roll {:.3} qr {:.3}", h.result, h.roll, h.qr)
                };
                out.simple.push(format!(
                    "{p} {ability} {caster} -> {}: {result}; {}",
                    label(h.target_id),
                    pools_text(h.before, h.after)
                ));
            }
            Note::Landing(l) => out.simple.push(format!(
                "{p} {ability} {caster} -> {}: landed; {}",
                label(l.recipient),
                pools_text(l.before, l.after)
            )),
            Note::Pulse(pl) => {
                out.simple.push(format!(
                    "{p} {ability} pulse eff {} {caster} -> {}: {}; {} left",
                    pl.effect_id,
                    label(pl.target_id),
                    pools_text(pl.before, pl.after),
                    pl.remaining
                ));
                out.verbose.push(format!(
                    "{p}  pulse eff {} -> {}: path {}",
                    pl.effect_id,
                    label(pl.target_id),
                    pl.path
                ));
            }
            Note::Plan(pl) => out.verbose.push(format!(
                "{p}  plan {} -> {}: {} ({})",
                effect_text(pl.effect_id),
                label(pl.target_id),
                pl.path,
                pl.reason
            )),
            Note::Nvp(v) => out.verbose.push(format!(
                "{p}  nvp {} -> {}: base H{} F{}, dealt H{} F{}, absorbed {} ({}); {}",
                effect_text(v.effect_id),
                label(v.target_id),
                v.health_base,
                v.focus_base,
                v.health_dealt,
                v.focus_dealt,
                v.absorbed,
                v.reason,
                pools_text(v.before, v.after)
            )),
            Note::Ledger(l) => {
                let how_long = if l.held {
                    "held".to_string()
                } else {
                    format!("{:.1} s", l.duration_secs)
                };
                out.verbose.push(format!(
                    "{p}  ledger eff {} -> {}: {}, {how_long}",
                    l.effect_id,
                    label(l.target_id),
                    l.outcome
                ));
            }
        }
    }
    if out.simple.is_empty() {
        let to = match fire_target {
            Some(Some(t)) => label(t),
            _ => "no target".to_string(),
        };
        out.simple.push(format!(
            "{p} {ability} {caster} -> {to}: fired, nothing resolved"
        ));
    }
    if rec.dropped_notes > 0 {
        out.verbose.push(format!(
            "{p}  +{} notes not kept (record full)",
            rec.dropped_notes
        ));
    }
    out
}

/// Split `text` into chat lines of at most `max_units` UTF-16 units, at a
/// space when there is one in the back half of the line. Continuation lines
/// start with `  ... `.
pub fn split_line(text: &str, max_units: usize) -> Vec<String> {
    let units = |s: &str| s.encode_utf16().count();
    if units(text) <= max_units {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut rest = text;
    let mut first = true;
    while !rest.is_empty() {
        let lead = if first { "" } else { CONTINUATION };
        let budget = max_units.saturating_sub(units(lead)).max(1);
        if units(rest) <= budget {
            out.push(format!("{lead}{rest}"));
            break;
        }
        // The byte index where `budget` units end, on a char boundary.
        let mut used = 0;
        let mut cut = 0;
        for (i, c) in rest.char_indices() {
            if used + c.len_utf16() > budget {
                break;
            }
            used += c.len_utf16();
            cut = i + c.len_utf8();
        }
        // Prefer a space in the back half, so words stay whole.
        let at = rest[..cut]
            .rfind(' ')
            .filter(|&s| s >= cut / 2 && s > 0)
            .unwrap_or(cut);
        out.push(format!("{lead}{}", rest[..at].trim_end()));
        rest = rest[at..].trim_start();
        first = false;
    }
    out
}
