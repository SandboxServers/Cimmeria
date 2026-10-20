//! Castle authored dialog overrides (CS-04aD, CIMMERIA_AUTHORING).
//!
//! Only the shape the client already renders from 3995/3996 is used: one
//! screen, `speaker_id` 0, zero buttons, UI type 2.

use super::{DialogOverride, DialogScreen};

/// Castle medical_q1 dialog 10700: the Medic's monologue, shown by the
/// medical quest after the accept. Frozen in the CS-04aD worknote (Frozen (c)).
pub(super) const MEDICAL_Q1_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10700,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10800,
        speaker_id: 0,
        text: "The medic barely glances up from her work. \"Every cot is full and more wounded keep coming in. Make your way out to the front courtyard and see who still needs help. Watch yourself out there.\"",
        buttons: &[],
    }],
};

/// Castle medical_q2 dialog 10701: the overlooked prisoner's monologue, shown by the
/// medical quest after the accept. Frozen in the CS-04aD2 worknote (Frozen (c)).
pub(super) const MEDICAL_Q2_PRISONER_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10701,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10801,
        speaker_id: 0,
        text: "The prisoner slumps against the wall, pale and shaking. \"Nobody has come down here since the fighting started. I need a medic, please. Tell the one in the infirmary that I am still here.\"",
        buttons: &[],
    }],
};

/// Castle medical_q2 dialog 10702: the Medic's reply, shown by the medical quest at the
/// turn-in. Frozen in the CS-04aD2 worknote (Frozen (c)).
pub(super) const MEDICAL_Q2_MEDIC_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10702,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10802,
        speaker_id: 0,
        text: "The medic nods and pulls a kit from the shelf. \"Someone down in the cells, overlooked? I will send help at once. Thank you for telling me.\"",
        buttons: &[],
    }],
};
/// Castle defense_q1 dialog 10709: the sentry's monologue, shown by the
/// defense quest after the accept. Frozen in the CS-04bD2a worknote (Frozen (c)).
pub(super) const DEFENSE_Q1_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10709,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10817,
        speaker_id: 0,
        text: "The sentry keeps his eyes on the approach as he talks. \"The watch changes at dusk and again before dawn, two to every post. Tell the infirmary the posts are covered, so nobody has to guess when the guards are thin.\"",
        buttons: &[],
    }],
};

/// Castle defense_q3 dialog 10717: Stanton's offer monologue, shown after
/// the accept. Frozen in the CS-04bD2b worknote (Frozen (c)).
pub(super) const DEFENSE_Q3_OFFER_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10717,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10833,
        speaker_id: 0,
        text: "Sergeant Stanton checks the load in his rifle. \"The Bravo officers hold the route and I cannot move a squad past them. Put all four down and the road is ours.\"",
        buttons: &[],
    }],
};

/// Castle defense_q3 dialog 10718: the completion monologue, shown when the
/// fourth objective completes the mission.
pub(super) const DEFENSE_Q3_DONE_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10718,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10834,
        speaker_id: 0,
        text: "The last officer falls and the Bravo route goes quiet. Sergeant Stanton will hold it until the next wave arrives.",
        buttons: &[],
    }],
};

/// Castle return_q1 dialog 10721: Marsh's report monologue, shown by the
/// Marsh click that completes the quest. Frozen in the CS-04cS2 worknote (Frozen (c)).
pub(super) const RETURN_Q1_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10721,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10841,
        speaker_id: 0,
        text: "Colonel Marsh looks up from the gate controls. \"Back from Harset, and in one piece. Good. Everything you saw out there goes into my report. The gate team is not done with you yet - Moh'katan is waiting at the DHD with questions about the address.\"",
        buttons: &[],
    }],
};

/// Castle return_q2 dialog 10725: Moh'katan's confirmation monologue, shown by
/// the nested accept chain. Frozen in the CS-04cS2 worknote (Frozen (c)).
pub(super) const RETURN_Q2_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10725,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10849,
        speaker_id: 0,
        text: "Moh'katan studies the DHD, then turns to you. \"The Harset address is in the gate's memory, but I will not trust it on your word alone. Dial Harset once more, and I will see for myself that the lock holds.\"",
        buttons: &[],
    }],
};

/// Castle repeatable_q1 dialog 10729, the Op-Core soldier's offer monologue:
/// the click accepts the run and shows this. Frozen in the CS-05S3 worknote
/// (Frozen (c)).
pub(super) const REPEATABLE_Q1_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10729,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10857,
        speaker_id: 0,
        text: "The Op-Core soldier looks up from a pile of requisition slips. If he still has a run on file for you, he hands you the stack and points you to the Castle Infirmary: get the slips signed off there, then bring them back. If not, he only shrugs and goes back to his paperwork.",
        buttons: &[],
    }],
};

