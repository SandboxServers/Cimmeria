//! The one way to change an NPC's `ai_state`.
//!
//! Before NA00 there were 23 raw `ai_state =` writes spread over the AI
//! handlers, the combat layer, the content executor, the GM console and the
//! respawn tick, and none of them logged the change (audit gap T8). There
//! was no way to read one NPC's state timeline out of SigNoz.
//!
//! Now `CellEntity::ai_state` is a private field. The only raw writer,
//! `CellEntity::replace_ai_state_unlogged`, is called from this file and
//! nowhere else — a guard test at the bottom scans the workspace for it —
//! so every production state change goes through [`set_ai_state_on`] and
//! emits:
//!
//! - `npc_ai.transition` DEBUG `event="state_change"` with `from`, `to`,
//!   `reason` and the common NPC fields (`npc_id, tag, template_id, world,
//!   space_id`), plus `npc_to_spawn`, `threat_count` and `nav_path_len`;
//! - counter `npc_ai_transitions_total{world, from, to, reason}`.
//!
//! A real state change also stops the NPC (clears `nav_path`, zeroes
//! `velocity`) through [`super::stop_movement_on`]. See
//! `movement_stop` for why (NA10).
//!
//! A write that does not change the state (content re-asserting `idle` on an
//! idle NPC) still writes but emits nothing: the row means "the state
//! changed", and a no-op would put a false edge on the timeline.
//!
//! See `docs/analysis/npc-ai-restoration/telemetry.md` §2.1.

use cimmeria_entity::cell_entity::{AiState, CellEntity};

use crate::cell::space_manager::SpaceManager;

/// Why an NPC's AI state changed. Enumerated, never free text: the label is
/// a metric label and a SigNoz group-by key.
///
/// `target_dead` from the telemetry plan is folded into
/// [`AiTransitionReason::TargetLost`]: since NA12 a target that dies,
/// disconnects or stays beyond the NPC's AoI for the grace period sends the
/// NPC home under that one reason. [`AiTransitionReason::ThreatEmpty`] is
/// what is left: the threat list was already empty when the fight tick ran
/// (content or the GM console cleared it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell) enum AiTransitionReason {
    /// Idle auto-aggro seeded threat on a witness (`cause=proximity`).
    AutoAggro,
    /// Damage preempted Idle / Patrol / Wander / Investigating / Follow.
    ThreatPreempt,
    /// Fighting with nobody left on the threat list.
    ThreatEmpty,
    /// The NPC itself went past its leash radius around spawn (NA12: the
    /// NPC's distance, not the target's).
    LeashOut,
    /// The last target died, disconnected, or stayed out of the NPC's AoI
    /// for the grace period; the NPC walks home.
    TargetLost,
    /// The NPC stood at the end of a route that cannot reach its target
    /// (another mesh island, an off-mesh target) for the grace period, or
    /// could not get onto the mesh at all; it walks home (NA15).
    Unreachable,
    /// The walk home reached spawn.
    LeashArrived,
    /// The walk home could not be planned, or took longer than the timeout,
    /// so the NPC was snapped to spawn instead.
    LeashSnapFallback,
    /// The NPC died (`combat::mark_npc_dead`).
    Died,
    /// The respawn tick revived a corpse.
    Respawn,
    /// A content-engine action (`set_npc_poi`, `set_follow_target`,
    /// `set_npc_ai_state`, or `generate_threat` preempting into Fighting).
    Content,
    /// A GM `.`-console command (`.debug_follow`, `.path_assign`,
    /// `.path_unassign`, `.respawnall`).
    GmCommand,
    /// Idle NPC with a patrol path starts patrolling.
    PatrolStart,
    /// Idle NPC with a wander radius starts wandering.
    WanderStart,
    /// Patrol state with an empty path drops to Idle.
    PatrolNoPath,
    /// Wander state with a zero radius drops to Idle.
    WanderNoRadius,
    /// Wander state with no spawn anchor drops to Idle.
    WanderNoSpawn,
    /// Investigating with no POI drops to Idle.
    InvestigateNoPoi,
    /// Investigate dwell elapsed; back to Idle.
    InvestigateDone,
    /// Follow state with no follow target drops to Idle.
    FollowNoTarget,
    /// The follow target no longer resolves (despawned / disconnected).
    FollowTargetGone,
}

