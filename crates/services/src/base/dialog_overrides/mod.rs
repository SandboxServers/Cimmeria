//! Cimmeria-side overrides for `CookedDataDialogs.pak` entries.
//!
//! A dialog's player-visible body text lives in the `_<dialogId>` entry
//! inside `CookedDataDialogs.pak` on the client — **not** in any wire
//! message or in the server's `dialogs` / `dialog_screens` tables. The
//! server's `displayDialog` path only carries the dialog *id*; the client
//! looks the screen text, the window type and the buttons up from its own
//! cooked catalogue. So editing a row in
//! `db/resources/Dialogs/Seed/dialog_screens.sql` has zero in-game effect
//! on what renders — and a brand-new dialog id the client has never seen
//! renders as an empty box.
//!
//! Same trick as [`super::mission_overrides`] / [`super::item_overrides`]:
//! patch the catalogue in memory at server startup and lean on the
//! cooked-data wire path (`versionInfoRequest` → `onVersionInfo(InvalidKeys
//! =[...])` → `resourceFragment(_key, patched XML)`) to push the entry to
//! the client. Self-healing — runtime cache invalidation, no manual client
//! steps, no on-disk PAK edit, no client-artifact redistribution.
//!
//! # Two override kinds
//!
//! **Full regeneration** — [`DialogOverride`], this file. Emits a complete
//! `<COOKED_DIALOG>` from Rust-authored text. Right for a dialog Cimmeria
//! invented, where there is no canonical entry to preserve: the new-entry
//! case (the Guard corpse's 3996, which the PAK never shipped) and the
//! corrected-entry case (Frost's 3995) are both just an
//! `elements.insert(dialog_id, generated_xml)`.
//!
//! **Patch** — [`DialogPatch`], in [`patch`]. Parses the entry the client
//! already shipped, edits only what the plan names, and re-emits. Right for
//! one of the 5,405 shipped dialogs: restating tens of screens of voiced
//! dialogue in Rust to move one button would be a transcription error
//! waiting to happen. Every screen's speaker, id and text come out
//! byte-identical to the source, `&#xA;` newline references and all.
//!
//! Both kinds funnel through the one emitter in [`emit`], so they cannot
//! drift into two different on-the-wire shapes. The emitted XML follows the
//! Server-Build conventions in `docs/engine/cooked-data-pak-format.md`: no
//! SOAP namespaces, alphabetised root attributes, `<Screens>` children with
//! `SpeakerID` → `ScreenID` → `Text` and an explicit close, and nested
//! `<Buttons ButtonType ButtonID Text></Buttons>`.
//!
//! # Buttons decide how a dialog closes
//!
//! Closing a dialog with ZERO buttons sends `dialogButtonChoice(id, -1)`;
//! closing one with ANY button sends nothing. That `-1` is what most
//! progression chains actually run on, so whether a dialog carries a button
//! is a gameplay decision, not a cosmetic one. See
//! `docs/analysis/dialog-ui-redesign/work-packets.md` for the client
//! contract this module implements.

pub mod emit;
pub mod parse;
pub mod patch;
mod patches_castle;
mod patches_cellblock;

#[cfg(test)]
mod patch_seed_agreement_castle;
#[cfg(test)]
mod patch_tests;

pub use emit::{emit_cooked_dialog, escape_xml_attr, CookedButton, CookedDialog, CookedScreen};
pub use patch::{apply_dialog_patches, no_patches_registered, DialogPatch, DIALOG_PATCH_TABLES};

/// One button on an authored screen.
///
/// `button_id` is the cooked id the client puts on the wire when the
/// button is clicked, not the button's index: 8 Accept, 9 More Info,
/// 70 Receive Item, 71 Take Missions. `button_type` picks the widget:
/// 1 More Info, 2 Accept, 3 Decline, 4/5/6 Generic1-3.
///
/// Order within [`DialogScreen::buttons`] is significant — the client
/// resolves a click to a position in the array and sends whichever
/// `ButtonID` sits there.
pub struct DialogButton {
    pub button_type: u32,
    pub button_id: u32,
    /// Raw, human-readable label. XML escaping is applied by the emitter.
    pub text: &'static str,
}