/// Castle repeatable_q1 dialog 10730, the turn-in monologue shown when the
/// run is completed at the Infirmary. Frozen in the CS-05S3 worknote (Frozen (c)).
pub(super) const REPEATABLE_Q1_TURN_IN_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10730,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10858,
        speaker_id: 0,
        text: "The Op-Core soldier checks the signatures, stamps the slips and drops them in his tray. That clears this batch.",
        buttons: &[],
    }],
};

/// Castle 567 cabinet dialog 10733: the folder monologue on the first cabinet
/// click. Frozen in the CS-08bS worknote (Frozen (c)).
pub(super) const ROMNEY_CABINET_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10733,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10865,
        speaker_id: 0,
        text: "Behind a stack of dusty ledgers you find a grey folder stamped with Romney's seal. These are the files Copplemann wants. Get them off the Castle.",
        buttons: &[],
    }],
};

/// Castle 567 cabinet dialog 10734: the empty-cabinet monologue on every later
/// click. Frozen in the CS-08bS worknote (Frozen (c)).
pub(super) const ROMNEY_CABINET_EMPTY_DIALOG: DialogOverride = DialogOverride {
    dialog_id: 10734,
    dialog_flags: 0,
    kismet_event_set_id: 0,
    ui_screen_type: 2,
    screens: &[DialogScreen {
        screen_id: 10866,
        speaker_id: 0,
        text: "The cabinet holds nothing else of use.",
        buttons: &[],
    }],
};

#[cfg(test)]
mod tests {
    use super::super::DIALOG_OVERRIDES;
    use super::{ROMNEY_CABINET_DIALOG, ROMNEY_CABINET_EMPTY_DIALOG};

    /// Read a dialog seed file with CRLF folded to LF.
    fn read_seed(name: &str) -> String {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../db/resources/Dialogs/Seed")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("seed file {} must be readable: {e}", path.display()))
            .replace("\r\n", "\n")
    }

    fn count_lines(seed: &str, line: &str) -> usize {
        seed.lines().filter(|l| *l == line).count()
    }

    /// Both 567 cabinet monologues are served once, as one screen with
    /// speaker 0 and no buttons, and their text and ids equal the
    /// `dialogs.sql` and `dialog_screens.sql` rows byte for byte (CS-08bS d1).
    /// Revert proof: change one character of either override text, or drop its
    /// `DIALOG_OVERRIDES` entry, and this fails.
    #[test]
    fn castle_567_monologues_are_registered() {
        let dialogs = read_seed("dialogs.sql");
        let screens = read_seed("dialog_screens.sql");
        for ov in [ROMNEY_CABINET_DIALOG, ROMNEY_CABINET_EMPTY_DIALOG] {
            let served = DIALOG_OVERRIDES
                .iter()
                .filter(|o| o.dialog_id == ov.dialog_id)
                .count();
            assert_eq!(served, 1, "dialog {} served once", ov.dialog_id);
            assert_eq!(ov.screens.len(), 1, "one screen");
            let screen = &ov.screens[0];
            assert_eq!(screen.speaker_id, 0, "speaker 0");
            assert!(screen.buttons.is_empty(), "a monologue has no buttons");
            let dialog_row = format!(
                "INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES ({}, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, NULL);",
                ov.dialog_id
            );
            assert_eq!(count_lines(&dialogs, &dialog_row), 1, "dialogs.sql row");
            let screen_row = format!(
                "INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES ({}, {}, '{}', 0, 0);",
                ov.dialog_id,
                screen.screen_id,
                screen.text.replace('\'', "''")
            );
            assert_eq!(
                count_lines(&screens, &screen_row),
                1,
                "dialog_screens.sql row for {} must equal the override text",
                ov.dialog_id
            );
        }
    }

    /// The offer map 1000018 binds dialog 2043 on set 512 with the
    /// non-A-story flag, once (CS-08bS d3). Revert proof: change the map's
    /// dialog and this fails.
    #[test]
    fn castle_567_offer_map_binds_2043() {
        let maps = read_seed("dialog_set_maps.sql");
        let row = "INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (1000018, 512, 2043, 'Romney''s Files', 134217728, 1, '{}', '{}', '{}', '{}');";
        assert_eq!(count_lines(&maps, row), 1, "map 1000018 once");
    }
}
