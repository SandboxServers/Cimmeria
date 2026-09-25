//! Reader for a canonical `<COOKED_DIALOG>` entry.
//!
//! Patch-mode overrides ([`super::DialogPatch`]) transform the entry the
//! client already shipped instead of re-authoring it, so they need to read
//! the QA-build XML back into a [`CookedDialog`].
//!
//! Built on `quick-xml`, which `cimmeria-services` already depends on (the
//! auth handlers, the minigame protocol and `spaces.xml` all use it). No
//! new dependency, and a real reader handles the cases a substring scanner
//! gets wrong: a `>` inside an attribute value, attribute values split
//! across lines, and the five namespace declarations the QA build hangs
//! off the root element.
//!
//! # What the parser tolerates
//!
//! * **QA build**: `<?xml …?>` + newline + `<COOKED_DIALOG xmlns:SOAP-ENV=…
//!   DialogFlags=… KismetEventSetID=… UIScreenType=… DialogID=…>`. This is
//!   what `data/cache/CookedDataDialogs.pak` actually holds.
//! * **Server build**: no namespaces, alphabetised root attributes. This is
//!   what [`super::emit_cooked_dialog`] writes, so a patch can be applied
//!   on top of a full-regeneration override.
//! * Either attribute order, since lookup is by name.
//! * `<Screens>` with or without a `ScreenID` attribute (218 committed
//!   dialogs omit it on at least one screen).
//! * `<Screens>` and `<Buttons>` written self-closing or with an explicit
//!   close tag. Every shipped one uses the explicit close; so does the
//!   emitter.
//!
//! # What it rejects
//!
//! Anything else returns `None`, and the caller keeps the canonical entry
//! untouched rather than shipping a half-understood rewrite: a missing or
//! malformed root, an unknown element, a missing required attribute, a
//! non-numeric id, a `<Buttons>` outside a `<Screens>`, anything nested
//! inside a `<Buttons>`, a `<Screens>` that is not a direct child of the
//! root, an unmatched close tag, anything after the root closes,
//! non-whitespace character data, or non-UTF-8 bytes.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::emit::{CookedButton, CookedDialog, CookedScreen};

/// Parse a cooked dialog entry. Returns `None` if the bytes are not a
/// `<COOKED_DIALOG>` document this module fully understands.
pub fn parse_cooked_dialog(xml: &[u8]) -> Option<CookedDialog> {
    let text = std::str::from_utf8(xml).ok()?;
    let mut reader = Reader::from_str(text);

    let mut dialog: Option<CookedDialog> = None;
    // Set by `</COOKED_DIALOG>`; nothing may follow the root.
    let mut root_closed = false;
    // `Some` between `<Screens …>` and `</Screens>`; buttons land in it.
    let mut open_screen: Option<CookedScreen> = None;
    // Between `<Buttons …>` and `</Buttons>`. A button has no children, so
    // anything opened inside it is a hierarchy the client never ships.
    let mut in_button = false;

    loop {
        let event = reader.read_event().ok()?;
        // Only whitespace, the declaration and comments may follow the
        // root, and nothing may sit inside a button.
        let structural = matches!(event, Event::Start(_) | Event::Empty(_) | Event::End(_));
        if structural
            && in_button
            && !matches!(&event, Event::End(e) if e.name().as_ref() == b"Buttons")
        {
            return None;
        }
        if structural && root_closed {
            return None;
        }
        match event {
            // ── Elements with a body ────────────────────────────────
            Event::Start(e) => match e.name().as_ref() {
                b"COOKED_DIALOG" => {
                    if dialog.is_some() {
                        return None; // second root
                    }
                    dialog = Some(parse_root(&e)?);
                }
                b"Screens" => {
                    // Directly under the root, never nested.
                    if dialog.is_none() || open_screen.is_some() {
                        return None;
                    }
                    open_screen = Some(parse_screen(&e)?);
                }
                b"Buttons" => {
                    // `?` on `None` here is the "button outside a screen"
                    // rejection.
                    open_screen.as_mut()?.buttons.push(parse_button(&e)?);
                    in_button = true;
                }
                _ => return None, // unknown element — refuse to guess
            },

            // ── Self-closing elements ───────────────────────────────
            Event::Empty(e) => match e.name().as_ref() {
                b"Screens" => {
                    if open_screen.is_some() {
                        return None;
                    }
                    // No body, so no buttons and no matching `End`.
                    dialog.as_mut()?.screens.push(parse_screen(&e)?);
                }
                b"Buttons" => {
                    open_screen.as_mut()?.buttons.push(parse_button(&e)?);
                }
                // A self-closing root would carry no screens at all; the
                // shipped data never does this, so refuse it.
                _ => return None,
            },

            Event::End(e) => match e.name().as_ref() {
                b"Screens" => {
                    // `None` means a `</Screens>` with nothing open.
                    let screen = open_screen.take()?;
                    dialog.as_mut()?.screens.push(screen);
                }
                b"Buttons" => {
                    if !in_button {
                        return None;
                    }
                    in_button = false;
                }
                b"COOKED_DIALOG" => {
                    if dialog.is_none() || open_screen.is_some() {
                        return None;
                    }
                    root_closed = true;
                }
                _ => return None,
            },

            // Whitespace between elements (the QA newline after the XML
            // declaration) is not content; any other character data is a
            // shape the emitter could not reproduce, so refuse it.
            Event::Text(t) => {
                if !t.iter().all(u8::is_ascii_whitespace) {
                    return None;
                }
            }
            Event::Decl(_) | Event::Comment(_) => {}
            Event::Eof => break,
            _ => return None,
        }
    }

    if open_screen.is_some() || !root_closed {
        return None; // unterminated <Screens> or root
    }
    dialog
}