/// One screen line within a dialog. `text` is the raw, human-readable
/// string — XML escaping (`"` → `&quot;`, `&` → `&amp;`, `<`/`>`) is
/// applied by the emitter, so callers write the text exactly as it
/// should read in-game.
pub struct DialogScreen {
    pub screen_id: u32,
    /// Speaker entity id. `0` for a system/narrator line (the search-body
    /// dialogs are narrator text, not spoken by an NPC).
    pub speaker_id: u32,
    pub text: &'static str,
    /// Buttons offered on this screen, in the order the client should see
    /// them. Empty means the dialog closes with `dialogButtonChoice(id,
    /// -1)`, which is what the two search-corpse dialogs below rely on.
    pub buttons: &'static [DialogButton],
}

/// One dialog's override: the dialog id to (re)generate and its screen
/// lines. Applies to both a corrected existing dialog and a brand-new one
/// the canonical PAK never shipped.
///
/// `dialog_id` is the cooked catalogue key (`_<dialog_id>` in the PAK),
/// the same id carried by the server's `displayDialog` path and stored in
/// `db/resources/Dialogs/Seed/dialogs.sql`.
///
/// `ui_screen_type` mirrors the `dialogs.ui_screen_type` enum value the
/// client expects; `2` is `DUIST_DefaultDialog` (the plain text box used
/// by the search-corpse dialogs — see `entities/defs/enumerations.xml`).
///
/// To change one of the dialogs the game shipped, reach for
/// [`DialogPatch`] instead.
pub struct DialogOverride {
    pub dialog_id: u32,
    pub dialog_flags: u32,
    pub kismet_event_set_id: u32,
    pub ui_screen_type: u32,
    pub screens: &'static [DialogScreen],
}

/// All Cimmeria-introduced dialog overrides for the `CookedDataDialogs`
/// category (id `5`). Adding a new entry here:
///
///   1. Insert/patch the row(s) in
///      `db/resources/Dialogs/Seed/dialogs.sql` and
///      `db/resources/Dialogs/Seed/dialog_screens.sql` so the server-side
///      catalogue agrees (documentary + used by any server-side log/debug
///      string — the client renders from this override, not the DB).
///   2. Add a `DialogOverride` here so the client's UI actually renders
///      the text via the per-key invalidation handshake.
///   3. Bind the dialog to its template via a `dialog_set_maps` row (and,
///      for a new clickable corpse, a content-chain `add_dialog_set` +
///      `set_interaction_type`).
///
/// The override text and the `dialog_screens.sql` text must be kept in
/// sync by hand — the override is the source of truth for what the player
/// sees; the seed row is the canonical record for committing to the repo.
pub const DIALOG_OVERRIDES: &[DialogOverride] = &[
    // Mission 622 "Arm Yourself!" — Frost's corpse (dialog 3995). The
    // canonical PAK never shipped 3995 (it's a Cimmeria-added dialog), and
    // after the loot split Frost grants ONLY the letter, so the body text
    // must not mention the pistol. Regenerated here, letter-only.
    DialogOverride {
        dialog_id: 3995,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 96108,
            speaker_id: 0,
            text: "There are no obvious wounds on Cpl. Frost, though it appears he died \
                   soon after killing that NID Guard. On his body you find a letter \
                   addressed to \"Jess\" - Frost's wife.",
            // Zero buttons: the search chain fires off the `-1` the client
            // sends when a button-less dialog is closed (fact F8).
            buttons: &[],
        }],
    },
    // Mission 622 loot split — the NID Guard's corpse (dialog 3996). A
    // brand-new dialog the PAK never shipped; bound to template 21 via
    // chain 1001's `add_dialog_set`. Searching the Guard yields the pistol
    // (chain 1005) — this is the immersion text shown on that search.
    DialogOverride {
        dialog_id: 3996,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 96109,
            speaker_id: 0,
            text: "The NID Guard died with his weapon still drawn. You pry the pistol \
                   from his grip - it's still serviceable.",
            buttons: &[],
        }],
    },
];

