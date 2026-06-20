//! Cimmeria-side overrides for `CookedDataDialogs.pak` entries.
//!
//! A dialog's player-visible body text lives in the `_<dialogId>` entry
//! inside `CookedDataDialogs.pak` on the client — **not** in any wire
//! message or in the server's `dialogs` / `dialog_screens` tables. The
//! server's `displayDialog` path only carries the dialog *id*; the client
//! looks the screen text up from its own cooked catalogue. So editing a
//! row in `db/resources/Dialogs/Seed/dialog_screens.sql` has zero in-game
//! effect on what renders — and a brand-new dialog id the client has never
//! seen renders as an empty box.
//!
//! Same trick as [`super::mission_overrides`] / [`super::item_overrides`]:
//! patch the catalogue in memory at server startup and lean on the
//! cooked-data wire path (`versionInfoRequest` → `onVersionInfo(InvalidKeys
//! =[...])` → `resourceFragment(_key, patched XML)`) to push the entry to
//! the client. Self-healing — runtime cache invalidation, no manual client
//! steps, no on-disk PAK edit, no client-artifact redistribution.
//!
//! Unlike the mission/item override modules, which *patch* an attribute or
//! splice a child into the canonical XML, a dialog override **regenerates
//! the whole `<COOKED_DIALOG>` entry from scratch**. The cooked dialog
//! entries are small (root + one or more `<Screens>`) and we control every
//! field, so emitting the complete Server-Build XML is more robust than
//! anchoring on a substring — and it makes the *new-entry* case (a dialog
//! id the PAK never shipped, like the Guard corpse's 3996) identical to the
//! *corrected-entry* case (Frost's 3995): both are just an
//! `elements.insert(dialog_id, generated_xml)`.
//!
//! The emitted XML follows the Server-Build conventions documented in
//! [`docs/engine/cooked-data-pak-format.md`]: no SOAP namespaces,
//! alphabetized root attributes (`DialogFlags`, `DialogID`,
//! `KismetEventSetID`, `UIScreenType`), and child `<Screens>` with
//! `SpeakerID` → `ScreenID` → `Text` order and an explicit `</Screens>`
//! close.

/// One screen line within a dialog. `text` is the raw, human-readable
/// string — XML escaping (`"` → `&quot;`, `&` → `&amp;`, `<`/`>`) is
/// applied by the generator, so callers write the text exactly as it
/// should read in-game.
pub struct DialogScreen {
    pub screen_id: u32,
    /// Speaker entity id. `0` for a system/narrator line (the search-body
    /// dialogs are narrator text, not spoken by an NPC).
    pub speaker_id: u32,
    pub text: &'static str,
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
        }],
    },
];

/// XML-escape a text value destined for a double-quoted attribute.
///
/// Escapes the five XML predefined entities. `"` → `&quot;` is the
/// load-bearing one for these dialogs (the Frost text quotes "Jess");
/// `&` must come logically "first" in intent but since we scan char by
/// char and emit the entity for each source char, ordering is a non-issue
/// — a literal `&` in the source becomes `&amp;`, never re-escaped.
fn escape_xml_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Generate the full Server-Build `<COOKED_DIALOG>` XML bytes for one
/// override. Always succeeds — there's no canonical entry to anchor
/// against, we emit the complete document from the struct fields.
///
/// Root attributes are alphabetized (`DialogFlags`, `DialogID`,
/// `KismetEventSetID`, `UIScreenType`); each `<Screens>` child emits
/// `SpeakerID` → `ScreenID` → `Text` with an explicit close, matching the
/// Server-Build shape in `docs/engine/cooked-data-pak-format.md`.
pub fn generate_dialog_xml(ov: &DialogOverride) -> Vec<u8> {
    let mut xml = String::with_capacity(256);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    xml.push_str(&format!(
        "<COOKED_DIALOG DialogFlags=\"{}\" DialogID=\"{}\" KismetEventSetID=\"{}\" \
         UIScreenType=\"{}\">",
        ov.dialog_flags, ov.dialog_id, ov.kismet_event_set_id, ov.ui_screen_type,
    ));
    for screen in ov.screens {
        xml.push_str(&format!(
            "<Screens SpeakerID=\"{}\" ScreenID=\"{}\" Text=\"{}\"></Screens>",
            screen.speaker_id,
            screen.screen_id,
            escape_xml_attr(screen.text),
        ));
    }
    xml.push_str("</COOKED_DIALOG>");
    xml.into_bytes()
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

    /// `escape_xml_attr` covers all five predefined entities and leaves
    /// apostrophes-as-`&apos;` (safe inside a double-quoted attribute, and
    /// harmless). Pins the escaping contract directly.
    #[test]
    fn escape_covers_predefined_entities() {
        assert_eq!(
            escape_xml_attr("a & b < c > d \" e ' f"),
            "a &amp; b &lt; c &gt; d &quot; e &apos; f",
        );
    }
}