/// Root attributes. All four are required: every one of the 5,405
/// committed entries carries all four, so a missing one means the shape
/// drifted and guessing a default would ship a silently wrong entry.
fn parse_root(e: &BytesStart<'_>) -> Option<CookedDialog> {
    let mut dialog_flags = None;
    let mut dialog_id = None;
    let mut kismet_event_set_id = None;
    let mut ui_screen_type = None;

    for attr in e.attributes() {
        let attr = attr.ok()?;
        match attr.key.as_ref() {
            b"DialogFlags" => dialog_flags = Some(attr_u32(&attr.value)?),
            b"DialogID" => dialog_id = Some(attr_u32(&attr.value)?),
            b"KismetEventSetID" => kismet_event_set_id = Some(attr_u32(&attr.value)?),
            b"UIScreenType" => ui_screen_type = Some(attr_u32(&attr.value)?),
            // The five `xmlns:*` declarations on a QA root, and anything
            // else a future cook adds, are dropped: Server-Build output
            // carries no namespaces.
            _ => {}
        }
    }

    Some(CookedDialog {
        dialog_flags: dialog_flags?,
        dialog_id: dialog_id?,
        kismet_event_set_id: kismet_event_set_id?,
        ui_screen_type: ui_screen_type?,
        screens: Vec::new(),
    })
}

/// `<Screens>` attributes. `SpeakerID` and `Text` are required;
/// `ScreenID` is optional (see [`CookedScreen::screen_id`]).
fn parse_screen(e: &BytesStart<'_>) -> Option<CookedScreen> {
    let mut speaker_id = None;
    let mut screen_id = None;
    let mut text_escaped = None;

    for attr in e.attributes() {
        let attr = attr.ok()?;
        match attr.key.as_ref() {
            b"SpeakerID" => speaker_id = Some(attr_u32(&attr.value)?),
            b"ScreenID" => screen_id = Some(attr_u32(&attr.value)?),
            // Raw, still-escaped value — see the `emit` module docs.
            b"Text" => text_escaped = Some(attr_raw(&attr.value)?),
            _ => {}
        }
    }

    Some(CookedScreen {
        speaker_id: speaker_id?,
        screen_id,
        text_escaped: text_escaped?,
        buttons: Vec::new(),
    })
}

/// `<Buttons>` attributes. All three are required — the census found no
/// shipped button missing any of them, and a button with no `ButtonID`
/// would put an unknown value on the wire when clicked.
fn parse_button(e: &BytesStart<'_>) -> Option<CookedButton> {
    let mut button_type = None;
    let mut button_id = None;
    let mut text_escaped = None;

    for attr in e.attributes() {
        let attr = attr.ok()?;
        match attr.key.as_ref() {
            b"ButtonType" => button_type = Some(attr_u32(&attr.value)?),
            b"ButtonID" => button_id = Some(attr_u32(&attr.value)?),
            b"Text" => text_escaped = Some(attr_raw(&attr.value)?),
            _ => {}
        }
    }

    Some(CookedButton {
        button_type: button_type?,
        button_id: button_id?,
        text_escaped: text_escaped?,
    })
}

/// Numeric attribute value. Ids never carry entity references, so the raw
/// bytes parse directly.
fn attr_u32(value: &[u8]) -> Option<u32> {
    std::str::from_utf8(value).ok()?.trim().parse::<u32>().ok()
}