impl AiTransitionReason {
    /// Stable snake_case label. Treat as API — see the enum doc.
    pub(in crate::cell) fn label(self) -> &'static str {
        match self {
            Self::AutoAggro => "auto_aggro",
            Self::ThreatPreempt => "threat_preempt",
            Self::ThreatEmpty => "threat_empty",
            Self::LeashOut => "leash_out",
            Self::TargetLost => "target_lost",
            Self::Unreachable => "unreachable",
            Self::LeashArrived => "leash_arrived",
            Self::LeashSnapFallback => "leash_snap_fallback",
            Self::Died => "died",
            Self::Respawn => "respawn",
            Self::Content => "content",
            Self::GmCommand => "gm_command",
            Self::PatrolStart => "patrol_start",
            Self::WanderStart => "wander_start",
            Self::PatrolNoPath => "patrol_no_path",
            Self::WanderNoRadius => "wander_no_radius",
            Self::WanderNoSpawn => "wander_no_spawn",
            Self::InvestigateNoPoi => "investigate_no_poi",
            Self::InvestigateDone => "investigate_done",
            Self::FollowNoTarget => "follow_no_target",
            Self::FollowTargetGone => "follow_target_gone",
        }
    }
}

/// World name of the space `entity_id` is in, or `"unknown"`. Owned so a
/// caller can resolve it before taking a `&mut` borrow of the entity.
pub(in crate::cell) fn world_label(space_mgr: &SpaceManager, entity_id: u32) -> String {
    space_mgr
        .get_entity_space_id(entity_id)
        .and_then(|sid| space_mgr.world_name_for_space(sid))
        .unwrap_or("unknown")
        .to_string()
}

/// Change `npc_id`'s AI state, logging and counting the transition.
///
/// Returns the previous state, or `None` when the entity does not exist (in
/// which case nothing is written or logged).
pub(in crate::cell) fn set_ai_state(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    to: AiState,
    reason: AiTransitionReason,
) -> Option<AiState> {
    let world = world_label(space_mgr, npc_id);
    let npc = space_mgr.get_entity_mut(npc_id)?;
    Some(set_ai_state_on(npc, &world, to, reason))
}

/// [`set_ai_state`] for a caller that already holds `&mut CellEntity`. Take
/// `world` from [`world_label`] before borrowing the entity.
///
/// Returns the previous state.
pub(in crate::cell) fn set_ai_state_on(
    npc: &mut CellEntity,
    world: &str,
    to: AiState,
    reason: AiTransitionReason,
) -> AiState {
    let from = npc.replace_ai_state_unlogged(to);
    if from == to {
        return from;
    }
    let reason_label = reason.label();
    cimmeria_observability::counter!(
        "npc_ai_transitions_total",
        "world" => world.to_string(),
        "from" => from.label(),
        "to" => to.label(),
        "reason" => reason_label,
    );
    tracing::debug!(
        target: "npc_ai.transition",
        event = "state_change",
        npc_id = npc.entity_id.0,
        tag = npc.tag.as_deref().unwrap_or(""),
        template_id = npc.template_id,
        world,
        space_id = npc.space_id.0,
        from = from.label(),
        to = to.label(),
        reason = reason_label,
        npc_to_spawn = npc.spawn_position.map(|p| p.distance_to(&npc.position)),
        threat_count = npc.threat_list.len(),
        nav_path_len = npc.nav_path.len(),
        "npc_ai: {} -> {} ({reason_label})",
        from.label(),
        to.label(),
    );
    // A route planned in one state is never valid in the next, and a path
    // cleared without zeroing velocity makes the client run the NPC in
    // place (NA10, audit S1). Stopped after the row above, so its
    // `nav_path_len` shows what was dropped.
    super::stop_movement_on(npc);
    // The chase's route record and unreachable timer belong to one fight: a
    // later fight must not inherit a hold that is already most of the way
    // to giving up (NA15).
    npc.leash.clear_chase();
    super::detectors::idle_parked::check(npc, world, from, reason_label);
    from
}

