//! LR-05: one recorded row through the mapping table (contract § Anchors
//! and the mapping table). Every action comes from a table entry; anything
//! else the row saw is a `# TODO` line, and a row that cannot run is marked
//! `blocked` so the emitter writes it as comments.

use std::collections::HashSet;

use super::super::clauses::{
    dialog_timeout_ms, diff_snapshots, row_clauses, row_snapshots, server_wait_timeout_ms, Diff,
    Inline, INV_ARTIFACT2, INV_BANDOLIER, INV_MAIN,
};
use super::super::decode::Call;
use super::super::log::{EntitySample, RecordLog};
use super::super::pose::{derive_pose, PoseError};
use super::super::sampler::KNOWN_SERVER_EVENTS;
use super::super::segment::{AnchorKind, RecordedRow};
use super::ticks::{before, host_ms, in_row, tick, walk_windows};
use super::{one_line, Line, RowDraft};

#[path = "emit_clicks.rs"]
mod clicks;

/// Extra `wait_server` match fields per `KNOWN_SERVER_EVENTS` key: (key,
/// field, value). The cinematic hold is released per witness, so without
/// `witness_id` another player's release would satisfy the wait (as in
/// `docs/guides/uat-specs/first-session.toml`'s movie-over steps).
const SERVER_EVENT_FIELDS: &[(&str, &str, &str)] =
    &[("cinematic-hold", "witness_id", "${player_entity_id}")];

/// Map one recorded row. `character_sql` goes into the clauses' SQL.
pub(super) fn draft_row(
    log: &RecordLog,
    row: &RecordedRow,
    id: &str,
    character_sql: &str,
) -> RowDraft {
    let mut d = RowDraft {
        id: id.to_string(),
        title: String::new(),
        notes: format!(
            "recorded {}, ticks {}-{}",
            log.start.date, row.first_tick, row.last_tick
        ),
        poses: Vec::new(),
        setup: Vec::new(),
        steps: Vec::new(),
        clauses: row_clauses(log, row, character_sql),
        blocked: None,
    };
    let mut labels = Labels::default();
    for chat in &row.setup_chat {
        d.setup
            .push(action(vec![("chat", s(chat)), ("tier", s("G"))], None));
    }

    let walk = walk_windows(log, row);
    anchor_steps(log, row, &mut d, &mut labels, walk.dialog_opened.is_some());
    if let Some(t) = walk.dialog_opened {
        d.steps.push(action(
            vec![
                ("tool", s("@wait_for")),
                (
                    "args",
                    table(vec![
                        (
                            "lua_condition",
                            s("DialogWin ~= nil and DialogWin:isVisible()"),
                        ),
                        ("timeout_ms", int(dialog_timeout_ms(t))),
                    ]),
                ),
            ],
            None,
        ));
    }

    // What happened after the anchor, in tick order (stable within a tick:
    // clicks, calls, server events, windows). Clicks lead because a tick
    // cannot order them against its calls and a click usually causes one
    // (FS-P4: the tutorial's X, then the dialog's close).
    let mut later = clicks::click_lines(log, row, &mut labels);
    for (tick, call) in &row.calls {
        later.push((*tick, call_line(log, *tick, call, &mut labels)));
    }
    let anchor_ms = host_ms(log, row.anchor.tick);
    for t in in_row(log, row) {
        for key in &t.server_events {
            let line = match KNOWN_SERVER_EVENTS.iter().find(|e| e.0 == key) {
                Some(&(key, target, event, message)) => {
                    let mut fields = vec![("event", s(event))];
                    fields.extend(
                        SERVER_EVENT_FIELDS
                            .iter()
                            .filter(|(k, _, _)| *k == key)
                            .map(|&(_, field, value)| (field, s(value))),
                    );
                    let wait = table(vec![
                        ("log", s(message)),
                        ("target", s(target)),
                        ("fields", table(fields)),
                        (
                            "timeout_ms",
                            int(server_wait_timeout_ms(t.host_ms.saturating_sub(anchor_ms))),
                        ),
                    ]);
                    action(vec![("wait_server", wait)], Some(labels.take(key)))
                }
                None => Line::Todo(format!("server event {}", one_line(key))),
            };
            later.push((t.seq, line));
        }
    }
    later.extend(walk.todos.into_iter().map(|(t, w)| (t, Line::Todo(w))));
    later.sort_by_key(|(t, _)| *t);
    d.steps.extend(later.into_iter().map(|(_, l)| l));

    if !d.steps.iter().any(|l| matches!(l, Line::Action(_))) {
        d.block("no step the mapping table knows");
    }
    if d.clauses.is_empty() {
        d.block("no server change was recorded to check");
    }
    d
}