/// Text attribute value, kept in its escaped wire form.
fn attr_raw(value: &[u8]) -> Option<String> {
    Some(std::str::from_utf8(value).ok()?.to_string())
}

#[cfg(test)]
mod tests {
    use super::super::emit::emit_cooked_dialog;
    use super::*;

    /// Inline QA-shape fixture modelled on the committed `_2576`
    /// (Copplemann's mission offer): SOAP namespaces on the root, QA
    /// attribute order (`SpeakerID Text ScreenID`), Take Missions on 3 of
    /// its 5 screens. Trimmed to 3 screens; the full 5-screen version
    /// lives in the patch tests.
    const QA_2576_HEAD: &str = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<COOKED_DIALOG xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
        "xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" ",
        "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
        "xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" ",
        "xmlns:CookedData1=\"SGW\" ",
        "DialogFlags=\"0\" KismetEventSetID=\"0\" UIScreenType=\"2\" DialogID=\"2576\">",
        "<Screens SpeakerID=\"1110\" Text=\"Rescue Dr. Zuritska.\" ScreenID=\"96821\">",
        "<Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\"></Buttons>",
        "</Screens>",
        "<Screens SpeakerID=\"0\" Text=\"Where is he?\" ScreenID=\"96822\">",
        "<Buttons ButtonType=\"4\" ButtonID=\"71\" Text=\"Take Missions\"></Buttons>",
        "</Screens>",
        "<Screens SpeakerID=\"0\" Text=\"Romney...\" ScreenID=\"96824\"></Screens>",
        "</COOKED_DIALOG>",
    );

    #[test]
    fn parses_qa_shape_with_soap_namespaces_and_nested_buttons() {
        let d = parse_cooked_dialog(QA_2576_HEAD.as_bytes()).expect("QA shape must parse");
        assert_eq!(d.dialog_id, 2576);
        assert_eq!(d.dialog_flags, 0);
        assert_eq!(d.kismet_event_set_id, 0);
        assert_eq!(d.ui_screen_type, 2);
        assert_eq!(d.screens.len(), 3);

        assert_eq!(d.screens[0].speaker_id, 1110);
        assert_eq!(d.screens[0].screen_id, Some(96821));
        assert_eq!(d.screens[0].text_escaped, "Rescue Dr. Zuritska.");
        assert_eq!(d.screens[0].buttons.len(), 1);
        assert_eq!(d.screens[0].buttons[0].button_type, 4);
        assert_eq!(d.screens[0].buttons[0].button_id, 71);
        assert_eq!(d.screens[0].buttons[0].text_escaped, "Take Missions");

        assert!(
            d.screens[2].buttons.is_empty(),
            "the button-less final screen must parse with zero buttons",
        );
    }

    /// The 218 committed dialogs with a `ScreenID`-less `<Screens>` must
    /// parse to `None` rather than a fabricated `0`, and re-emit without
    /// the attribute.
    #[test]
    fn parses_screen_without_screen_id() {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
                   <COOKED_DIALOG DialogFlags=\"1\" DialogID=\"1011\" KismetEventSetID=\"0\" \
                   UIScreenType=\"2\">\
                   <Screens SpeakerID=\"417\" Text=\"No key.\"></Screens>\
                   </COOKED_DIALOG>";
        let d = parse_cooked_dialog(xml.as_bytes()).expect("must parse");
        assert_eq!(d.screens[0].screen_id, None);
        let out = emit_cooked_dialog(&d);
        assert!(
            !std::str::from_utf8(&out).unwrap().contains("ScreenID"),
            "a ScreenID the source never had must not be invented",
        );
    }

    /// Escaped text and the `&#xA;` newline reference survive a
    /// parse → emit cycle byte-for-byte. This is what lets a button patch
    /// promise the body text is untouched.
    #[test]
    fn round_trip_preserves_escaped_text_verbatim() {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
                   <COOKED_DIALOG DialogFlags=\"0\" DialogID=\"5354\" KismetEventSetID=\"0\" \
                   UIScreenType=\"2\">\
                   <Screens SpeakerID=\"0\" ScreenID=\"1\" \
                   Text=\"a &amp; b &lt;&lt;playername>> &quot;q&quot;&#xA;next\"></Screens>\
                   </COOKED_DIALOG>";
        let d = parse_cooked_dialog(xml.as_bytes()).expect("must parse");
        assert_eq!(
            d.screens[0].text_escaped, "a &amp; b &lt;&lt;playername>> &quot;q&quot;&#xA;next",
            "attribute text must be kept in escaped form, not decoded",
        );
        assert_eq!(
            String::from_utf8(emit_cooked_dialog(&d)).unwrap(),
            xml,
            "a Server-Build-shaped entry must survive parse → emit unchanged",
        );
    }

    /// Emitter output feeds back into the parser, so a patch can be
    /// applied on top of a full-regeneration override.
    #[test]
    fn emitter_output_parses_back_to_an_equal_model() {
        let original = parse_cooked_dialog(QA_2576_HEAD.as_bytes()).expect("must parse");
        let reparsed =
            parse_cooked_dialog(&emit_cooked_dialog(&original)).expect("emitted form must parse");
        assert_eq!(original, reparsed);
    }

    /// Self-closing forms are accepted even though nothing ships them,
    /// because the Server-Build cooker self-closes empty children in
    /// every other category (`docs/engine/cooked-data-pak-format.md`).
    #[test]
    fn accepts_self_closing_screens_and_buttons() {
        let xml = "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"7\" KismetEventSetID=\"0\" \
                   UIScreenType=\"2\">\
                   <Screens SpeakerID=\"1\" ScreenID=\"2\" Text=\"a\"/>\
                   <Screens SpeakerID=\"1\" ScreenID=\"3\" Text=\"b\">\
                   <Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"Accept\"/>\
                   </Screens>\
                   </COOKED_DIALOG>";
        let d = parse_cooked_dialog(xml.as_bytes()).expect("self-closing forms must parse");
        assert_eq!(d.screens.len(), 2);
        assert!(d.screens[0].buttons.is_empty());
        assert_eq!(d.screens[1].buttons.len(), 1);
        assert_eq!(d.screens[1].buttons[0].button_id, 8);
    }

    /// Malformed shapes are refused, not guessed at. Each of these makes
    /// the caller keep the canonical entry and emit a warn.
    #[test]
    fn rejects_shapes_it_does_not_fully_understand() {
        let cases: &[(&str, &str)] = &[
            ("not xml at all", "garbage"),
            (
                "missing UIScreenType",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\"></COOKED_DIALOG>",
            ),
            (
                "non-numeric DialogID",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"abc\" KismetEventSetID=\"0\" \
                 UIScreenType=\"2\"></COOKED_DIALOG>",
            ),
            (
                "screen without Text",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\" \
                 UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\"></Screens>\
                 </COOKED_DIALOG>",
            ),
            (
                "button outside a screen",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\" \
                 UIScreenType=\"2\"><Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"A\">\
                 </Buttons></COOKED_DIALOG>",
            ),
            (
                "button without ButtonID",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\" \
                 UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"a\">\
                 <Buttons ButtonType=\"2\" Text=\"A\"></Buttons></Screens></COOKED_DIALOG>",
            ),
            (
                "unknown child element",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\" \
                 UIScreenType=\"2\"><Mystery/></COOKED_DIALOG>",
            ),
            (
                "Buttons nested inside Buttons",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\"                  UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"a\">                 <Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"A\">                 <Buttons ButtonType=\"2\" ButtonID=\"9\" Text=\"B\"></Buttons>                 </Buttons></Screens></COOKED_DIALOG>",
            ),
            (
                "Screens nested inside Buttons",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\"                  UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"a\">                 <Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"A\">                 <Screens SpeakerID=\"0\" ScreenID=\"2\" Text=\"b\"/>                 </Buttons></Screens></COOKED_DIALOG>",
            ),
            (
                "stray </Buttons> with none open",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\"                  UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"a\">                 <Buttons ButtonType=\"2\" ButtonID=\"8\" Text=\"A\"/></Buttons>                 </Screens></COOKED_DIALOG>",
            ),
            (
                "non-whitespace character data inside a screen",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\"                  UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"a\">                 stray body text</Screens></COOKED_DIALOG>",
            ),
            (
                "element after the root closed",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\"                  UIScreenType=\"2\"></COOKED_DIALOG><Screens SpeakerID=\"0\" Text=\"a\"/>",
            ),
            (
                "unterminated root",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\"                  UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"a\">                 </Screens>",
            ),
            (
                "unterminated Screens",
                "<COOKED_DIALOG DialogFlags=\"0\" DialogID=\"1\" KismetEventSetID=\"0\" \
                 UIScreenType=\"2\"><Screens SpeakerID=\"0\" ScreenID=\"1\" Text=\"a\">",
            ),
        ];
        for (label, xml) in cases {
            assert!(
                parse_cooked_dialog(xml.as_bytes()).is_none(),
                "{label}: must be refused, not parsed",
            );
        }
    }
}