/// Build the emitter's model from an authored override. Raw text is
/// escaped here; everything downstream deals in escaped values.
fn cooked_from_override(ov: &DialogOverride) -> CookedDialog {
    CookedDialog {
        dialog_flags: ov.dialog_flags,
        dialog_id: ov.dialog_id,
        kismet_event_set_id: ov.kismet_event_set_id,
        ui_screen_type: ov.ui_screen_type,
        screens: ov
            .screens
            .iter()
            .map(|screen| CookedScreen {
                speaker_id: screen.speaker_id,
                screen_id: Some(screen.screen_id),
                text_escaped: escape_xml_attr(screen.text),
                buttons: screen
                    .buttons
                    .iter()
                    .map(|b| CookedButton {
                        button_type: b.button_type,
                        button_id: b.button_id,
                        text_escaped: escape_xml_attr(b.text),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Generate the full Server-Build `<COOKED_DIALOG>` XML bytes for one
/// override. Always succeeds — there's no canonical entry to anchor
/// against, we emit the complete document from the struct fields.
pub fn generate_dialog_xml(ov: &DialogOverride) -> Vec<u8> {
    emit_cooked_dialog(&cooked_from_override(ov))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generated XML must be well-formed Server-Build shape: no SOAP
    /// namespaces, alphabetized root attributes, explicit `</Screens>` and
    /// `</COOKED_DIALOG>` closes.
    #[test]
    fn generated_xml_has_server_build_shape() {
        let ov = &DIALOG_OVERRIDES[0];
        let bytes = generate_dialog_xml(ov);
        let s = std::str::from_utf8(&bytes).expect("generated XML is utf-8");

        assert!(s.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(
            s.contains(
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"3995\" \
                 KismetEventSetID=\"0\" UIScreenType=\"2\">"
            ),
            "root attributes must be alphabetized Server-Build order: {s}",
        );
        assert!(
            !s.contains("SOAP-ENV") && !s.contains("xmlns"),
            "Server-Build XML must not carry SOAP namespaces: {s}",
        );
        assert!(
            s.contains("</Screens>"),
            "explicit Screens close required: {s}"
        );
        assert!(
            s.ends_with("</COOKED_DIALOG>"),
            "explicit root close required: {s}"
        );
    }

    /// The Frost text quotes "Jess" — those double quotes MUST be escaped
    /// to `&quot;` or the attribute terminates early and the client either
    /// truncates the line or rejects the entry. Load-bearing for 3995.
    #[test]
    fn frost_dialog_escapes_embedded_quotes() {
        let ov = DIALOG_OVERRIDES
            .iter()
            .find(|o| o.dialog_id == 3995)
            .expect("Frost dialog 3995 must be registered");
        let bytes = generate_dialog_xml(ov);
        let s = std::str::from_utf8(&bytes).expect("utf-8");

        assert!(
            s.contains("&quot;Jess&quot;"),
            "embedded quotes around Jess must be escaped to &quot;: {s}",
        );
        // The raw, unescaped `Text="..."Jess"..."` must never appear — that
        // would be a prematurely-terminated attribute.
        assert!(
            !s.contains("\"Jess\""),
            "no raw unescaped double-quoted Jess may survive into the attribute: {s}",
        );
    }

    /// Frost (3995) must be letter-only after the loot split — no mention
    /// of the pistol. The pistol lives on the Guard corpse (3996). Pins the
    /// text contract that mirrors the chain-level loot split.
    #[test]
    fn frost_dialog_is_letter_only_no_pistol() {
        let ov = DIALOG_OVERRIDES
            .iter()
            .find(|o| o.dialog_id == 3995)
            .expect("Frost dialog 3995 must be registered");
        let body = ov.screens[0].text.to_lowercase();
        assert!(
            body.contains("letter"),
            "Frost dialog must mention the letter"
        );
        assert!(
            !body.contains("pistol"),
            "Frost dialog must NOT mention the pistol after the loot split — \
             that belongs to the Guard corpse (3996)",
        );
    }

    /// The Guard corpse (3996) is the pistol-search immersion dialog. Pins
    /// that it's registered and mentions the pistol.
    #[test]
    fn guard_dialog_mentions_pistol() {
        let ov = DIALOG_OVERRIDES
            .iter()
            .find(|o| o.dialog_id == 3996)
            .expect("Guard dialog 3996 must be registered");
        assert!(
            ov.screens[0].text.to_lowercase().contains("pistol"),
            "Guard corpse dialog must mention the pistol for immersion",
        );
    }

    /// Every registered override generates a `Text="..."` whose ScreenID
    /// and DialogID land in the output. Guards against a future entry whose
    /// fields don't make it into the emitted XML.
    #[test]
    fn all_overrides_emit_their_ids_and_text() {
        for ov in DIALOG_OVERRIDES {
            let bytes = generate_dialog_xml(ov);
            let s = std::str::from_utf8(&bytes).expect("utf-8");
            assert!(
                s.contains(&format!("DialogID=\"{}\"", ov.dialog_id)),
                "override {} must emit its DialogID",
                ov.dialog_id,
            );
            for screen in ov.screens {
                assert!(
                    s.contains(&format!("ScreenID=\"{}\"", screen.screen_id)),
                    "override {} must emit ScreenID {}",
                    ov.dialog_id,
                    screen.screen_id,
                );
            }
        }
    }

    /// Byte-exact pin on 3995. The two shipped overrides predate the
    /// patch engine; routing them through the shared emitter must not
    /// have moved a single byte, or every client that already cached
    /// them refetches for nothing.
    #[test]
    fn frost_override_bytes_are_unchanged_by_the_shared_emitter() {
        let ov = DIALOG_OVERRIDES
            .iter()
            .find(|o| o.dialog_id == 3995)
            .expect("Frost dialog 3995 must be registered");
        assert_eq!(
            String::from_utf8(generate_dialog_xml(ov)).unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <COOKED_DIALOG DialogFlags=\"0\" DialogID=\"3995\" KismetEventSetID=\"0\" \
             UIScreenType=\"2\">\
             <Screens SpeakerID=\"0\" ScreenID=\"96108\" \
             Text=\"There are no obvious wounds on Cpl. Frost, though it appears he died \
             soon after killing that NID Guard. On his body you find a letter addressed to \
             &quot;Jess&quot; - Frost&apos;s wife.\"></Screens>\
             </COOKED_DIALOG>",
        );
    }

    /// An authored override CAN now carry buttons, which is what lets a
    /// future Cimmeria-invented dialog offer one. The two shipped entries
    /// deliberately do not.
    #[test]
    fn authored_buttons_reach_the_emitted_xml() {
        const WITH_BUTTON: DialogOverride = DialogOverride {
            dialog_id: 4242,
            dialog_flags: 0,
            kismet_event_set_id: 0,
            ui_screen_type: 2,
            screens: &[DialogScreen {
                screen_id: 1,
                speaker_id: 0,
                text: "Well?",
                buttons: &[DialogButton {
                    button_type: 2,
                    button_id: 8,
                    text: "Accept",
                }],
            }],
        };
        assert_eq!(
            String::from_utf8(generate_dialog_xml(&WITH_BUTTON)).unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <COOKED_DIALOG DialogFlags=\"0\" DialogID=\"4242\" KismetEventSetID=\"0\" \
             UIScreenType=\"2\">\
             <Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"Well?\">\
             <Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"Accept\"></Buttons>\
             </Screens>\
             </COOKED_DIALOG>",
        );
    }

    /// The two shipped overrides must keep zero buttons. A button here
    /// would stop the client sending `dialogButtonChoice(id, -1)` on
    /// close, and the mission 622 search chains key on exactly that.
    #[test]
    fn shipped_overrides_carry_no_buttons() {
        for ov in DIALOG_OVERRIDES {
            for screen in ov.screens {
                assert!(
                    screen.buttons.is_empty(),
                    "dialog {} screen {} must stay button-less: the search chains \
                     depend on the close sending -1 (client contract F8)",
                    ov.dialog_id,
                    screen.screen_id,
                );
            }
        }
    }
}
