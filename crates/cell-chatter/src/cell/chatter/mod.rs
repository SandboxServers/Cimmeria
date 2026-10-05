//! The ambient chatter tick.
//!
//! The cell's startup puts an `AmbientChatterCatalog` in
//! `SpaceManager::resources`; this tick keeps a [`ChatterRuntime`] beside it,
//! one [`GroupRun`] per group. Each tick returns at once until the earliest
//! group has something due, so the 100 ms loop costs one comparison between
//! lines. When a group is due it either starts its next exchange (only with a
//! player in earshot of one of its speakers) or speaks its due line to the
//! players in earshot of that line's speaker.
//!
//! Logs go to target `chatter`: INFO when an exchange starts (who hears it),
//! DEBUG per line, and a WARN once per group and tag when a line's speaker is
//! missing or nameless, the seed fault that makes a scene skip a line.

pub mod schedule;
pub mod speak;

use std::collections::HashSet;
use std::time::Instant;

use cimmeria_cell_catalog::cell::spawner::{AmbientChatterCatalog, ChatterExchange, ChatterGroup};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use schedule::{Due, GroupRun};

/// The tick's state, kept in `SpaceManager::resources`.
#[derive(Debug, Default)]
pub struct ChatterRuntime {
    /// One per catalog group, in catalog order.
    runs: Vec<GroupState>,
    /// When the earliest group next has something due; `None` when no group
    /// can ever speak.
    next_due: Option<Instant>,
    /// `(group_id, speaker_tag, event)` already warned about, so a missing
    /// speaker logs once, not once per scene.
    warned: HashSet<(i32, String, &'static str)>,
}

#[derive(Debug)]
struct GroupState {
    run: GroupRun,
    /// The group's world, as `spaces.xml` names it; `None` when this cell
    /// does not load it (the group is then idle for the process lifetime).
    world_name: Option<String>,
}

impl ChatterRuntime {
    fn new(catalog: &AmbientChatterCatalog, space_mgr: &SpaceManager, now: Instant) -> Self {
        let runs: Vec<GroupState> = catalog
            .groups
            .iter()
            .map(|g| {
                let world_name = space_mgr
                    .worlds
                    .values()
                    .find(|w| w.world_id == Some(g.world_id))
                    .map(|w| w.world_name.clone())
                    .filter(|name| space_mgr.space_id_for_world(name).is_some());
                if world_name.is_none() {
                    tracing::warn!(
                        target: "chatter",
                        event = "chatter.group_world_missing",
                        group_id = g.group_id,
                        group_name = %g.name,
                        world_id = g.world_id, // nt:id-only the world has no space here, so no name to give
                        reason = "world_not_loaded",
                        "ambient chatter group's world has no shared space on this cell -- \
                         the group stays silent"
                    );
                }
                GroupState {
                    run: GroupRun::new(now),
                    world_name,
                }
            })
            .collect();
        let mut rt = Self {
            runs,
            next_due: None,
            warned: HashSet::new(),
        };
        rt.recompute_next_due();
        tracing::info!(
            target: "chatter",
            event = "chatter.ready",
            groups = rt.runs.len(),
            live_groups = rt.runs.iter().filter(|s| s.world_name.is_some()).count(),
            "ambient chatter ready"
        );
        rt
    }

