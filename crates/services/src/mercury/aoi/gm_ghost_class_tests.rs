//! Pins the two `.def`-derived facts that make it safe to introduce a GM to
//! other players as `SGWGmPlayer` (0x03) while encoding its methods with
//! [`IDBASE_SGW_PLAYER`] — see
//! [`crate::mercury::player_class_id_for_access_level`].
//!
//! Read straight from `entities/defs/` rather than restated as constants, so
//! an edit to either def that breaks the assumption — a new `<Implements>` on
//! `SGWGmPlayer`, enough new client methods to change the idBase bucket —
//! fails here instead of silently mis-decoding every method on a GM ghost.

use std::path::PathBuf;

use cimmeria_mercury::channel_bundle::{idbase_from_exposed_method_count, IDBASE_SGW_PLAYER};
use quick_xml::events::Event;
use quick_xml::Reader;

fn defs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../entities/defs")
}

/// `(parent, implements in XML order, own client-method names)` of one def.
fn read_def(name: &str, interface: bool) -> (Option<String>, Vec<String>, Vec<String>) {
    let mut path = defs_dir();
    if interface {
        path.push("interfaces");
    }
    path.push(format!("{name}.def"));
    let xml = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));

    let mut reader = Reader::from_str(&xml);
    let mut stack: Vec<String> = Vec::new();
    let (mut parent, mut implements, mut methods) = (None, Vec::new(), Vec::new());
    loop {
        match reader.read_event().expect("well-formed def") {
            Event::Start(e) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                // A direct child of the root's <ClientMethods> is one method.
                if stack.len() == 2 && stack[1] == "ClientMethods" {
                    methods.push(tag.clone());
                }
                stack.push(tag);
            }
            Event::Empty(e) => {
                if stack.len() == 2 && stack[1] == "ClientMethods" {
                    methods.push(String::from_utf8_lossy(e.name().as_ref()).into_owned());
                }
            }
            Event::Text(t) => {
                let text = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                if text.is_empty() {
                    continue;
                }
                match stack.iter().map(String::as_str).collect::<Vec<_>>()[1..] {
                    ["Parent"] => parent = Some(text),
                    ["Implements", "Interface"] => implements.push(text),
                    _ => {}
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    (parent, implements, methods)
}

/// Flattened client-method table in BigWorld parse order: `<Parent>`
/// recursively, then `<Implements>` in XML order, then the def's own
/// `<ClientMethods>` (entity-property-sync spec §1.1).
fn client_method_table(name: &str, interface: bool) -> Vec<String> {
    let (parent, implements, own) = read_def(name, interface);
    let mut table = Vec::new();
    if let Some(parent) = parent {
        table.extend(client_method_table(&parent, false));
    }
    for iface in implements {
        table.extend(client_method_table(&iface, true));
    }
    table.extend(own);
    table
}

#[test]
fn sgwgmplayer_shares_sgwplayers_idbase() {
    let player = client_method_table("SGWPlayer", false);
    let gm = client_method_table("SGWGmPlayer", false);

    assert_eq!(
        idbase_from_exposed_method_count(player.len()),
        IDBASE_SGW_PLAYER
    );
    assert_eq!(
        idbase_from_exposed_method_count(gm.len()),
        IDBASE_SGW_PLAYER,
        "SGWGmPlayer ({} client methods) left SGWPlayer's idBase bucket: every \
         method >= idBase on a GM ghost now mis-decodes on the witness",
        gm.len()
    );
}

#[test]
fn sgwplayers_client_methods_are_a_prefix_of_sgwgmplayers() {
    let player = client_method_table("SGWPlayer", false);
    let gm = client_method_table("SGWGmPlayer", false);

    assert!(
        gm.len() > player.len(),
        "SGWGmPlayer adds its own client methods"
    );
    assert_eq!(
        &gm[..player.len()],
        player.as_slice(),
        "a method index must mean the same thing on an SGWPlayer and an \
         SGWGmPlayer ghost; GM-only methods may only be appended"
    );
}