/// Test fixtures only: put an NPC into `to` without a transition row, the
/// way a scenario's starting state is arranged rather than reached.
#[cfg(test)]
pub(in crate::cell) fn force_ai_state(npc: &mut CellEntity, to: AiState) {
    npc.replace_ai_state_unlogged(to);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    fn mgr_with_npc() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.spawn_npc(100, "Agnos", [15.0, 0.0, 15.0], [0.0; 3])
            .unwrap();
        let npc = mgr.get_entity_mut(100).unwrap();
        npc.tag = Some("guard_a".into());
        npc.template_id = Some(77);
        npc.spawn_position = Some(cimmeria_common::Vector3::new(12.0, 0.0, 11.0));
        npc.threat_list.insert(1, 5.0);
        mgr
    }

    fn transitions(
        logs: &crate::test_support::LogCaptureGuard,
    ) -> Vec<crate::test_support::Captured> {
        logs.all()
            .into_iter()
            .filter(|c| c.target == "npc_ai.transition")
            .collect()
    }

    /// The row carries every field the telemetry plan names, with the
    /// enumerated labels, and the write lands.
    #[test]
    fn state_change_row_carries_from_to_reason_and_common_fields() {
        let mut mgr = mgr_with_npc();
        let logs = LogCapture::install();

        let prev = set_ai_state(
            &mut mgr,
            100,
            AiState::Leashing,
            AiTransitionReason::LeashOut,
        );

        assert_eq!(prev, Some(AiState::Idle));
        assert_eq!(mgr.get_entity(100).unwrap().ai_state(), AiState::Leashing);
        let rows = transitions(&logs);
        assert_eq!(rows.len(), 1, "exactly one transition row: {rows:?}");
        let row = &rows[0];
        assert_eq!(row.level, Level::DEBUG);
        for (k, v) in [
            ("event", "state_change"),
            ("from", "idle"),
            ("to", "leashing"),
            ("reason", "leash_out"),
            ("npc_id", "100"),
            ("tag", "guard_a"),
            ("template_id", "77"),
            ("world", "Agnos"),
            ("threat_count", "1"),
            ("nav_path_len", "0"),
            ("npc_to_spawn", "5.0"),
        ] {
            assert!(row.has_field(k, v), "field {k}={v} missing: {row:?}");
        }
        assert!(row.fields.contains_key("space_id"), "{row:?}");
    }

    /// Re-asserting the current state writes nothing new and logs nothing:
    /// the row means "the state changed".
    #[test]
    fn same_state_write_emits_no_row() {
        let mut mgr = mgr_with_npc();
        let logs = LogCapture::install();
        set_ai_state(&mut mgr, 100, AiState::Idle, AiTransitionReason::Content);
        assert!(transitions(&logs).is_empty());
    }

    /// Missing entity: no write, no row, `None`.
    #[test]
    fn missing_entity_returns_none() {
        let mut mgr = mgr_with_npc();
        let logs = LogCapture::install();
        assert_eq!(
            set_ai_state(
                &mut mgr,
                999,
                AiState::Fighting,
                AiTransitionReason::AutoAggro
            ),
            None
        );
        assert!(transitions(&logs).is_empty());
    }

    /// Labels are API. Pin a few so a rename is a deliberate test edit.
    #[test]
    fn reason_labels_are_stable_snake_case() {
        assert_eq!(AiTransitionReason::ThreatPreempt.label(), "threat_preempt");
        assert_eq!(AiTransitionReason::AutoAggro.label(), "auto_aggro");
        assert_eq!(AiTransitionReason::LeashArrived.label(), "leash_arrived");
        assert_eq!(AiTransitionReason::TargetLost.label(), "target_lost");
        assert_eq!(
            AiTransitionReason::LeashSnapFallback.label(),
            "leash_snap_fallback"
        );
        assert_eq!(AiState::Investigating.label(), "investigating");
    }

    /// Guard for T8: the raw writer may be named only here and at its
    /// definition. The field itself is private, so `npc.ai_state = X` no
    /// longer compiles anywhere; this closes the one remaining door.
    ///
    /// Scans every `.rs` file under `crates/`. Reverting any call site to the
    /// raw writer — or adding a new one — fails this test.
    #[test]
    fn raw_ai_state_writer_is_called_only_from_the_transition_helper() {
        const RAW: &str = "replace_ai_state_unlogged";
        let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/ dir")
            .to_path_buf();
        let allowed = [
            "entity/src/cell_entity/ai_state.rs",
            "services/src/cell/service/npc_ai/transition.rs",
        ];
        let mut offenders = Vec::new();
        let mut stack = vec![crates_dir.clone()];
        let mut scanned = 0usize;
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in rd.flatten() {
                let path = entry.path();
                let name = entry.file_name();
                if path.is_dir() {
                    if name != "target" && name != "node_modules" {
                        stack.push(path);
                    }
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs") {
                    continue;
                }
                scanned += 1;
                let rel = path
                    .strip_prefix(&crates_dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if allowed.contains(&rel.as_str()) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (i, line) in text.lines().enumerate() {
                    if line.contains(RAW) {
                        offenders.push(format!("{rel}:{}: {}", i + 1, line.trim()));
                    }
                }
            }
        }
        assert!(
            scanned > 100,
            "scan found only {scanned} files; wrong root?"
        );
        assert!(
            offenders.is_empty(),
            "raw AI-state writes outside the transition helper — route them through \
             `npc_ai::set_ai_state` / `set_ai_state_on` so the transition is logged:\n{}",
            offenders.join("\n")
        );
    }

    /// NA00 review: `force_ai_state` is the one other caller of the raw
    /// writer allowed in this file, and it is only safe because it is
    /// test-only. Losing the `#[cfg(test)]` directly above it would hand
    /// production code an unlogged state write that the scan above allows
    /// (it whitelists this whole file).
    #[test]
    fn force_ai_state_stays_test_only() {
        let src = include_str!("transition.rs");
        let lines: Vec<&str> = src.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.contains("fn force_ai_state("))
            .expect("force_ai_state is defined in transition.rs");
        assert_eq!(
            lines[at - 1].trim(),
            "#[cfg(test)]",
            "`#[cfg(test)]` must sit directly above `fn force_ai_state`"
        );
    }

    /// NA00 review: inside the entity crate the field is reachable without
    /// the setter (privacy is per module tree), so a raw `ai_state =` write
    /// anywhere under `cell_entity/` other than `ai_state.rs` — the setter's
    /// home — bypasses the transition row. Flag any.
    #[test]
    fn no_raw_ai_state_assignment_in_the_entity_crate() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/ dir")
            .join("entity/src/cell_entity");
        let mut offenders = Vec::new();
        let mut scanned = 0usize;
        let mut stack = vec![dir];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else {
                continue;
            };
            for entry in rd.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|e| e != "rs")
                    || path.file_name().is_some_and(|n| n == "ai_state.rs")
                {
                    continue;
                }
                scanned += 1;
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (i, line) in text.lines().enumerate() {
                    let code = line.split("//").next().unwrap_or("");
                    // `ai_state =` but not `ai_state ==`.
                    if let Some(p) = code.find("ai_state =") {
                        if !code[p..].starts_with("ai_state ==") {
                            offenders.push(format!(
                                "{}:{}: {}",
                                path.display(),
                                i + 1,
                                line.trim()
                            ));
                        }
                    }
                }
            }
        }
        assert!(scanned > 5, "scan found only {scanned} files; wrong root?");
        assert!(
            offenders.is_empty(),
            "raw `ai_state =` writes in cell_entity outside ai_state.rs:\n{}",
            offenders.join("\n")
        );
    }
}
