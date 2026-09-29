//! Squad chat (ORG-04): the line reaches every member of the speaker's
//! squad wherever they are, and nobody else; a speaker in no squad sends
//! nothing and reads one feedback line. Fan-out (TESTING.md type 8) against
//! the captured `CellToBaseMsg` stream, plus the `squad.chat` outcome rows
//! (type 12).

use super::super::*;
use super::decode_on_player_communication;
use crate::cell::squad::SquadMember;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
use cimmeria_cell_world::cell::squad::SquadResources;
use tracing::Level;

/// Two spaces, Agnos and Castle, both loaded.
fn two_spaces() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /><Space WorldName="Castle" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Castle" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// A connected player: entity `eid`, character `eid + 100`, account
/// `eid + 1000`, named `name`, in `world`.
fn add_player(mgr: &mut SpaceManager, eid: u32, name: &str, world: &str) {
    mgr.create_entity(eid, world, [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(eid);
    let e = mgr.get_entity_mut(eid).unwrap();
    e.player_id = Some(eid as i32 + 100);
    e.account_id = Some(eid + 1000);
    e.character_name = Some(name.to_owned());
}

fn member(mgr: &SpaceManager, eid: u32) -> SquadMember {
    let e = mgr.get_entity(eid).unwrap();
    SquadMember {
        player_id: e.player_id.unwrap(),
        name: e.character_name.clone().unwrap(),
        level: 1,
        archetype: 1,
    }
}

/// Alice (1) and Bob (2) in Agnos, Cara (3) in Castle, all in Alice's
/// squad; Dave (4) in Agnos, in no squad, and a witness of Alice.
fn world() -> (SpaceManager, i32) {
    let mut mgr = two_spaces();
    add_player(&mut mgr, 1, "Alice", "Agnos");
    add_player(&mut mgr, 2, "Bob", "Agnos");
    add_player(&mut mgr, 3, "Cara", "Castle");
    add_player(&mut mgr, 4, "Dave", "Agnos");
    for w in [2u32, 4] {
        mgr.get_entity_mut(1)
            .unwrap()
            .witnesses
            .insert(cimmeria_common::EntityId(w as i32));
    }
    let (alice, bob, cara) = (member(&mgr, 1), member(&mgr, 2), member(&mgr, 3));
    let sid = mgr
        .resources
        .squads_mut()
        .force_join(bob, alice.clone())
        .expect("join")
        .squad_id;
    mgr.resources
        .squads_mut()
        .force_join(cara, alice)
        .expect("join");
    (mgr, sid)
}

/// `(recipient, channel, text)` of every `onPlayerCommunication` queued.
fn lines(rx: &mut tokio::sync::mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, u8, String)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        else {
            panic!("only entity method calls are expected: {msg:?}");
        };
        assert_eq!(method_index, ON_PLAYER_COMMUNICATION);
        let (_, channel, text) = decode_on_player_communication(&args);
        out.push((entity_id, channel, text));
    }
    out
}

fn chat_rows(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "squad" && c.has_field("event", "squad.chat"))
        .collect()
}

/// The line reaches every member, in both spaces, including the speaker's
/// own copy, on the squad channel with the speaker's name; Dave, a witness
/// in the same space but no member, gets nothing.
#[tokio::test]
async fn squad_line_reaches_members_in_two_spaces_and_no_one_else() {
    let capture = LogCapture::install();
    let (mut mgr, sid) = world();
    assert_ne!(
        mgr.get_entity(2).map(|e| e.space_id),
        mgr.get_entity(3).map(|e| e.space_id),
        "the fixture must put Bob and Cara in different spaces"
    );
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let engine = ChainEngine::new();

    handle_chat_message(1, "Alice", 0, CHAN_SQUAD, "Regroup", &tx, &mut mgr, &engine).await;

    let mut got = lines(&mut rx);
    got.sort();
    let want: Vec<(u32, u8, String)> = [1, 2, 3]
        .into_iter()
        .map(|e| (e, CHAN_SQUAD, "Regroup".to_owned()))
        .collect();
    assert_eq!(got, want);

    let rows = chat_rows(&capture);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = &rows[0];
    assert_eq!(row.level, Level::INFO);
    for (k, v) in [
        ("outcome", "ok"),
        ("player_id", "101"),
        ("account_id", "1001"),
        ("recipients", "2"),
        ("text_units", "7"),
        ("squad_id", &sid.to_string()),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {:?}", row.fields);
    }
    assert!(!row.fields.contains_key("reason"), "{:?}", row.fields);
}

/// A speaker in no squad sends nothing to anyone else and reads one line.
#[tokio::test]
async fn non_member_sends_nothing_and_gets_feedback() {
    let capture = LogCapture::install();
    let (mut mgr, _) = world();
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let engine = ChainEngine::new();

    handle_chat_message(4, "Dave", 0, CHAN_SQUAD, "Anyone?", &tx, &mut mgr, &engine).await;

    let got = lines(&mut rx);
    assert_eq!(
        got,
        vec![(4, CHAN_FEEDBACK, squad::NOT_IN_SQUAD_TEXT.to_owned())]
    );
    let rows = chat_rows(&capture);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    for (k, v) in [
        ("outcome", "rejected"),
        ("reason", "not_in_squad"),
        ("player_id", "104"),
        ("recipients", "0"),
        ("text_units", "7"),
    ] {
        assert!(rows[0].has_field(k, v), "{k}={v}: {:?}", rows[0].fields);
    }
}

/// A member in gate transit (no live entity) misses the line; everyone
/// else still gets it, and `recipients` counts only the delivered copies.
#[tokio::test]
async fn member_in_transit_is_skipped() {
    let capture = LogCapture::install();
    let (mut mgr, _) = world();
    mgr.destroy_entity(3);
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let engine = ChainEngine::new();

    handle_chat_message(1, "Alice", 0, CHAN_SQUAD, "Hi", &tx, &mut mgr, &engine).await;

    let mut got: Vec<u32> = lines(&mut rx).into_iter().map(|l| l.0).collect();
    got.sort();
    assert_eq!(got, vec![1, 2]);
    assert!(chat_rows(&capture)[0].has_field("recipients", "1"));
}

/// A line that cannot be queued warns once per recipient on `squad`.
#[tokio::test]
async fn dropped_send_warns() {
    let capture = LogCapture::install();
    let (mut mgr, _) = world();
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    drop(rx);
    let engine = ChainEngine::new();

    handle_chat_message(1, "Alice", 0, CHAN_SQUAD, "Hi", &tx, &mut mgr, &engine).await;

    let warns: Vec<Captured> = capture
        .all()
        .into_iter()
        .filter(|c| {
            c.level == Level::WARN
                && c.target == "squad"
                && c.has_field("event", "squad.send_failed")
                && c.has_field("reason", "cell_to_base_closed")
        })
        .collect();
    assert_eq!(warns.len(), 3, "{warns:#?}");
    assert!(chat_rows(&capture)[0].has_field("recipients", "0"));
}
