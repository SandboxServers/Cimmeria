//! The parsed cooked-dialog model and the single Server-Build XML emitter.
//!
//! Both dialog override kinds funnel through [`emit_cooked_dialog`]:
//!
//! * a **full regeneration** ([`super::DialogOverride`]) builds a
//!   [`CookedDialog`] from Rust-authored text, and
//! * a **patch** ([`super::DialogPatch`]) parses the canonical PAK entry
//!   into a [`CookedDialog`], edits it, and re-emits.
//!
//! One emitter means the two kinds can never drift into two different
//! on-the-wire shapes for the same logical content.
//!
//! # Text values are stored ESCAPED
//!
//! Every `*_escaped` field holds the attribute value exactly as it appears
//! between the quotes in the XML — entity references and all. The emitter
//! writes it back verbatim.
//!
//! This is deliberate. The shipped QA entries use `&#xA;` for a newline
//! (`docs/engine/cooked-data-pak-format.md`, "Newline encoding"), leave
//! apostrophes raw, and carry `&lt;&lt;playername>>` template markers. A
//! decode-then-re-encode round trip would rewrite `&#xA;` to a literal
//! newline (which an XML reader then normalises to a space, silently
//! reflowing the line) and would have to guess which of the five
//! predefined entities to re-emit. Keeping the raw form makes "patch the
//! buttons, keep the text" a byte-exact promise rather than a best effort.
//!
//! Rust-authored text goes the other way: it is written plainly in the
//! source and run through [`escape_xml_attr`] on the way in.
//!
//! # Attribute order
//!
//! Root attributes are alphabetised (`DialogFlags`, `DialogID`,
//! `KismetEventSetID`, `UIScreenType`), which is the Server-Build
//! convention for the root element.
//!
//! Children are **not** alphabetised, because the Server Build did not
//! alphabetise them either: `docs/engine/cooked-data-pak-format.md` shows
//! `<Screens SpeakerID ScreenID Text>` where alphabetical would be
//! `ScreenID SpeakerID Text`. Comparing the two builds, the cooker's child
//! transform was "move `Text` last, leave the other attributes in their
//! cooked order" — QA's `SpeakerID Text ScreenID` becomes Server's
//! `SpeakerID ScreenID Text`.
//!
//! Applying that same transform to `<Buttons>` is a no-op: all 4,349
//! `<Buttons>` elements in the committed QA PAK are already
//! `ButtonType ButtonID Text` with `Text` last (census 2026-09-21), and
//! that is also the order the client contract records (ledger fact F3).
//! So the emitter writes `ButtonType`, `ButtonID`, `Text`.
//!
//! Order is a documentation choice, not a functional one — the client's
//! cooked parser looks attributes up by name (Native Findings,
//! `FUN_015e4d10`). Emitting the shipped order keeps a diff between a
//! patched entry and its canonical original readable.

/// One button inside a `<Screens>` element.
///
/// `button_id` is the value the client puts on the wire when the button is
/// clicked — *not* the button's index. Accept is 8, More Info 9, Receive
/// Item 70, Take Missions 71 (ledger fact F14).
///
/// `button_type` selects the widget: 1 More Info, 2 Accept, 3 Decline,
/// 4/5/6 Generic1-3. `DialogWin` renders only type 2 and 4+; `BlurbWin`
/// renders only 1 and 2 (ledger fact F7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookedButton {
    pub button_type: u32,
    pub button_id: u32,
    /// XML-escaped label; emitted verbatim. See the module docs.
    pub text_escaped: String,
}

/// One `<Screens>` row: a line of dialog plus the buttons offered on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookedScreen {
    pub speaker_id: u32,
    /// `None` for a `<Screens>` element that carries no `ScreenID`
    /// attribute at all. 218 of the 5,405 committed QA dialogs contain at
    /// least one such screen (census 2026-09-21); none of them is in the
    /// dialog UI redesign target matrix, but the parser must round-trip
    /// them rather than invent a `ScreenID="0"` the client never shipped.
    pub screen_id: Option<u32>,
    /// XML-escaped body text; emitted verbatim. See the module docs.
    pub text_escaped: String,
    /// Buttons in document order. Order is load-bearing: the client
    /// resolves a click to a position in this array and then sends the
    /// `ButtonID` sitting at that position (Native Findings,
    /// `FUN_00ad8690`).
    pub buttons: Vec<CookedButton>,
}

/// A whole `<COOKED_DIALOG>` entry, parsed or authored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookedDialog {
    pub dialog_flags: u32,
    pub dialog_id: u32,
    pub kismet_event_set_id: u32,
    pub ui_screen_type: u32,
    pub screens: Vec<CookedScreen>,
}

