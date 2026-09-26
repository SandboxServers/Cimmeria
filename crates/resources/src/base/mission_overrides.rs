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
    // Mission 622's loot split is sequenced Frost → Guard via an intermediate
    // step, so the corpse search survives a relog: each step gates the next
    // body's binding (re-applied on login by chains 1006/1007). Two
    // Cimmeria-introduced steps sit between the canonical step 2113 and the
    // equip step, in XML index order:
    //   index 0: 2113   (canonical) — search Cpl. Frost
    //   index 1: 80623  (below)     — search the NID Guard's corpse
    //   index 2: 80622  (below)     — equip the pistol
    // The two overrides MUST stay in this array order: 80623 is injected after
    // 2113 first, so the 80622 entry can anchor on 80623's `</Steps>`. Every
    // advance is +1 in XML index (2113→80623, 80623→80622, 80622→complete),
    // which the client's sequential-progression guard honours.
    MissionOverride {
        mission_id: 622,
        insert_after_step_id: 2113,
        injected_steps_xml:
            "<Steps StepEnabled=\"false\" StepID=\"80623\" AwardXP=\"false\" Difficulty=\"1\">\
             <StepDisplayLogText>Search the NID Guard's body for a weapon.</StepDisplayLogText>\
             <Objectives IsOptional=\"false\" ObjectiveID=\"90623\" AwardXP=\"false\" \
             IsHidden=\"false\" IsEnabled=\"false\" Difficulty=\"1\">\
             <DisplayLogText> </DisplayLogText>\
             </Objectives>\
             </Steps>",
    },
    MissionOverride {
        mission_id: 622,
        // Anchor on 80623 (injected by the entry above), not 2113 — the equip
        // step must land at XML index 2, one past the Guard-search step.
        insert_after_step_id: 80623,
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
    MissionOverride {
        mission_id: 688,
        // Insert immediately after step 2356 ("Secure the Castle's main
        // armory for Op-CORE."). Splits mission 688 into two phases so
        // chain 1107 can advance from 2356 → 80688 on terminal-use
        // (rather than completing obj 2734 directly, which would trip
        // the cell::missions::complete_objective auto-complete and end
        // the mission before chain 1109 ever fires on the ring switch).
        // Step 2356 has only one required objective (2734); without a
        // second step, the mission has nowhere to go after that
        // objective is marked complete.
        insert_after_step_id: 2356,
        injected_steps_xml:
            "<Steps StepEnabled=\"false\" StepID=\"80688\" AwardXP=\"false\" Difficulty=\"1\">\
             <StepDisplayLogText>Use the ring transport to escape.</StepDisplayLogText>\
             <Objectives IsOptional=\"false\" ObjectiveID=\"90688\" AwardXP=\"false\" \
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
    const STEPS_CLOSE: &str = "</Steps>";

    let xml = std::str::from_utf8(original).ok()?;

    // Anchor on the StepID attribute (canonical attribute order puts
    // StepEnabled first; scanning by substring keeps us insensitive to
    // attribute reordering).
    let step_needle = format!("StepID=\"{}\"", ov.step_id);
    let step_attr_idx = xml.find(&step_needle)?;

    // Bound the search to THIS step's `<Steps>` block. If the matched
    // step is missing a `<StepDisplayLogText>` (XML drift, stripped
    // child element, etc.), an unbounded scan would patch the NEXT
    // step's caption instead of failing. Find the closing `</Steps>`
    // for the matched StepID first; only accept a `<StepDisplayLogText>`
    // located before that boundary.
    let after_step = &xml[step_attr_idx..];
    let steps_close_offset = after_step.find(STEPS_CLOSE)?;
    let open_at = after_step.find(OPEN)?;
    if open_at >= steps_close_offset {
        // The matched step doesn't contain a StepDisplayLogText — the
        // next OPEN is in a sibling step. Refuse rather than patch the
        // wrong step. Return None so the caller logs and keeps the
        // original bytes unmodified (same defensive shape as the
        // upper-level `apply_override`).
        return None;
    }
    let inner_start = step_attr_idx + open_at + OPEN.len();
    let close_offset = after_step[open_at + OPEN.len()..].find(CLOSE)?;
    // Same boundary check on the closing tag — guards against a
    // `<StepDisplayLogText>` that opens inside our step but has its
    // closing tag rewritten or stripped, which would otherwise
    // consume bytes from a sibling step.
    if open_at + OPEN.len() + close_offset >= steps_close_offset {
        return None;
    }
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

    /// Mission 622 now injects TWO steps (80623 Guard-search, then 80622
    /// equip) so the loot split is sequenced and relog-safe. Applying the 622
    /// overrides in array order against the canonical single-step shape must
    /// yield XML order 2113 → 80623 → 80622 — the same index-discipline the
    /// 641 test pins, since the client treats XML order as the step index and
    /// snaps forward on any advance that jumps the index by more than one.
    #[test]
    fn override_622_injects_guard_then_equip_in_order() {
        let ovs_622: Vec<&MissionOverride> = MISSION_OVERRIDES
            .iter()
            .filter(|o| o.mission_id == 622)
            .collect();
        assert_eq!(
            ovs_622.len(),
            2,
            "mission 622 must have exactly two overrides (Guard-search + equip)",
        );

        // Apply both in registry order, compounding on the same XML.
        let mut bytes = SAMPLE_622.to_vec();
        for ov in &ovs_622 {
            bytes = apply_override(&bytes, ov).unwrap_or_else(|| {
                panic!(
                    "622 override (after step {}) must apply",
                    ov.insert_after_step_id
                )
            });
        }
        let s = std::str::from_utf8(&bytes).expect("output is utf-8");

        let pos_2113 = s.find("StepID=\"2113\"").expect("step 2113 must remain");
        let pos_80623 = s
            .find("StepID=\"80623\"")
            .expect("Guard-search step 80623 must appear");
        let pos_80622 = s
            .find("StepID=\"80622\"")
            .expect("equip step 80622 must appear");
        let close_at = s.find("</COOKED_MISSION>").expect("root close must remain");

        assert!(
            pos_2113 < pos_80623 && pos_80623 < pos_80622 && pos_80622 < close_at,
            "XML order must be 2113 → 80623 → 80622 → </COOKED_MISSION>; got \
             2113={pos_2113} 80623={pos_80623} 80622={pos_80622} close={close_at}\n{s}",
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

    /// The matched `<Steps>` block doesn't have a `<StepDisplayLogText>`,
    /// but the *next* sibling step does. `apply_step_text_override` must
    /// refuse rather than reach across the `</Steps>` boundary and patch
    /// the next step's caption — that would silently corrupt unrelated
    /// mission UI text. Pins the bounded-scan defensive posture.
    #[test]
    fn step_text_override_returns_none_when_matched_step_has_no_caption() {
        // Step 5000 is the matched StepID; it has no <StepDisplayLogText>.
        // Step 5001 (sibling) does — an unbounded scan would patch it.
        let xml = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
            <COOKED_MISSION MissionID=\"999\">\
            <Steps StepEnabled=\"false\" StepID=\"5000\" AwardXP=\"false\" Difficulty=\"1\">\
            <Objectives ObjectiveID=\"1\"><DisplayLogText> </DisplayLogText></Objectives>\
            </Steps>\
            <Steps StepEnabled=\"false\" StepID=\"5001\" AwardXP=\"false\" Difficulty=\"1\">\
            <StepDisplayLogText>Sibling caption</StepDisplayLogText></Steps>\
            </COOKED_MISSION>";
        let ov = StepTextOverride {
            mission_id: 999,
            step_id: 5000,
            new_step_display_log_text: "should not apply",
        };
        assert!(
            apply_step_text_override(xml, &ov).is_none(),
            "matched step has no <StepDisplayLogText>; the override must not \
             cross the </Steps> boundary into the next step",
        );
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
