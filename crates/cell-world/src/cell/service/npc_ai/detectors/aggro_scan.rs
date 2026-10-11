//! `npc_ai.aggro_scan`: why the Idle auto-aggro scan passed over a witness
//! (`event=candidate_rejected`) or found nobody (`event=no_candidates`).
//!
//! The reasons are the gates of `npc_ai::aggro_gates` (NA13): `not_player`,
//! `dead`, `same_faction`, `not_hostile`, `gm_ignored`,
//! `out_of_vertical_band`, `out_of_radius`, `no_los`, plus
//! `post_reset_suppressed` for the NA12 window. `aggro_radius` is the NPC's
//! radius in world units (NA02 logged `unbounded` before the gate existed).
//!
//! #1009 (NPC-vs-NPC) adds `event=npc_candidate_rejected` for a HOSTILE NPC
//! the scan passed over, with both entities' ids and factions (reasons
//! `dead`, `target_evading`, `target_unavailable`, `out_of_vertical_band`,
//! `out_of_radius`, `no_los`). An NPC the scanner is not hostile to is not a
//! candidate at all and logs nothing: that is every same-faction neighbour.
//!
//! NA14 adds `event=assist_rejected` for a same-faction neighbour the assist
//! fan-out considered and passed over (reasons `dead`, `not_idle`,
//! `not_hostile`, `post_reset_suppressed`, `gm_ignored`,
//! `out_of_vertical_band`, `out_of_radius`, `no_los`), and
//! `event=assist_joined` for one it pulled in.

use std::time::{Duration, Instant};

use super::NpcIdent;
use crate::cell::space_manager::SpaceManager;

/// One `(npc, player)` pair logs at most once per this window.
const REJECT_SAMPLE_INTERVAL: Duration = Duration::from_secs(10);
/// One NPC's `no_candidates` row at most once per this window.
const NO_CANDIDATES_SAMPLE_INTERVAL: Duration = Duration::from_secs(30);

/// Why a witness is not an aggro candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanReject {
    NotPlayer,
    Dead,
    SameFaction,
    /// The NPC's effective aggression toward the player is not HOSTILE.
    NotHostile,
    /// A GM with `.aggro off` set.
    GmIgnored,
    OutOfVerticalBand,
    OutOfRadius,
    NoLos,
    /// The NPC is inside its post-reset window (NA12); every witness is
    /// passed over until it closes.
    PostResetSuppressed,
    /// NA14 assist only: the would-be assister is not Idle, patrolling or
    /// wandering (it is already fighting, walking home, investigating,
    /// following or dead), so it is not pulled.
    NotIdle,
    /// NPC-vs-NPC only (#1009): the hostile NPC candidate is walking home
    /// (Leashing). It evades (NA12, D-NA03): it takes no threat and could not
    /// fight back, so it is not engaged.
    TargetEvading,
    /// NPC-vs-NPC only (#1009): the hostile NPC candidate is surrendered,
    /// despawning, spawning or in the error state, and is not a fight.
    TargetUnavailable,
}

impl ScanReject {
    /// Stable snake_case label. Treat as API.
    pub fn label(self) -> &'static str {
        match self {
            Self::NotPlayer => "not_player",
            Self::Dead => "dead",
            Self::SameFaction => "same_faction",
            Self::NotHostile => "not_hostile",
            Self::GmIgnored => "gm_ignored",
            Self::OutOfVerticalBand => "out_of_vertical_band",
            Self::OutOfRadius => "out_of_radius",
            Self::NoLos => "no_los",
            Self::PostResetSuppressed => "post_reset_suppressed",
            Self::NotIdle => "not_idle",
            Self::TargetEvading => "target_evading",
            Self::TargetUnavailable => "target_unavailable",
        }
    }
}