/// XML-escape a text value destined for a double-quoted attribute.
///
/// Escapes the five XML predefined entities. `"` → `&quot;` is the
/// load-bearing one for the Rust-authored dialogs (the Frost text quotes
/// "Jess"); an unescaped quote terminates the attribute early and the
/// client either truncates the line or rejects the entry.
///
/// Scanning char by char means ordering is a non-issue — a literal `&` in
/// the source becomes `&amp;` and is never re-escaped.
///
/// Only ever applied to *authored* text. Text lifted out of a canonical
/// PAK entry is already escaped and is carried through verbatim.
pub fn escape_xml_attr(text: &str) -> String {
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

/// Serialise a [`CookedDialog`] as Server-Build `<COOKED_DIALOG>` bytes.
///
/// Infallible: every field is already a number or an escaped string, so
/// there is no shape to fail against. See the module docs for the
/// attribute-order rationale.
pub fn emit_cooked_dialog(dialog: &CookedDialog) -> Vec<u8> {
    let mut xml = String::with_capacity(256);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    xml.push_str("<COOKED_DIALOG DialogFlags=\"");
    xml.push_str(&dialog.dialog_flags.to_string());
    xml.push_str("\" DialogID=\"");
    xml.push_str(&dialog.dialog_id.to_string());
    xml.push_str("\" KismetEventSetID=\"");
    xml.push_str(&dialog.kismet_event_set_id.to_string());
    xml.push_str("\" UIScreenType=\"");
    xml.push_str(&dialog.ui_screen_type.to_string());
    xml.push_str("\">");

    for screen in &dialog.screens {
        xml.push_str("<Screens SpeakerID=\"");
        xml.push_str(&screen.speaker_id.to_string());
        // Omitted entirely when the source screen had no ScreenID, so a
        // parse → emit round trip of such an entry is byte-stable.
        if let Some(screen_id) = screen.screen_id {
            xml.push_str("\" ScreenID=\"");
            xml.push_str(&screen_id.to_string());
        }
        xml.push_str("\" Text=\"");
        xml.push_str(&screen.text_escaped);
        xml.push_str("\">");

        for button in &screen.buttons {
            xml.push_str("<Buttons ButtonType=\"");
            xml.push_str(&button.button_type.to_string());
            xml.push_str("\" ButtonID=\"");
            xml.push_str(&button.button_id.to_string());
            xml.push_str("\" Text=\"");
            xml.push_str(&button.text_escaped);
            // Explicit close: every one of the 4,349 shipped `<Buttons>`
            // elements uses one, and none is self-closing.
            xml.push_str("\"></Buttons>");
        }

        xml.push_str("</Screens>");
    }

    xml.push_str("</COOKED_DIALOG>");
    xml.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(
        speaker_id: u32,
        screen_id: u32,
        text: &str,
        buttons: Vec<CookedButton>,
    ) -> CookedScreen {
        CookedScreen {
            speaker_id,
            screen_id: Some(screen_id),
            text_escaped: text.to_string(),
            buttons,
        }
    }

    fn button(button_type: u32, button_id: u32, text: &str) -> CookedButton {
        CookedButton {
            button_type,
            button_id,
            text_escaped: text.to_string(),
        }
    }

    fn dialog(screens: Vec<CookedScreen>) -> CookedDialog {
        CookedDialog {
            dialog_flags: 0,
            dialog_id: 2576,
            kismet_event_set_id: 0,
            ui_screen_type: 2,
            screens,
        }
    }

    /// Byte-exact emitter pin, zero buttons. This is the shape every
    /// Cimmeria-authored dialog has shipped with since the search-corpse
    /// entries (3995 / 3996) landed, so the bytes must not move.
    #[test]
    fn emits_screen_with_zero_buttons_byte_exact() {
        let out = emit_cooked_dialog(&dialog(vec![screen(1110, 96821, "Hello.", vec![])]));
        assert_eq!(
            std::str::from_utf8(&out).unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <COOKED_DIALOG DialogFlags=\"0\" DialogID=\"2576\" KismetEventSetID=\"0\" \
             UIScreenType=\"2\">\
             <Screens SpeakerID=\"1110\" ScreenID=\"96821\" Text=\"Hello.\"></Screens>\
             </COOKED_DIALOG>",
        );
    }

    /// Byte-exact emitter pin, one button. Nests inside `<Screens>`,
    /// carries `ButtonType` then `ButtonID` then `Text`, and closes
    /// explicitly — the shape of all 4,349 shipped `<Buttons>` elements.
    #[test]
    fn emits_screen_with_one_button_byte_exact() {
        let out = emit_cooked_dialog(&dialog(vec![screen(
            1110,
            96825,
            "Take these.",
            vec![button(4, 71, "Take Missions")],
        )]));
        assert_eq!(
            std::str::from_utf8(&out).unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <COOKED_DIALOG DialogFlags=\"0\" DialogID=\"2576\" KismetEventSetID=\"0\" \
             UIScreenType=\"2\">\
             <Screens SpeakerID=\"1110\" ScreenID=\"96825\" Text=\"Take these.\">\
             <Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\"></Buttons>\
             </Screens>\
             </COOKED_DIALOG>",
        );
    }

    /// Byte-exact emitter pin, two buttons — the shipped Blurb 2572 shape
    /// (More Info then Accept). Document order must survive: the client
    /// turns a click into an index into this array and sends whichever
    /// `ButtonID` sits there, so swapping these two would make "More Info"
    /// send 8 (Accept).
    #[test]
    fn emits_screen_with_two_buttons_in_document_order_byte_exact() {
        let out = emit_cooked_dialog(&dialog(vec![screen(
            0,
            96765,
            "Reinforce Copplemann.",
            vec![button(1, 9, "More Info"), button(2, 8, "Accept")],
        )]));
        assert_eq!(
            std::str::from_utf8(&out).unwrap(),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <COOKED_DIALOG DialogFlags=\"0\" DialogID=\"2576\" KismetEventSetID=\"0\" \
             UIScreenType=\"2\">\
             <Screens SpeakerID=\"0\" ScreenID=\"96765\" Text=\"Reinforce Copplemann.\">\
             <Buttons ButtonType=\"1\" ButtonID=\"9\" Text=\"More Info\"></Buttons>\
             <Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"Accept\"></Buttons>\
             </Screens>\
             </COOKED_DIALOG>",
        );
    }

    /// A screen parsed from one of the 218 entries with no `ScreenID`
    /// emits no `ScreenID` attribute. Emitting `ScreenID="0"` instead
    /// would invent a screen key the client never shipped.
    #[test]
    fn omits_screen_id_attribute_when_absent() {
        let out = emit_cooked_dialog(&dialog(vec![CookedScreen {
            speaker_id: 417,
            screen_id: None,
            text_escaped: "No key here.".to_string(),
            buttons: vec![],
        }]));
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("<Screens SpeakerID=\"417\" Text=\"No key here.\"></Screens>"),
            "ScreenID must be omitted, not defaulted: {s}",
        );
        assert!(!s.contains("ScreenID"), "no ScreenID may be invented: {s}");
    }

    /// Escaped text is emitted verbatim — no double-escaping of an entity
    /// that arrived already escaped, and no decoding of `&#xA;`. This is
    /// the invariant that lets a patch promise "text unchanged".
    #[test]
    fn escaped_text_passes_through_verbatim() {
        let out = emit_cooked_dialog(&dialog(vec![screen(
            0,
            1,
            "a &amp; b &lt;&lt;playername>> &quot;q&quot;&#xA;next",
            vec![],
        )]));
        let s = std::str::from_utf8(&out).unwrap();
        assert!(
            s.contains("Text=\"a &amp; b &lt;&lt;playername>> &quot;q&quot;&#xA;next\""),
            "escaped source text must survive byte-for-byte: {s}",
        );
        assert!(
            !s.contains("&amp;amp;"),
            "already-escaped entities must not be re-escaped: {s}",
        );
    }

    /// `escape_xml_attr` covers all five predefined entities.
    #[test]
    fn escape_covers_predefined_entities() {
        assert_eq!(
            escape_xml_attr("a & b < c > d \" e ' f"),
            "a &amp; b &lt; c &gt; d &quot; e &apos; f",
        );
    }

    /// Escaping round trip against a real XML reader: whatever
    /// `escape_xml_attr` produces must decode back to the original string.
    /// Using quick-xml's own unescape (the same crate that backs
    /// [`super::super::parse`]) makes this a statement about agreement
    /// with a conforming parser rather than about an in-house inverse.
    #[test]
    fn escape_round_trips_through_an_xml_reader() {
        for original in [
            "plain",
            "a & b",
            "<tag>",
            "quote \" here",
            "apostrophe ' here",
            "all five: & < > \" '",
            "Frost's letter to \"Jess\" & co. <redacted>",
        ] {
            let escaped = escape_xml_attr(original);
            let decoded = quick_xml::escape::unescape(&escaped)
                .unwrap_or_else(|e| panic!("escaped form of {original:?} must parse: {e}"));
            assert_eq!(
                decoded, original,
                "escape/unescape must round trip for {original:?}",
            );
        }
    }
}
