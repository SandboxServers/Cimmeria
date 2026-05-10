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
//! and use the protocol's existing per-key invalidation channel: the
//! `onVersionInfo` packet carries an `InvalidKeys` ARRAY<u32>, which the
//! client (`ServerConnection::onVersionInfo`) reads and uses to drop only
//! the named entries from its local cache. The client does **not** then
//! send `elementDataRequest` for the invalidated keys — it waits for the
//! server to push them. Our `handle_version_info_request` does that push
//! immediately after the `onVersionInfo` reply, with `RequiredUpdates`
//! set to the InvalidKeys count so the client knows how many fragments
//! to expect.
//!
//! This module's job: produce the patched XML bytes for missions that
//! Cimmeria adds steps to. The byte layout follows the QA-build conventions
//! documented in [`docs/engine/cooked-data-pak-format.md`] — same
//! `<COOKED_MISSION>` root, same `<Steps>`/`<Objectives>` children, same
//! attribute style. The new `<Steps>` block goes immediately after the
//! closing `</Steps>` of the named anchor step (see `insert_after_step_id`
//! on `MissionOverride`), since the client uses XML declaration order as
//! the step index and a step appended past every other `</Steps>` reads
//! as a multi-step skip on advance.

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

/// In-place patch of `<StepDisplayLogText>` on an already-shipped step in
/// the canonical PAK. Used to fix wrong player-visible text on existing
/// steps without inserting new ones.
///
/// Distinct from [`MissionOverride`] because the existing `<Steps>` block
/// is preserved (StepID, ObjectiveID, IsHidden flags untouched) — only
/// the inner display text changes. Inserting a new step would shift XML
/// indexes and break the client's sequential-progression guard the same
/// way an appended `<Steps>` does for new-step inserts.
pub struct StepTextOverride {
    pub mission_id: u32,
    pub step_id: u32,
    pub new_step_display_log_text: &'static str,
}