/// Log the scan's rejects (sampled per pair) and, when nothing qualified,
/// the `no_candidates` row (sampled per NPC).
pub fn report_scan(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    witness_count: usize,
    rejects: &[(u32, ScanReject)],
    found_candidate: bool,
    now: Instant,
) {
    if rejects.is_empty() && found_candidate {
        return;
    }
    // Built on the first admitted row, not before the sampling below: the
    // names cost NameBook and interner reads (Rule 6, hot path).
    let mut ident_cache: Option<Option<NpcIdent>> = None;
    let Some((npc_pos, radius)) = space_mgr
        .get_entity(npc_id)
        .map(|e| (e.position, crate::cell::combat::aggro_radius(e)))
    else {
        return;
    };
    for &(witness_id, reason) in rejects {
        let Some(suppressed) = space_mgr.npc_detectors.pair_log.admit(
            npc_id,
            witness_id,
            reason.label(),
            now,
            REJECT_SAMPLE_INTERVAL,
        ) else {
            continue;
        };
        let Some(ident) = ident_cache
            .get_or_insert_with(|| NpcIdent::of(space_mgr, npc_id))
            .clone()
        else {
            return;
        };
        let target_pos = space_mgr.get_entity(witness_id).map(|p| p.position);
        // The rejected candidate is a witness entity (an NPC one for
        // `not_player`), so its entity id is `witness_id`; `player_id` is the
        // sgw character id of a player witness and is left out for an NPC
        // (TG-NPC-07: this row logged the entity id as `player_id`).
        let who = space_mgr.player_identity(witness_id);
        tracing::debug!(
            target: "npc_ai.aggro_scan",
            event = "candidate_rejected",
            npc_id,
            tag = %ident.tag,
            npc_name = ident.npc_name,
            template_id = ident.template_id,
            template_name = ident.template_name,
            world = %ident.world,
            space_id = ident.space_id,
            reason = reason.label(),
            witness_id,
            witness_name = space_mgr.entity_label(witness_id),
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            npc_to_target = target_pos.map(|p| p.distance_to(&npc_pos)),
            dy = target_pos.map(|p| p.y - npc_pos.y),
            aggro_radius = radius,
            suppressed,
            "npc_ai.aggro_scan: witness rejected as an aggro candidate"
        );
    }
    if found_candidate {
        return;
    }
    // Nobody is watching: the row would describe a scan no player saw.
    if witness_count == 0 {
        return;
    }
    let Some(suppressed) = space_mgr.npc_detectors.admit_sample(
        npc_id,
        "aggro_scan_no_candidates",
        now,
        NO_CANDIDATES_SAMPLE_INTERVAL,
    ) else {
        return;
    };
    let Some(ident) = ident_cache
        .get_or_insert_with(|| NpcIdent::of(space_mgr, npc_id))
        .clone()
    else {
        return;
    };
    tracing::debug!(
        target: "npc_ai.aggro_scan",
        event = "no_candidates",
        npc_id,
        tag = %ident.tag,
        npc_name = ident.npc_name,
        template_id = ident.template_id,
        template_name = ident.template_name,
        world = %ident.world,
        space_id = ident.space_id,
        witness_count,
        rejected = rejects.len(),
        suppressed,
        "npc_ai.aggro_scan: aggressive Idle NPC found no candidate"
    );
}

/// Log the NA14 assist fan-out's considered-but-rejected neighbours:
/// `event=assist_rejected`, one row per `(assister, victim, reason)` per
/// [`REJECT_SAMPLE_INTERVAL`]. `npc_id` is the would-be assister,
/// `victim_id` the neighbour that just engaged, `target_entity_id` the entity
/// it is fighting. A player target also carries the `target_` identity
/// quartet (TG-NPC-07: the entity id used to be logged as `player_id`).
pub fn report_assist_rejects(
    space_mgr: &mut SpaceManager,
    victim_id: u32,
    target_entity_id: u32,
    rejects: &[(u32, ScanReject)],
    now: Instant,
) {
    let Some(victim_pos) = space_mgr.get_entity(victim_id).map(|e| e.position) else {
        return;
    };
    let target = space_mgr.player_identity(target_entity_id);
    for &(assister_id, reason) in rejects {
        let Some(suppressed) = space_mgr.npc_detectors.pair_log.admit(
            assister_id,
            victim_id,
            reason.label(),
            now,
            REJECT_SAMPLE_INTERVAL,
        ) else {
            continue;
        };
        let Some(ident) = NpcIdent::of(space_mgr, assister_id) else {
            continue;
        };
        let Some(e) = space_mgr.get_entity(assister_id) else {
            continue;
        };
        tracing::debug!(
            target: "npc_ai.aggro_scan",
            event = "assist_rejected",
            npc_id = assister_id,
            tag = %ident.tag,
            npc_name = ident.npc_name,
            template_id = ident.template_id,
            template_name = ident.template_name,
            world = %ident.world,
            space_id = ident.space_id,
            reason = reason.label(),
            victim_id,
            victim_name = space_mgr.entity_label(victim_id),
            target_entity_id,
            target_entity_name = space_mgr.entity_label(target_entity_id),
            target_account_id = target.account_id,
            target_account_name = target.account_name,
            target_player_id = target.player_id,
            target_player_name = target.player_name,
            ai_state = e.ai_state().label(),
            npc_to_victim = e.position.distance_to(&victim_pos),
            dy = victim_pos.y - e.position.y,
            assist_radius = crate::cell::combat::assist_radius(e),
            suppressed,
            "npc_ai.aggro_scan: neighbour not pulled into the fight (assist)"
        );
    }
}

