//! `.effects [target]` — the AB-T5 ability state as chat lines. Read-only.
//!
//! The same snapshot the `abilities.snapshot` row and the lab's
//! `server_ability_state` return, cut down to what a GM can read in the chat
//! window: one header line, then one line each for cooldowns, pulsing
//! effects, ledger entries and held state flags. A list longer than
//! [`MAX_PER_LINE`] ends with `+N more`; the lab tool has the full set.

use cimmeria_entity::cell_entity::{AbilityStateSnapshot, LedgerEntryState};
use cimmeria_entity::stats::{FOCUS, HEALTH};
use tokio::sync::mpsc;

use super::display_name;
use crate::cell::console::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Entries shown per line before `+N more`.
pub(crate) const MAX_PER_LINE: usize = 6;

pub(super) async fn show(
    caller_id: u32,
    subject: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let Some(snapshot) = space_mgr.ability_state(subject) else {
        send_gm_feedback(
            caller_id,
            &format!(".effects: entity {subject} is gone"),
            tx,
        )
        .await;
        return;
    };
    for line in format_lines(&display_name(space_mgr, subject), &snapshot) {
        send_gm_feedback(caller_id, &line, tx).await;
    }
}

/// The chat lines for one snapshot. Pure, so the format is testable.
pub(crate) fn format_lines(name: &str, s: &AbilityStateSnapshot) -> Vec<String> {
    let pool = |id: i32| {
        s.stat(id)
            .map_or_else(|| "-".to_string(), |p| format!("{}/{}", p.cur, p.max))
    };
    let warmup = s.pending_cast.as_ref().map_or_else(
        || "none".to_string(),
        |p| {
            format!(
                "ability {} cast {} {:.1}s left",
                p.ability_id, p.cast_id, p.warmup_remaining_secs
            )
        },
    );
    let mut lines = vec![format!(
        "effects [{}] {}: Health {}, Focus {}, state 0x{:x}, warmup {}",
        s.entity_id,
        name,
        pool(HEALTH),
        pool(FOCUS),
        s.state_field,
        warmup
    )];
    lines.push(list(
        "cooldowns",
        s.cooldowns
            .iter()
            .map(|c| format!("{} {:.1}s", c.ability_id, c.remaining_secs))
            .collect(),
    ));
    lines.push(list(
        "pulsing",
        s.pulsing
            .iter()
            .map(|p| {
                format!(
                    "{} (ability {}) from {}, {}/{} pulses left{}",
                    p.effect_id,
                    p.ability_id,
                    p.invoker_id,
                    p.pulses_left,
                    p.total_pulses,
                    cast(p.cast_id)
                )
            })
            .collect(),
    ));
    lines.push(list("ledger", s.ledger.iter().map(ledger_entry).collect()));
    if !s.state_flag_refcounts.is_empty() {
        lines.push(list(
            "state refs",
            s.state_flag_refcounts
                .iter()
                .map(|r| format!("bit {} x{}", r.bit, r.count))
                .collect(),
        ));
    }
    lines
}

fn ledger_entry(l: &LedgerEntryState) -> String {
    let when = match l.expires_in_secs {
        Some(secs) => format!("{secs:.1}s"),
        None => "held".to_string(),
    };
    let mut parts = vec![format!(
        "{} (ability {}) from {}, {}",
        l.effect_id, l.ability_id, l.invoker_id, when
    )];
    for d in &l.stats {
        parts.push(format!("stat {} {:+}", d.stat_id, d.requested));
    }
    for p in &l.absorb {
        parts.push(format!(
            "absorb {} {}/{}",
            p.stat_id, p.remaining, p.granted
        ));
    }
    if l.state_flags != 0 {
        parts.push(format!("flags 0x{:x}", l.state_flags));
    }
    format!("{}{}", parts.join(", "), cast(l.cast_id))
}

fn cast(cast_id: Option<i32>) -> String {
    cast_id.map_or_else(String::new, |c| format!(" [cast {c}]"))
}

/// `label: a; b; c` with at most [`MAX_PER_LINE`] items, or `label: none`.
fn list(label: &str, items: Vec<String>) -> String {
    if items.is_empty() {
        return format!("{label}: none");
    }
    let extra = items.len().saturating_sub(MAX_PER_LINE);
    let shown: Vec<String> = items.into_iter().take(MAX_PER_LINE).collect();
    if extra > 0 {
        format!("{label}: {}; +{extra} more", shown.join("; "))
    } else {
        format!("{label}: {}", shown.join("; "))
    }
}