/// Step-text overrides applied alongside [`MISSION_OVERRIDES`] at PAK
/// load. Each one rewrites the inner text of an existing `<Steps
/// StepID="N">` `<StepDisplayLogText>` element.
pub const STEP_TEXT_OVERRIDES: &[StepTextOverride] = &[
    // Mission 639 step 2343: the canonical PAK ships
    //   "press 'i' to open inventory"
    // but the live SGW client uses 'b' to open inventory; the player must
    // then select Mission Inventory from the tabs. QA pass 2026-05-09
    // surfaced the misdirection — players can't progress past the stasis-
    // sickness step without the right key. Server-side parallel text in
    // `db/resources/Missions/Seed/mission_steps.sql:5810` and
    // `db/resources/Texts/Seed/texts.sql` moniker 13971 are kept in sync
    // so server-side log/debug strings match what the client renders.
    StepTextOverride {
        mission_id: 639,
        step_id: 2343,
        new_step_display_log_text: "Use the Ambernol in your mission inventory (press 'b' to open \
             inventory, then select Mission Inventory) to cure yourself of \
             Stasis Sickness.",
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

/// Apply a single [`StepTextOverride`] to a chunk of XML bytes from the PAK.
///
/// Locates `<Steps ... StepID="<step_id>" ...>` then the first
/// `<StepDisplayLogText>` inside it, and replaces the inner text. The
/// surrounding `<Steps>` attributes (StepEnabled, AwardXP, Difficulty) and
/// any `<Objectives>` children are preserved verbatim — only the visible
/// step caption changes.
///
/// Returns `Some(patched)` on success, `None` if the input shape didn't
/// match. As with `apply_override`, the caller logs and keeps the
/// original bytes unmodified rather than risk shipping corrupted XML.
pub fn apply_step_text_override(original: &[u8], ov: &StepTextOverride) -> Option<Vec<u8>> {
    const OPEN: &str = "<StepDisplayLogText>";
    const CLOSE: &str = "</StepDisplayLogText>";

    let xml = std::str::from_utf8(original).ok()?;

    // Anchor on the StepID attribute (canonical attribute order puts
    // StepEnabled first; scanning by substring keeps us insensitive to
    // attribute reordering).
    let step_needle = format!("StepID=\"{}\"", ov.step_id);
    let step_attr_idx = xml.find(&step_needle)?;

    // The first <StepDisplayLogText> after the StepID attribute is the
    // one belonging to this step's <Steps> block (children don't nest;
    // see `apply_override`).
    let after_step = &xml[step_attr_idx..];
    let open_at = after_step.find(OPEN)?;
    let inner_start = step_attr_idx + open_at + OPEN.len();
    let close_offset = after_step[open_at + OPEN.len()..].find(CLOSE)?;
    let inner_end = step_attr_idx + open_at + OPEN.len() + close_offset;

    let mut out = Vec::with_capacity(original.len() + ov.new_step_display_log_text.len());
    out.extend_from_slice(&original[..inner_start]);
    out.extend_from_slice(ov.new_step_display_log_text.as_bytes());
    out.extend_from_slice(&original[inner_end..]);
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

    /// 639-shape sample for the in-place text replacement path. The wrong
    /// "press 'i'" caption mirrors what the canonical PAK ships for step 2343.
    const SAMPLE_639_AMBERNOL: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
        <COOKED_MISSION MissionID=\"639\">\
        <Steps StepEnabled=\"false\" StepID=\"2117\" AwardXP=\"false\" Difficulty=\"1\">\
        <StepDisplayLogText>Look</StepDisplayLogText></Steps>\
        <Steps StepEnabled=\"false\" StepID=\"2343\" AwardXP=\"false\" Difficulty=\"1\">\
        <StepDisplayLogText>Use the Ambernol in your mission inventory \
(press 'i') to cure yourself of Stasis Sickness.</StepDisplayLogText></Steps>\
        </COOKED_MISSION>";

    /// Step text override rewrites only the matched step's caption and
    /// preserves surrounding XML. This is the load-bearing test for the
    /// Ambernol "press 'i' → press 'b'" fix.
    #[test]
    fn step_text_override_rewrites_caption_and_preserves_other_steps() {
        let ov = STEP_TEXT_OVERRIDES
            .iter()
            .find(|o| o.mission_id == 639 && o.step_id == 2343)
            .expect("639/2343 step text override must be registered");

        let patched = apply_step_text_override(SAMPLE_639_AMBERNOL, ov)
            .expect("step text override must apply against canonical XML shape");
        let s = std::str::from_utf8(&patched).expect("output is utf-8");

        assert!(
            !s.contains("press 'i'"),
            "patched XML must no longer contain the wrong 'press 'i'' caption: {s}"
        );
        assert!(
            s.contains("press 'b'"),
            "patched XML must contain the corrected 'press 'b'' caption: {s}"
        );
        assert!(
            s.contains("StepID=\"2117\"")
                && s.contains("<StepDisplayLogText>Look</StepDisplayLogText>"),
            "step 2117 caption must be untouched: {s}"
        );
        assert!(
            s.contains("StepID=\"2343\""),
            "step 2343 attributes must remain intact: {s}"
        );
    }

    /// Bad shape (no matching StepID) → returns None. Same defensive
    /// posture as `apply_override` — refusing to guess prevents a
    /// corrupted PAK shipment from a malformed override.
    #[test]
    fn step_text_override_returns_none_on_missing_step() {
        let bad = b"<COOKED_MISSION MissionID=\"639\"></COOKED_MISSION>";
        let ov = StepTextOverride {
            mission_id: 639,
            step_id: 9999,
            new_step_display_log_text: "won't apply",
        };
        assert!(apply_step_text_override(bad, &ov).is_none());
    }

    /// The Ambernol fix entry must be present and reference the correct
    /// step. Pins the override registration so a future refactor of
    /// `STEP_TEXT_OVERRIDES` doesn't silently drop it.
    #[test]
    fn ambernol_step_text_override_is_registered() {
        let ov = STEP_TEXT_OVERRIDES
            .iter()
            .find(|o| o.mission_id == 639 && o.step_id == 2343)
            .expect("639/2343 (Ambernol) step text override must be registered");
        assert!(
            ov.new_step_display_log_text.contains("press 'b'"),
            "Ambernol override must instruct the player to press 'b'",
        );
        assert!(
            !ov.new_step_display_log_text.contains("press 'i'"),
            "Ambernol override must not still reference the wrong 'i' key",
        );
    }
}