/// The anchor's title, poses, setup and action steps.
fn anchor_steps(
    log: &RecordLog,
    row: &RecordedRow,
    d: &mut RowDraft,
    labels: &mut Labels,
    dialog_opened: bool,
) {
    let player_entity = log.start.player_entity;
    match (&row.anchor.kind, &row.anchor.call) {
        (AnchorKind::Interact, _) => interact(log, row, d, labels, dialog_opened),
        (AnchorKind::UseItem, &Call::UseItem { item, target }) => {
            let name = item_name(log, row, |from, to, _| to < from);
            d.title = match &name {
                Some(n) => format!("Use {n}"),
                None => format!("Use item {item}"),
            };
            match name {
                Some(n) => d.steps.push(item_action("use", &n, labels)),
                None => d.steps.push(Line::Todo(format!(
                    "useItem {item}: no single named carried stack fell, so the item has no name"
                ))),
            }
            if row.anchor.target.is_some_and(|t| t != player_entity) {
                d.steps.push(Line::Todo(format!("useItem target {target}")));
            }
        }
        (AnchorKind::MoveItem, &Call::MoveItem { item, bag, .. }) => {
            let equipment = INV_BANDOLIER..=INV_ARTIFACT2;
            let (verb, name) = if equipment.contains(&bag) {
                // Into an equipment bag: the stack that grew there.
                (
                    "equip",
                    item_name(log, row, |from, to, c| c == bag && to > from),
                )
            } else if bag == INV_MAIN {
                // Back to the main bag: the equipment stack that shrank.
                let shrank = |from, to, c| equipment.contains(&c) && to < from;
                ("unequip", item_name(log, row, shrank))
            } else {
                ("move", None)
            };
            match (verb, name) {
                ("move", _) | (_, None) => {
                    d.title = format!("Move item {item} to bag {bag}");
                    d.steps
                        .push(Line::Todo(format!("moveItem {item} to bag {bag}")));
                }
                (verb, Some(n)) => {
                    d.title = format!("{} {n}", capitalised(verb));
                    d.steps.push(item_action(verb, &n, labels));
                }
            }
        }
        (AnchorKind::UseAbility, &Call::UseAbility { ability, .. }) => {
            d.title = format!("Use ability {ability}");
            if let Some(t) = row.anchor.target.filter(|t| *t != player_entity) {
                match entity(log, t, row.last_tick).and_then(loadable_tag) {
                    Some(tag) => {
                        d.title = format!("Use ability {ability} on {tag}");
                        d.steps.push(action(
                            vec![
                                ("tool", s("@target")),
                                ("args", table(vec![("tag", s(&tag))])),
                            ],
                            Some(labels.take("target")),
                        ));
                    }
                    None => d.steps.push(Line::Todo(format!(
                        "useAbility {ability} target {t} has no tag"
                    ))),
                }
            }
            d.steps.push(action(
                vec![
                    ("tool", s("@use_ability")),
                    (
                        "args",
                        table(vec![("ability_id", Inline::Int(ability.into()))]),
                    ),
                ],
                Some(labels.take("use")),
            ));
        }
        (AnchorKind::Region, &Call::Region { region, .. }) => {
            d.title = format!("Enter region {region}");
            let at = tick(log, row.anchor.tick)
                .and_then(|t| t.player.as_ref())
                .or_else(|| before(log, row.anchor.tick, |t| t.player.as_ref()));
            let point = match at {
                Some(p) if p.pos.iter().all(|v| v.is_finite()) => p.pos,
                _ => {
                    d.block("no player position in the region's tick");
                    [0.0; 3]
                }
            };
            let xyz = ["x", "y", "z"]
                .into_iter()
                .zip(point)
                .map(|(k, v)| (k, Inline::Float(v, 2)))
                .collect();
            d.steps.push(action(
                vec![
                    ("tool", s("@stand")),
                    ("args", table(vec![("point", table(xyz))])),
                ],
                Some(labels.take("enter")),
            ));
        }
        (kind, call) => {
            d.title = format!("{kind:?}");
            d.steps
                .push(Line::Todo(one_line(&format!("anchor {kind:?} {call:?}"))));
        }
    }
}

