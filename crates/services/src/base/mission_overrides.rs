//! Cimmeria-side overrides for `CookedDataMissions.pak` entries.
//!
//! The client's mission catalogue (step ids, step display text, objective
//! ids, objective display text) is loaded from the `_<missionId>` entries
//! inside `CookedDataMissions.pak` — *not* from any wire message the server
//! sends. So introducing a new server-side step id (e.g., for "Equip the
//! pistol") is invisible to the UI unless the corresponding mission XML in
//! the client's cache gains a matching `<Steps StepID="…">` row.
//!
//! Rather than ship a modified PAK file (which would mean every player
//! redownloads the artifact), we patch entries in-memory at server startup
//! and use the protocol's existing per-key invalidation channel — the
//! `onVersionInfo` packet carries an `InvalidKeys` ARRAY<u32>, which the
//! client (`ServerConnection::onVersionInfo`) reads and uses to drop only
//! the named entries from its local cache. The client then issues
//! `elementDataRequest` for each invalidated key and receives the patched
//! XML from the server's [`ResourceCache`].
//!
//! This module's job: produce the patched XML bytes for missions that
//! Cimmeria adds steps to. The byte layout follows the QA-build conventions
//! documented in [`docs/engine/cooked-data-pak-format.md`] — same
//! `<COOKED_MISSION>` root, same `<Steps>`/`<Objectives>` children, same
//! attribute style. The new `<Steps>` block goes immediately before the
//! closing `</COOKED_MISSION>` tag so existing entries' byte ranges stay
//! intact for any downstream tooling that diffs by offset.

/// One mission's override: which mission to patch, what `<Steps>` XML to
/// inject, and which existing step the new block should sit *after* in the
/// XML stream.
///
/// The original BigWorld client mission-state machine indexes each step by
/// its order in the XML (mirroring `MissionManager.py`'s
/// `mission.steps[stepId].index` and the sequential `nextIndex <= current`
/// guard). A step injected past every existing `</Steps>` close lands at
/// the highest index, and an `advance_step` from a low-index step to a
/// high-index one reads to the client as "skip everything in between" —
/// it then snaps the displayed step to the next sequential step it knows
/// instead of honouring the targeted advance. So the override has to
/// place itself adjacent to the step the chain is advancing *from*, not
/// at the tail of the XML.
pub struct MissionOverride {
    pub mission_id: u32,
    /// Insert the new `<Steps>` block immediately after this step's
    /// closing `</Steps>` tag. Use the step id the chain is advancing
    /// from when introducing a new intermediary step.
    pub insert_after_step_id: u32,
    pub injected_steps_xml: &'static str,
}

/// All Cimmeria-introduced mission overrides for the `CookedDataMissions`
/// category (id `3`). Adding a new entry here:
///
///   1. Insert the row in `db/resources/Missions/Seed/mission_steps.sql`
///      (server keeps its own catalog used by the chain engine for
///      `advance_step` / `objective_status` evaluation).
///   2. Insert the matching objective rows in
///      `db/resources/Missions/Seed/mission_objectives.sql`.
///   3. Add a `MissionOverride` here so the client's UI can render the
///      step text and the new objective.
///   4. Reference the new step id in the relevant content chain action.
///
/// The server-side seed and the client-side override must agree on
/// `StepID`, `ObjectiveID`, and the `IsHidden` / `IsOptional` flags — the
/// `onObjectiveUpdate` wire message only carries the id and status, so any
/// drift surfaces as a missing UI line on the player's screen even though
/// the chain engine thinks it's making progress.
// The objective `<DisplayLogText>` is a single space (`" "`) on purpose.
// The original game's mission XML uses the step's `<StepDisplayLogText>`
// for the player-visible objective string and leaves the per-objective
// `<DisplayLogText>` as a space — see `_622` step 2113 / objective 2452 /
// `_641` step 2121 / objective 4116 in the canonical PAK. Putting the
// real text on both produces a visibly duplicated line in the mission log
// (the screenshot from the live UI on the Frost step).
pub const MISSION_OVERRIDES: &[MissionOverride] = &[
    MissionOverride {
        mission_id: 622,
        // Insert immediately after step 2113 ("Search the nearby corpses").
        // Chain 1003 advances 2113 → 80622 on Frost loot; for the client
        // to render the new step instead of skipping it, 80622's XML
        // index must equal current+1 = 1.
        insert_after_step_id: 2113,
        injected_steps_xml:
            "<Steps StepEnabled=\"false\" StepID=\"80622\" AwardXP=\"false\" Difficulty=\"1\">\
             <StepDisplayLogText>Equip the pistol from your inventory.</StepDisplayLogText>\
             <Objectives IsOptional=\"false\" ObjectiveID=\"90622\" AwardXP=\"false\" \
             IsHidden=\"false\" IsEnabled=\"false\" Difficulty=\"1\">\
             <DisplayLogText> </DisplayLogText>\
             </Objectives>\
             </Steps>",
    },
    MissionOverride {
        mission_id: 641,
        // Insert immediately after step 2121 ("Prepare yourself for the
        // escape"), pushing the existing 3563 / 3564 steps' XML indexes
        // up by one. Without this, 80641 lands at index 3 while the
        // chain advances from index 0 (step 2121); the client treats
        // that as a skip-three jump, the sequential-progression guard
        // snaps the displayed step to the next sequential index (3563),
        // and the player never sees "Equip the P90".
        insert_after_step_id: 2121,
        injected_steps_xml:
            "<Steps StepEnabled=\"false\" StepID=\"80641\" AwardXP=\"false\" Difficulty=\"1\">\
             <StepDisplayLogText>Equip the P90 from your inventory.</StepDisplayLogText>\
             <Objectives IsOptional=\"false\" ObjectiveID=\"90641\" AwardXP=\"false\" \
             IsHidden=\"false\" IsEnabled=\"false\" Difficulty=\"1\">\
             <DisplayLogText> </DisplayLogText>\
             </Objectives>\
             </Steps>",
    },
];