/// Log the NPC-vs-NPC scan's rejects (#1009): `event=npc_candidate_rejected`,
/// one row per `(npc, target, reason)` per [`REJECT_SAMPLE_INTERVAL`], with
/// both entities' ids, factions and tags, so a friendly that stands idle next
/// to a hostile can be explained from SigNoz alone. Only HOSTILE pairs reach
/// here; the scan never considers an NPC its faction does not fight.
pub fn report_npc_rejects(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    rejects: &[(u32, ScanReject)],
    now: Instant,
) {
    if rejects.is_empty() {
        return;
    }
    // Built on the first admitted row, not before the sampling below: the
    // names cost NameBook and interner reads (Rule 6, hot path).
    let mut ident_cache: Option<Option<NpcIdent>> = None;
    let Some((npc_pos, npc_faction, radius)) = space_mgr
        .get_entity(npc_id)
        .map(|e| (e.position, e.faction, crate::cell::combat::aggro_radius(e)))
    else {
        return;
    };
    for &(target_id, reason) in rejects {
        let Some(suppressed) = space_mgr.npc_detectors.pair_log.admit(
            npc_id,
            target_id,
            reason.label(),
            now,
            REJECT_SAMPLE_INTERVAL,
        ) else {
            continue;
        };
        let Some(ident) = ident_cache
            .get_or_insert_with(|| NpcIdent::of(space_mgr, npc_id))
            .clone()
        else {
            return;
        };
        let Some(t) = space_mgr.get_entity(target_id) else {
            continue;
        };
        tracing::debug!(
            target: "npc_ai.aggro_scan",
            event = "npc_candidate_rejected",
            npc_id,
            tag = %ident.tag,
            npc_name = ident.npc_name,
            template_id = ident.template_id,
            template_name = ident.template_name,
            world = %ident.world,
            space_id = ident.space_id,
            npc_faction,
            reason = reason.label(),
            target_id,
            target_name = space_mgr.entity_label(target_id),
            target_tag = t.tag.as_deref().unwrap_or(""),
            target_faction = t.faction,
            target_ai_state = t.ai_state().label(),
            npc_to_target = t.position.distance_to(&npc_pos),
            dy = t.position.y - npc_pos.y,
            aggro_radius = radius,
            suppressed,
            "npc_ai.aggro_scan: hostile NPC rejected as an aggro candidate"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use cimmeria_names::{NameBook, Table};

    use super::{report_assist_rejects, report_scan, ScanReject, SpaceManager};
    use crate::test_support::LogCapture;

    const NPC: u32 = 910;
    /// A player entity whose sgw `player_id` is deliberately not its entity
    /// id, so a row that logs the entity id as `player_id` is caught.
    const PLAYER_ENTITY: u32 = 911;
    const SGW_PLAYER_ID: i32 = 4242;
    const ASSISTER: u32 = 912;

    fn add_player(mgr: &mut SpaceManager) {
        mgr.create_entity(PLAYER_ENTITY, "Agnos", [5.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let p = mgr.get_entity_mut(PLAYER_ENTITY).unwrap();
        p.is_player = true;
        p.account_id = Some(6);
        p.player_id = Some(SGW_PLAYER_ID);
        p.character_name = Some("Teal'c".into());
        p.stamp_log_names(Some("Teal'c"), Some("sgc_login"));
    }

    /// TG-NPC-07 revert proof: log the witness's entity id as `player_id`
    /// again and `player_id` reads 911, not 4242, and `witness_id` is gone.
    #[test]
    fn candidate_rejected_logs_witness_entity_and_sgw_player_id() {
        let mut mgr = agnos_with_npc();
        add_player(&mut mgr);
        let logs = LogCapture::install();
        report_scan(
            &mut mgr,
            NPC,
            1,
            &[(PLAYER_ENTITY, ScanReject::SameFaction)],
            false,
            Instant::now(),
        );
        let all = logs.all();
        let row = all
            .iter()
            .find(|c| c.target == "npc_ai.aggro_scan" && c.has_field("event", "candidate_rejected"))
            .expect("the candidate_rejected row");
        assert!(row.has_field("witness_id", "911"), "{row:?}");
        assert!(row.has_field("player_id", "4242"), "{row:?}");
        assert!(row.has_field("player_name", "Teal'c"), "{row:?}");
        assert!(row.has_field("account_id", "6"), "{row:?}");
        assert!(row.has_field("account_name", "sgc_login"), "{row:?}");
    }

    /// TG-NPC-07 revert proof: log the fight target's entity id as
    /// `player_id` again and `target_player_id` (and `target_entity_id`)
    /// disappear from the row.
    #[test]
    fn assist_rejected_logs_target_entity_and_sgw_player_id() {
        let mut mgr = agnos_with_npc();
        add_player(&mut mgr);
        mgr.spawn_npc(ASSISTER, "Agnos", [40.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let logs = LogCapture::install();
        report_assist_rejects(
            &mut mgr,
            NPC,
            PLAYER_ENTITY,
            &[(ASSISTER, ScanReject::OutOfRadius)],
            Instant::now(),
        );
        let all = logs.all();
        let row = all
            .iter()
            .find(|c| c.target == "npc_ai.aggro_scan" && c.has_field("event", "assist_rejected"))
            .expect("the assist_rejected row");
        assert!(row.has_field("target_entity_id", "911"), "{row:?}");
        assert!(row.has_field("target_player_id", "4242"), "{row:?}");
        assert!(row.has_field("target_player_name", "Teal'c"), "{row:?}");
        assert!(row.has_field("target_account_id", "6"), "{row:?}");
        assert!(!row.fields.contains_key("player_id"), "{row:?}");
    }

    fn agnos_with_npc() -> SpaceManager {
        let mut book = NameBook::empty();
        book.insert(Table::Templates, 81, "NT03_Scan_Template");
        book.insert(Table::Texts, 82, "Watchful Guard");
        cimmeria_names::global().store(book);

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
        )
        .unwrap();
        mgr.spawn_npc(NPC, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.template_id = Some(81);
        npc.name_id = Some(82);
        mgr
    }

    /// Revert proof: drop the `witness_count == 0` return and the first
    /// assertion fails, because the unwitnessed scan writes a `no_candidates`
    /// row.
    #[test]
    fn no_candidates_needs_a_witness() {
        let mut mgr = agnos_with_npc();
        let logs = LogCapture::install();
        report_scan(&mut mgr, NPC, 0, &[], false, Instant::now());
        let unwitnessed = logs.all();
        assert!(
            !unwitnessed
                .iter()
                .any(|c| c.target == "npc_ai.aggro_scan" && c.has_field("event", "no_candidates")),
            "{unwitnessed:?}"
        );

        report_scan(&mut mgr, NPC, 1, &[], false, Instant::now());
        let witnessed = logs.all();
        let row = witnessed
            .iter()
            .find(|c| c.target == "npc_ai.aggro_scan" && c.has_field("event", "no_candidates"))
            .expect("the no_candidates row once someone watches");
        assert!(row.has_field("witness_count", "1"), "{row:?}");
    }
}