/// `interact`: a pose, `@stand` at it, `@camera` and a right click on it.
fn interact(
    log: &RecordLog,
    row: &RecordedRow,
    d: &mut RowDraft,
    labels: &mut Labels,
    dialog_opened: bool,
) {
    let Some(target) = row.anchor.target else {
        d.title = "Interact with an unknown target".into();
        d.block("interact named no target (0, and nothing was targeted)");
        return;
    };
    let sample = entity(log, target, row.last_tick);
    let tag = sample.and_then(|e| e.tag.clone());
    d.title = match &tag {
        Some(t) => format!("Interact with {t}"),
        None => format!("Interact with entity {target}"),
    };
    let pose_id = match &tag {
        Some(t) => pose_id(t),
        None => format!("entity-{target}"),
    };
    let player = before(log, row.anchor.tick, |t| t.player.as_ref());
    let camera = before(log, row.anchor.tick, |t| t.camera.as_ref());
    match (sample, player) {
        (None, _) => d.block(format!("entity {target} was never sampled")),
        (_, None) => d.block("no player position before the interact"),
        (Some(e), Some(p)) => match derive_pose(&pose_id, e, p, camera, &log.start.date) {
            Ok(pose) => d.poses.push(pose),
            Err(err) => d.block(pose_reason(&err)),
        },
    }
    let pose = || ("args", table(vec![("pose", s(&pose_id))]));
    d.setup.push(action(
        vec![("tool", s("@stand")), pose()],
        Some(labels.take("stand")),
    ));
    d.steps.push(action(
        vec![("tool", s("@camera")), pose()],
        Some(labels.take("aim")),
    ));
    let mut click = vec![("pose", s(&pose_id)), ("button", s("right"))];
    if dialog_opened {
        click.push(("expect", s("window")));
    }
    d.steps.push(action(
        vec![("tool", s("@world_click")), ("args", table(click))],
        Some(labels.take("interact")),
    ));
}

/// A call after the anchor: a dialog choice the table knows, else a TODO.
fn call_line(log: &RecordLog, at: u32, call: &Call, labels: &mut Labels) -> Line {
    let finish = |args: Vec<(&'static str, Inline)>, label: String| {
        action(
            vec![("tool", s("@finish_dialog")), ("args", table(args))],
            Some(label),
        )
    };
    match call {
        Call::DialogChoice { button: -1, .. } => finish(vec![], labels.take("close")),
        Call::DialogChoice { dialog, button } => {
            let buttons = at
                .checked_sub(1)
                .and_then(|seq| tick(log, seq))
                .and_then(|t| t.dialog.as_ref())
                .map(|d| d.buttons.clone())
                .unwrap_or_default();
            // The wire button is the cooked ButtonID stored at the clicked
            // position, not an index into the visible buttons the UI read
            // lists (in widget order), so with two or more buttons the click
            // cannot be named (dialog-controller-wire-flow.md § The two
            // return paths). Only a lone Accept is surely the one clicked.
            if buttons == ["Accept"] {
                finish(vec![("accept", Inline::Bool(true))], labels.take("accept"))
            } else {
                Line::Todo(one_line(&format!(
                    "dialogButtonChoice({dialog}, {button}): buttons [{}]",
                    buttons.join(", ")
                )))
            }
        }
        Call::Other { name, method_index } => {
            Line::Todo(format!("c2s {} (method {method_index})", one_line(name)))
        }
        other => Line::Todo(one_line(&format!("c2s {other:?}"))),
    }
}

