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
mod override_seed_agreement_debug_hub;
#[cfg(test)]
mod patch_seed_agreement_castle;
#[cfg(test)]
mod patch_seed_agreement_cellblock;
#[cfg(test)]
mod patch_tests;

pub use emit::{emit_cooked_dialog, escape_xml_attr, CookedButton, CookedDialog, CookedScreen};
pub use patch::{apply_dialog_patches, no_patches_registered, DialogPatch, DIALOG_PATCH_TABLES};

/// Largest element id Cimmeria may push as a cooked-data override, in any
/// category. Overrides of dialogs 100100 and 100101 crashed the client on
/// map load; every id the client itself ships is far below this bound. See
/// `docs/reverse-engineering/findings/cooked-dialog-override-crash.md`.
pub const MAX_COOKED_ELEMENT_ID: u32 = u16::MAX as u32;

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
///      catalogue agrees. The client renders from this override, not the
///      DB, but the server does read the seed: a dialog whose screens all
///      have `speaker_id = 0` is a monologue, and `display_dialog` binds the
///      player rather than an NPC as its speaker (`load_monologue_dialog_ids`).
///   2. Add a `DialogOverride` here so the client's UI actually renders
///      the text via the per-key invalidation handshake.
///   3. Bind the dialog to its template via a `dialog_set_maps` row (and,
///      for a new clickable corpse, a content-chain `add_dialog_set` +
///      `set_interaction_type`).
///
/// The override text and the `dialog_screens.sql` text must be kept in
/// sync by hand — the override is the source of truth for what the player
/// sees; the seed row is the canonical record for committing to the repo.
/// The quarantined debug-hub entries ([`QUARANTINED_DIALOG_OVERRIDES`]) are
/// checked against the seed by `override_seed_agreement_debug_hub`.
///
/// Append new entries at the END: tests elsewhere read `DIALOG_OVERRIDES[0]`
/// as the Frost dialog.
///
/// Every `dialog_id` must be at most [`MAX_COOKED_ELEMENT_ID`] (65535).
/// Pushing overrides 100100 and 100101 crashed the 2009 client while it
/// loaded Castle_CellBlock (colo deploy, 2026-09-27). The id's width was
/// first suspected but has been ruled out, and those dialogs are now
/// quarantined (see [`QUARANTINED_DIALOG_OVERRIDES`]). The 16-bit bound
/// stays as a guard. Cimmeria-authored dialogs live in 60100-60199, well above
/// the client's own ids (which stop at 6427). See
/// `docs/reverse-engineering/findings/cooked-dialog-override-crash.md`. The
/// guard is `every_cooked_override_element_id_fits_in_16_bits` in
/// `resources/tests/committed_paks.rs`.
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