/// Apply a single mission override to a chunk of XML bytes from the PAK.
///
/// Inserts the new `<Steps>` block immediately after the closing
/// `</Steps>` tag of the override's `insert_after_step_id`. This places
/// the new step at the right XML index for the client's sequential
/// progression to honour an `advance_step` from the previous step
/// straight into the new one.
///
/// Returns `Some(patched)` on success, `None` if the input shape didn't
/// match — the caller logs and keeps the original bytes unmodified
/// rather than risk shipping a corrupted XML entry.
pub fn apply_override(original: &[u8], ov: &MissionOverride) -> Option<Vec<u8>> {
    const STEPS_CLOSE: &str = "</Steps>";

    let xml = std::str::from_utf8(original).ok()?;

    // Locate `<Steps StepID="<after_id>"`. The attribute order in the
    // canonical PAK puts StepEnabled before StepID, so we scan for the
    // StepID="N" substring instead of anchoring on the open tag.
    let needle = format!("StepID=\"{}\"", ov.insert_after_step_id);
    let step_attr_idx = xml.find(&needle)?;
    // Find the closing `</Steps>` of that step. XML structure is flat —
    // `<Steps>` children of `<COOKED_MISSION>` don't nest — so the next
    // close after the StepID attribute is the matching one.
    let close_offset_in_remainder = xml[step_attr_idx..].find(STEPS_CLOSE)?;
    let insert_idx = step_attr_idx + close_offset_in_remainder + STEPS_CLOSE.len();

    let mut out = Vec::with_capacity(original.len() + ov.injected_steps_xml.len());
    out.extend_from_slice(&original[..insert_idx]);
    out.extend_from_slice(ov.injected_steps_xml.as_bytes());
    out.extend_from_slice(&original[insert_idx..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_622: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <COOKED_MISSION MissionID=\"622\">\
        <Steps StepEnabled=\"false\" StepID=\"2113\" AwardXP=\"false\" Difficulty=\"1\">\
        <StepDisplayLogText>Search</StepDisplayLogText></Steps>\
        </COOKED_MISSION>";

    /// 641-shape sample: a multi-step mission so we can pin where the
    /// new step lands in the XML stream relative to other existing steps
    /// (load-bearing for the client's index-based progression).
    const SAMPLE_641: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <COOKED_MISSION MissionID=\"641\">\
        <Steps StepEnabled=\"false\" StepID=\"2121\" AwardXP=\"false\" Difficulty=\"1\">\
        <StepDisplayLogText>Prepare</StepDisplayLogText></Steps>\
        <Steps StepEnabled=\"false\" StepID=\"3563\" AwardXP=\"false\" Difficulty=\"1\">\
        <StepDisplayLogText>Speak</StepDisplayLogText></Steps>\
        <Steps StepEnabled=\"false\" StepID=\"3564\" AwardXP=\"false\" Difficulty=\"1\">\
        <StepDisplayLogText>Terminal</StepDisplayLogText></Steps>\
        </COOKED_MISSION>";

    #[test]
    fn override_inserts_immediately_after_named_step() {
        let ov = &MISSION_OVERRIDES[0];
        assert_eq!(ov.mission_id, 622);

        let patched = apply_override(SAMPLE_622, ov).expect("override must apply");
        let s = std::str::from_utf8(&patched).expect("output is utf-8");

        assert!(s.contains("StepID=\"2113\""), "original step missing: {s}");
        assert!(s.contains("StepID=\"80622\""), "new step missing: {s}");
        let new_steps_at = s.find("StepID=\"80622\"").unwrap();
        let close_at = s.find("</COOKED_MISSION>").unwrap();
        assert!(
            new_steps_at < close_at,
            "injected steps must precede closing tag",
        );
    }

    /// Load-bearing test for the P90 fix: the new step must sit between
    /// step 2121 and step 3563 in the XML stream so its index is 1 (one
    /// past 2121), not 3 (past 3563/3564). The client's mission-state
    /// machine uses XML order as the index and snaps the displayed step
    /// forward sequentially when an advance jumps the index by more
    /// than one — that was the bug that left the P90 step invisible.
    #[test]
    fn override_641_lands_between_2121_and_3563() {
        let ov = MISSION_OVERRIDES
            .iter()
            .find(|o| o.mission_id == 641)
            .expect("641 override must be registered");
        assert_eq!(
            ov.insert_after_step_id, 2121,
            "641 override must follow step 2121 to keep index = 1",
        );

        let patched = apply_override(SAMPLE_641, ov).expect("override must apply");
        let s = std::str::from_utf8(&patched).expect("output is utf-8");

        let pos_2121 = s
            .find("StepID=\"2121\"")
            .expect("step 2121 must remain in patched XML");
        let pos_80641 = s
            .find("StepID=\"80641\"")
            .expect("new step 80641 must appear in patched XML");
        let pos_3563 = s
            .find("StepID=\"3563\"")
            .expect("step 3563 must remain in patched XML");
        let pos_3564 = s
            .find("StepID=\"3564\"")
            .expect("step 3564 must remain in patched XML");

        assert!(
            pos_2121 < pos_80641 && pos_80641 < pos_3563 && pos_3563 < pos_3564,
            "XML order must be 2121 → 80641 → 3563 → 3564; got positions \
             2121={pos_2121} 80641={pos_80641} 3563={pos_3563} 3564={pos_3564}\n{s}",
        );
    }

    #[test]
    fn override_returns_none_on_malformed_xml() {
        // No matching StepID="2113" — apply_override must refuse rather than guess.
        let bad = b"<?xml?><COOKED_MISSION MissionID=\"622\"></COOKED_MISSION>";
        let ov = &MISSION_OVERRIDES[0];
        assert!(apply_override(bad, ov).is_none());
    }

    #[test]
    fn override_641_targets_p90() {
        let ov = MISSION_OVERRIDES
            .iter()
            .find(|o| o.mission_id == 641)
            .expect("641 override must be registered");
        assert!(
            ov.injected_steps_xml.contains("StepID=\"80641\""),
            "P90 override must use step 80641",
        );
        assert!(
            ov.injected_steps_xml.contains("Equip the P90"),
            "P90 override must mention P90",
        );
    }

    /// Objective `<DisplayLogText>` must be a single space, not the full
    /// step text. The original game's mission XML uses the step's
    /// `<StepDisplayLogText>` for the visible line; the objective row is
    /// the in-step checkbox/marker and is intentionally blank. Putting
    /// the real text on both surfaces as a visibly duplicated line in
    /// the live mission log (regression observed on the Frost quest UI).
    #[test]
    fn objective_display_text_is_blank_to_avoid_double_render() {
        for ov in MISSION_OVERRIDES {
            assert!(
                ov.injected_steps_xml
                    .contains("<DisplayLogText> </DisplayLogText>"),
                "mission {} override must use blank DisplayLogText (single \
                 space) for the objective row to match the original game's \
                 step/objective duplication-avoidance pattern; got: {}",
                ov.mission_id,
                ov.injected_steps_xml,
            );
        }
    }
}