/// The name of the one carried stack whose count change `pick(from, to,
/// container)` accepts; None when no stack, or stacks of different names,
/// qualify. The anchor's item is an instance id the snapshots do not hold,
/// so the snapshot diff is the only place its name can come from.
fn item_name(
    log: &RecordLog,
    row: &RecordedRow,
    pick: impl Fn(i32, i32, i32) -> bool,
) -> Option<String> {
    let (before, after) = row_snapshots(log, row)?;
    let names: HashSet<String> = diff_snapshots(before, after)
        .into_iter()
        .filter_map(|d| match d {
            Diff::ItemCount {
                container,
                from,
                to,
                name,
                ..
            } if pick(from, to, container) => name,
            _ => None,
        })
        .collect();
    (names.len() == 1)
        .then(|| names.into_iter().next())
        .flatten()
}

fn item_action(verb: &str, name: &str, labels: &mut Labels) -> Line {
    action(
        vec![
            ("tool", s("@item_action")),
            ("args", table(vec![("action", s(verb)), ("name", s(name))])),
        ],
        Some(labels.take(verb)),
    )
}

/// The first sample of `entity` recorded up to `last_tick`.
fn entity(log: &RecordLog, entity: u32, last_tick: u32) -> Option<&EntitySample> {
    log.ticks
        .iter()
        .filter(|t| t.seq <= last_tick)
        .flat_map(|t| &t.entities)
        .find(|e| e.entity_id == entity)
}

/// A tag the spec validator accepts (`[A-Za-z0-9_.]+`).
fn loadable_tag(e: &EntitySample) -> Option<String> {
    e.tag.clone().filter(|t| {
        !t.is_empty()
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    })
}

/// The pose id for a tag: lower-cased, `_` and `.` to `-`.
fn pose_id(tag: &str) -> String {
    tag.to_ascii_lowercase().replace(['_', '.'], "-")
}

fn pose_reason(e: &PoseError) -> String {
    match e {
        PoseError::RadiusOutOfRange(r) => {
            format!("the player stood {r:.2} m from the target (a pose is 1 to 8 m)")
        }
        PoseError::DyOutOfRange(dy) => {
            format!("the player stood {dy:.2} m above the target (a pose is -3 to 3 m)")
        }
        PoseError::NoTag => "the target has no spawn tag".into(),
        PoseError::NoCamera => "the camera readout is not finite".into(),
        PoseError::NoPosition => "a position is missing or not finite".into(),
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}

/// Labels unique in the row: `close`, then `close-2`, ...
#[derive(Default)]
struct Labels(HashSet<String>);

impl Labels {
    fn take(&mut self, base: &str) -> String {
        let mut label = base.to_string();
        let mut n = 1;
        while !self.0.insert(label.clone()) {
            n += 1;
            label = format!("{base}-{n}");
        }
        label
    }
}

fn action(mut kv: Vec<(&'static str, Inline)>, label: Option<String>) -> Line {
    if let Some(l) = label {
        kv.push(("label", Inline::Str(l)));
    }
    Line::Action(table(kv))
}

fn table(kv: Vec<(&'static str, Inline)>) -> Inline {
    Inline::Table(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn s(v: &str) -> Inline {
    Inline::Str(v.to_string())
}

fn int(v: u64) -> Inline {
    Inline::Int(i64::try_from(v).unwrap_or(i64::MAX))
}