/// Cimmeria-authored overrides that are defined but NOT served: nothing
/// reads this list at runtime, so no client is sent these entries.
///
/// **Why they are held back (2026-09-27).** After the debug-hub dialogs
/// started going out as category-5 overrides (as 100100/100101 in the
/// 16:19 UTC colo deploy, then as 60100/60101/60104 in the 17:49 deploy), a
/// tester's client died on every entry into Castle_CellBlock. It crashed
/// after `onClientMapLoad` and never sent `mapLoaded`. SigNoz shows the
/// client writes pushed overrides to its disk cache
/// (`Cache.en-US\CookedDataDialogs.pak`): a session that got no push, only
/// the cached copy, crashed the same way. These dialogs are the only
/// category-5 content that client received between its last good session
/// and the crash. The renumber below 65536 (#938) did not help, and PR #939
/// found the client's dialog cache takes any 32-bit key, so the width of the
/// id is not the fault. Which field is, is still open. The candidates are
/// what these entries have and the long-served 3995/3996 lack: a nonzero
/// speaker (754, 843), more than one screen, screen ids 200000-200005, and
/// button type 4.
///
/// **To restore one**, once RE names the bad field and it is fixed, move
/// the entry back into [`DIALOG_OVERRIDES`] and drop its id from this list.
/// `quarantined_dialogs_are_not_served` fails while an id is in both.
///
/// The seed rows (`dialogs.sql`, `dialog_screens.sql`,
/// `dialog_screen_buttons.sql`) and chains 7001-7003 / 7010-7011 stay. The
/// server still sends `onDialogDisplay` for these ids, but a client without
/// the override has no entry for them, so the NPCs show no dialog.
///
/// Removing these from the served list does NOT clean a client that already
/// cached them: the server evicts nothing. Such a client must delete
/// `Cache.en-US\CookedDataDialogs.pak`.
pub const QUARANTINED_DIALOG_OVERRIDES: &[DialogOverride] = &[
    // NEW CONTENT (debug hub): the stasis-room dialog NPC (template 302,
    // Airman Lance, speaker 754), chains 7001-7003 in debug_hub_chains.sql.
    // Two screens so the player pages with Next; ONE button, on the final
    // screen (hard rule 1), so reading to the end always leaves something to
    // press. Type 4 (Generic 1) because a `DUIST_DefaultDialog` draws only
    // types 2 and 4-6, and type 2 (Accept) would hide the label behind a
    // fixed image and bring an inert Decline.
    DialogOverride {
        dialog_id: 60100,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[
            DialogScreen {
                screen_id: 200000,
                speaker_id: 754,
                text: "Debug hub dialog test. This is screen one of two. Page forward to \
                       reach the button.",
                buttons: &[],
            },
            DialogScreen {
                screen_id: 200001,
                speaker_id: 754,
                text: "Screen two. The button below sends your choice to the server, and the \
                       server answers by opening a second dialog.",
                buttons: &[DialogButton {
                    button_type: 4,
                    button_id: 8,
                    text: "Send my choice",
                }],
            },
        ],
    },
    // NEW CONTENT (debug hub): the answer to 60100's button. Zero buttons on
    // purpose: closing it sends `dialogButtonChoice(60101, -1)` (fact F8),
    // which is the other half of the round trip (chain 7003).
    DialogOverride {
        dialog_id: 60101,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 200002,
            speaker_id: 754,
            text: "Choice received. This dialog has no buttons, so closing it sends -1 to \
                   the server, which answers in your chat window.",
            buttons: &[],
        }],
    },
    // Social-systems campaign, SS-U3: the Gate Mail Clerk (template 390,
    // Sgt. Harriman, speaker 843), chains 7010-7011 in debug_hub_chains.sql.
    // One screen, one Generic 1 button (the 60100 reasoning): pressing it
    // sends `dialogButtonChoice(60104, 8)` and chain 7011 mails the player.
    // Closing with X sends nothing, so nothing is mailed.
    DialogOverride {
        dialog_id: 60104,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 200005,
            speaker_id: 843,
            text: "Gate Mail. I can send you a test mail with a stack of Health Slappacks \
                   and 50 naquadah. Open your mail afterwards to take them. One mail every \
                   10 minutes.",
            buttons: &[DialogButton {
                button_type: 4,
                button_id: 8,
                text: "Send me a mail",
            }],
        }],
    },
    // Bank and Vault campaign, BV-05: the Banker's Expand vault offer
    // (`cimmeria_wire::cell::vault::VAULT_EXPAND_DIALOG_ID`). One screen,
    // one Generic 1 button (the 60100 reasoning). The server shows it beside
    // `onVaultOpen` while the vault is below 100 slots; the answer goes to
    // the purchase path, which re-checks everything. Closing with X sends
    // nothing, so nothing is bought. Speaker 0, because any Banker (or the
    // GM's own entity after `.bank`) speaks it.
    //
    // Authored under quarantine: it shares two of the suspects above (screen
    // id 200010, button type 4), so it is not served until RE names the
    // field. Until then the server still sends `onDialogDisplay(60110)` and
    // the client shows nothing, so no expansion can be bought from the UI.
    DialogOverride {
        dialog_id: 60110,
        dialog_flags: 0,
        kismet_event_set_id: 0,
        ui_screen_type: 2,
        screens: &[DialogScreen {
            screen_id: 200010,
            speaker_id: 0,
            text: "Vault expansion. I can add 10 slots to your vault, up to 100 in all. The \
                   price of the next 10 slots is in your chat window.",
            buttons: &[DialogButton {
                button_type: 4,
                button_id: 8,
                text: "Expand vault",
            }],
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
        for ov in DIALOG_OVERRIDES.iter().chain(QUARANTINED_DIALOG_OVERRIDES) {
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

    /// A quarantined dialog must not be served. Moving one back into
    /// `DIALOG_OVERRIDES` without also taking it out of the quarantine list
    /// fails here, so a restore is always a deliberate two-sided edit, made
    /// once RE has named the field that crashed the client on map load.
    #[test]
    fn quarantined_dialogs_are_not_served() {
        for q in QUARANTINED_DIALOG_OVERRIDES {
            assert!(
                !DIALOG_OVERRIDES
                    .iter()
                    .any(|ov| ov.dialog_id == q.dialog_id),
                "dialog {} is quarantined (client map-load crash, 2026-09-27) but is \
                 back in DIALOG_OVERRIDES; lift the quarantine explicitly by removing \
                 it from QUARANTINED_DIALOG_OVERRIDES",
                q.dialog_id,
            );
        }
    }

    /// The debug-hub dialogs are the ones held back, and each is still
    /// defined in full so a restore is a move, not a rewrite.
    #[test]
    fn debug_hub_dialogs_are_quarantined() {
        let ids: Vec<u32> = QUARANTINED_DIALOG_OVERRIDES
            .iter()
            .map(|ov| ov.dialog_id)
            .collect();
        assert_eq!(ids, [60100, 60101, 60104, 60110]);
        let xml = |id: u32| {
            let ov = QUARANTINED_DIALOG_OVERRIDES
                .iter()
                .find(|ov| ov.dialog_id == id)
                .expect("quarantined definition");
            String::from_utf8(generate_dialog_xml(ov)).unwrap()
        };
        // The button-carrying contract each one had while served.
        assert!(xml(60100)
            .contains("<Buttons ButtonType=\"4\" ButtonID=\"8\" Text=\"Send my choice\">"));
        assert!(!xml(60101).contains("<Buttons"));
        assert!(xml(60104)
            .contains("<Buttons ButtonType=\"4\" ButtonID=\"8\" Text=\"Send me a mail\">"));
        // BV-05: exactly one Generic 1 button, the one the purchase buys on.
        let expand = xml(60110);
        assert!(expand.contains("<Buttons ButtonType=\"4\" ButtonID=\"8\" Text=\"Expand vault\">"));
        assert_eq!(expand.matches("<Buttons").count(), 1, "{expand}");
    }

    /// The two mission-622 overrides must keep zero buttons. A button here
    /// would stop the client sending `dialogButtonChoice(id, -1)` on
    /// close, and the mission 622 search chains key on exactly that.
    #[test]
    fn shipped_overrides_carry_no_buttons() {
        for ov in DIALOG_OVERRIDES
            .iter()
            .filter(|ov| [3995, 3996].contains(&ov.dialog_id))
        {
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
