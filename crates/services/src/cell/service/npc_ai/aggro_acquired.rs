//! `npc_ai.aggro` `event="acquired"`: one INFO row for every entry into
//! Fighting, saying *why* the NPC engaged (audit gap T4).
//!
//! Every entry into Fighting goes through `combat::generate_threat`, so that
//! is where this row is emitted; the caller names the `AggroCause`. Before
//! NA00 the only trace was an unstructured "preempt -> Fighting" line with no
//! cause, so "did it aggro because I walked in, or because I shot it?" could
//! not be answered from logs.
//!
//! See `docs/analysis/npc-ai-restoration/telemetry.md` §2.1.

use cimmeria_entity::navigation::LineOfSight;

use super::transition::{world_label, AiTransitionReason};
use crate::cell::combat::AggroCause;
use crate::cell::space_manager::SpaceManager;

// `AggroCause` itself lives next to `generate_threat` (it is part of that
// public signature); the transition mapping is AI-side, so it lives here.
impl AggroCause {
    /// The `npc_ai.transition` reason for the Fighting entry this cause
    /// produces.
    pub(in crate::cell) fn transition_reason(self) -> AiTransitionReason {
        match self {
            Self::Proximity => AiTransitionReason::AutoAggro,
            Self::Damage => AiTransitionReason::ThreatPreempt,
            Self::ContentThreat => AiTransitionReason::Content,
        }
    }
}

/// The three-state `los` / `has_los` label on the AI's per-NPC rows
/// (`clear`, `blocked`, `unknown`). `npc_ai.los` itself uses the longer
/// `LineOfSight::label` (`unknown_off_mesh`) because it is the row that
/// explains the unknown.
pub(super) fn los_label(los: LineOfSight) -> &'static str {
    match los {
        LineOfSight::Clear => "clear",
        LineOfSight::Blocked => "blocked",
        LineOfSight::Unknown => "unknown",
    }
}

/// Emit the `acquired` row and count it. Call after the NPC is in Fighting.
///
/// `has_los` is three-state (`clear`, `blocked`, `unknown`) rather than the
/// combat policy's bool, because "unknown" (an endpoint off the mesh) is
/// exactly what an operator needs to tell apart from a real wall.
pub(in crate::cell) fn log_aggro_acquired(
    space_mgr: &SpaceManager,
    npc_id: u32,
    target_id: u32,
    from: cimmeria_entity::cell_entity::AiState,
    cause: AggroCause,
) {
    let Some(npc) = space_mgr.get_entity(npc_id) else {
        return;
    };
    let world = world_label(space_mgr, npc_id);
    let target = space_mgr.get_entity(target_id);
    let (player_id, account_id) = match target {
        Some(t) if t.is_player => (t.player_id, t.account_id),
        _ => (None, None),
    };
    let npc_to_target = target.map(|t| t.position.distance_to(&npc.position));
    let dy = target.map(|t| t.position.y - npc.position.y);
    let has_los = los_label(space_mgr.line_of_sight(npc_id, target_id));

    cimmeria_observability::counter!(
        "npc_ai_aggro_total",
        "world" => world.clone(),
        "cause" => cause.label(),
    );
    tracing::info!(
        target: "npc_ai.aggro",
        event = "acquired",
        cause = cause.label(),
        npc_id,
        tag = npc.tag.as_deref().unwrap_or(""),
        template_id = npc.template_id,
        world = world.as_str(),
        space_id = npc.space_id.0,
        from = from.label(),
        target_id,
        player_id,
        account_id,
        npc_to_target,
        dy,
        has_los,
        // Effective `EMobAggressionLevel` toward players (1 = hostile) and
        // whether it came from an override (NA13).
        aggression = crate::cell::combat::aggression_toward_players(npc).level(),
        aggression_override = npc.aggro.override_level.map(|l| l.level()),
        "npc_ai: aggro acquired ({})",
        cause.label(),
    );
}

#[cfg(test)]
mod tests {
    use crate::cell::combat::{generate_threat, AggroCause};
    use crate::cell::space_manager::SpaceManager;
    use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
    use tracing::Level;

    fn mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [10.0, 2.0, 10.0], [0.0; 3])
            .unwrap();
        let p = mgr.get_entity_mut(1).unwrap();
        p.is_player = true;
        p.player_id = Some(4242);
        p.account_id = Some(77);
        mgr.spawn_npc(100, "Agnos", [13.0, 0.0, 14.0], [0.0; 3])
            .unwrap();
        mgr.get_entity_mut(100).unwrap().aggro.override_level =
            Some(cimmeria_entity::cell_entity::MobAggression::Hostile);
        mgr
    }

    fn acquired(logs: &LogCaptureGuard) -> Vec<Captured> {
        logs.all()
            .into_iter()
            .filter(|c| c.target == "npc_ai.aggro")
            .collect()
    }

    fn transition(logs: &LogCaptureGuard) -> Captured {
        logs.all()
            .into_iter()
            .find(|c| c.target == "npc_ai.transition")
            .expect("a Fighting entry must log a transition")
    }

    /// Entry into Fighting emits one INFO `acquired` row with its cause and
    /// the target's identity and geometry. Fails if the row is removed from
    /// `generate_threat`.
    #[test]
    fn entry_into_fighting_emits_acquired_with_cause() {
        let mut mgr = mgr();
        let logs = LogCapture::install();

        let _ = generate_threat(&mut mgr, 1, 100, 1.0, AggroCause::Proximity);

        let rows = acquired(&logs);
        assert_eq!(rows.len(), 1, "{rows:?}");
        let row = &rows[0];
        assert_eq!(row.level, Level::INFO);
        for (k, v) in [
            ("event", "acquired"),
            ("cause", "proximity"),
            ("from", "idle"),
            ("npc_id", "100"),
            ("target_id", "1"),
            ("player_id", "4242"),
            ("account_id", "77"),
            ("npc_to_target", "5.385164737701416"),
            ("dy", "2.0"),
            // No navmesh in this space: the three-state answer is unknown,
            // which the combat policy treats as clear.
            ("has_los", "unknown"),
            ("aggression", "1"),
            ("world", "Agnos"),
        ] {
            assert!(row.has_field(k, v), "field {k}={v} missing: {row:?}");
        }
        let t = transition(&logs);
        assert!(t.has_field("reason", "auto_aggro"), "{t:?}");
        assert!(t.has_field("to", "fighting"), "{t:?}");
    }

    /// More threat on an NPC that is already fighting is not a new
    /// acquisition: one row per entry, not per hit.
    #[test]
    fn further_threat_while_fighting_emits_nothing() {
        let mut mgr = mgr();
        let _ = generate_threat(&mut mgr, 1, 100, 1.0, AggroCause::Damage);
        let logs = LogCapture::install();

        let _ = generate_threat(&mut mgr, 1, 100, 50.0, AggroCause::Damage);

        assert!(acquired(&logs).is_empty());
    }

    /// Damage and content causes map to their own transition reasons.
    #[test]
    fn damage_and_content_causes_label_the_transition() {
        for (cause, label, reason) in [
            (AggroCause::Damage, "damage", "threat_preempt"),
            (AggroCause::ContentThreat, "content_threat", "content"),
        ] {
            let mut mgr = mgr();
            let logs = LogCapture::install();
            let _ = generate_threat(&mut mgr, 1, 100, 10.0, cause);
            let row = acquired(&logs).pop().expect("acquired row");
            assert!(row.has_field("cause", label), "{row:?}");
            assert!(transition(&logs).has_field("reason", reason));
        }
    }
}