    fn recompute_next_due(&mut self) {
        self.next_due = self
            .runs
            .iter()
            .filter(|s| s.world_name.is_some())
            .map(|s| s.run.due_at())
            .min();
    }
}

/// Run the chatter tick now.
pub async fn run(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    run_at(tx, space_mgr, Instant::now()).await;
}

/// Run the chatter tick as of `now`.
pub async fn run_at(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager, now: Instant) {
    // The fast path: nothing due yet.
    if let Some(rt) = space_mgr.resources.get::<ChatterRuntime>() {
        match rt.next_due {
            Some(due) if due <= now => {}
            _ => return,
        }
    }
    let Some(catalog) = space_mgr.resources.remove::<AmbientChatterCatalog>() else {
        // No catalog loaded (no DB, or the load failed and said so): an
        // empty runtime keeps the fast path above.
        space_mgr.resources.insert(ChatterRuntime::default());
        return;
    };
    let mut rt = space_mgr
        .resources
        .remove::<ChatterRuntime>()
        .unwrap_or_else(|| ChatterRuntime::new(&catalog, space_mgr, now));

    let mut out: Vec<CellToBaseMsg> = Vec::new();
    for (state, group) in rt.runs.iter_mut().zip(&catalog.groups) {
        let Some(world) = state.world_name.as_deref() else {
            continue;
        };
        let Some(space) = space_mgr
            .space_id_for_world(world)
            .and_then(|sid| space_mgr.spaces.get(&sid))
        else {
            continue;
        };
        // Bounded: at most one exchange starts per group per tick, and each
        // other step moves on to a later deadline or to the next of an
        // exchange's finitely many lines. Without the one-start rule a zero
        // gap with zero-delay lines would loop here for ever (the seed's
        // CHECK keeps the gap at 5 s or more, the loader at 1 s or more).
        let mut started = false;
        while let Some(due) = state.run.due(now) {
            match due {
                Due::Start { exchange } => {
                    if started {
                        break;
                    }
                    started = true;
                    if !speak::anyone_in_earshot(space, &group.speaker_tags(), group.hear_radius) {
                        state.run.no_audience(now);
                        continue;
                    }
                    log_exchange_start(space_mgr, space, group, exchange);
                    state.run.start(now, group);
                }
                Due::Line { exchange, line } => {
                    out.extend(speak_line(
                        space_mgr,
                        space,
                        group,
                        exchange,
                        line,
                        &mut rt.warned,
                    ));
                    state.run.line_done(now, group);
                }
            }
        }
    }
    rt.recompute_next_due();
    space_mgr.resources.insert(catalog);
    space_mgr.resources.insert(rt);

    for msg in out {
        if let Err(e) = tx.send(msg).await {
            tracing::warn!(
                target: "chatter",
                event = "chatter.send_failed",
                reason = "cell_to_base_closed",
                "ambient chatter line not sent -- the cell->base channel is closed: {e}"
            );
            return;
        }
    }
}

fn log_exchange_start(
    space_mgr: &SpaceManager,
    space: &crate::cell::space_manager::SpaceInstance,
    group: &ChatterGroup,
    exchange: usize,
) {
    let mut heard_by: Vec<u32> = group
        .speaker_tags()
        .iter()
        .filter_map(|t| speak::speaker(space, t))
        .flat_map(|npc| speak::listeners(space, &npc.position, group.hear_radius))
        .collect();
    heard_by.sort_unstable();
    heard_by.dedup();
    let names: Vec<&str> = heard_by
        .iter()
        .map(|&id| space_mgr.entity_label(id).unwrap_or("?"))
        .collect();
    let ex = &group.exchanges[exchange % group.exchanges.len()];
    tracing::info!(
        target: "chatter",
        event = "chatter.exchange_started",
        group_id = group.group_id,
        group_name = %group.name,
        world = %space.world_name,
        exchange_id = ex.exchange_id,
        exchange_name = exchange_label(ex),
        lines = ex.lines.len(),
        listener_count = heard_by.len(),
        listener_names = %names.join(","),
        "ambient chatter exchange started"
    );
}

/// The messages for one line, or none when its speaker is missing (warned
/// once per group and tag) or nobody is in earshot.
fn speak_line(
    space_mgr: &SpaceManager,
    space: &crate::cell::space_manager::SpaceInstance,
    group: &ChatterGroup,
    exchange: usize,
    line: usize,
    warned: &mut HashSet<(i32, String, &'static str)>,
) -> Vec<CellToBaseMsg> {
    let ex = &group.exchanges[exchange];
    let Some(l) = ex.lines.get(line) else {
        return Vec::new();
    };
    let Some(npc) = speak::speaker(space, &l.speaker_tag) else {
        if warned.insert((group.group_id, l.speaker_tag.clone(), "speaker_missing")) {
            tracing::warn!(
                target: "chatter",
                event = "chatter.speaker_missing",
                group_id = group.group_id,
                group_name = %group.name,
                world = %space.world_name,
                exchange_id = ex.exchange_id,
                exchange_name = exchange_label(ex),
                line_index = line,
                speaker_tag = %l.speaker_tag,
                reason = "no_living_npc_with_tag",
                "ambient chatter line skipped: no living NPC carries its speaker tag \
                 (a seed tag that names nobody, or a speaker who was killed; warned \
                 once per group and tag)"
            );
        }
        return Vec::new();
    };
    let npc_id = npc.entity_id.0 as u32;
    let Some(name) = space_mgr.entity_label(npc_id) else {
        if warned.insert((group.group_id, l.speaker_tag.clone(), "speaker_unnamed")) {
            tracing::warn!(
                target: "chatter",
                event = "chatter.speaker_unnamed",
                group_id = group.group_id,
                group_name = %group.name,
                exchange_id = ex.exchange_id,
                exchange_name = exchange_label(ex),
                line_index = line,
                speaker_tag = %l.speaker_tag,
                entity_id = npc_id, // nt:id-only the speaker has no name, which is why this warns
                template_id = npc.template_id,
                template_name = space_mgr.entity_names(npc_id).template_name,
                reason = "speaker_has_no_name",
                "ambient chatter line skipped: its speaker has no display name, and a \
                 say line with a blank speaker renders as garbage (warned once per group and tag)"
            );
        }
        return Vec::new();
    };
    let listeners = speak::listeners(space, &npc.position, group.hear_radius);
    tracing::debug!(
        target: "chatter",
        event = "chatter.line",
        group_id = group.group_id,
        group_name = %group.name,
        exchange_id = ex.exchange_id,
        exchange_name = exchange_label(ex),
        line_index = line,
        entity_id = npc_id,
        entity_name = name,
        listener_count = listeners.len(),
        text_len = l.text.len(),
        "ambient chatter line"
    );
    speak::line_messages(name, &l.text, &listeners)
}

/// An exchange's name for a log line: the start of its first line, which is
/// what the seed has instead of a name column (Rule 6, as `npc_bark` names a
/// dialog screen). Cut at 48 characters.
fn exchange_label(ex: &ChatterExchange) -> &str {
    let text = ex.lines.first().map_or("", |l| l.text.trim());
    text.char_indices()
        .nth(48)
        .map_or(text, |(i, _)| &text[..i])
}

#[cfg(test)]
mod tests;
