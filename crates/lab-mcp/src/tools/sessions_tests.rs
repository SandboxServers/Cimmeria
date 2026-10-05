//! NT-41: `server_sessions` pairs every id with its name (Rule 6) and leaves
//! a name the session does not hold out. `name` and `zone` stay.

use cimmeria_services::base::OnlinePlayer;

use super::shape;

fn player() -> OnlinePlayer {
    OnlinePlayer {
        id: 100,
        name: "Tealc".to_string(),
        player_id: Some(42),
        account_id: 7,
        account_name: Some("tester".to_string()),
        archetype: "Soldier",
        level: 5,
        zone: "Castle_CellBlock".to_string(),
        ping: None,
        status: "in_world",
        session: "127.0.0.1:50000".to_string(),
    }
}

#[test]
fn nt41_sessions_pair_every_id_with_its_name() {
    let out = shape(&[player()]);
    assert_eq!(out["count"], 1);
    let s = &out["sessions"][0];
    assert_eq!(s["entity_id"], 100);
    assert_eq!(s["entity_name"], "Tealc");
    assert_eq!(s["player_id"], 42);
    assert_eq!(s["player_name"], "Tealc");
    assert_eq!(s["account_id"], 7);
    assert_eq!(s["account_name"], "tester");
    assert_eq!(s["world"], "Castle_CellBlock");
    // Pre-Rule-6 keys the UAT runner and older prompts read.
    assert_eq!(s["name"], "Tealc");
    assert_eq!(s["zone"], "Castle_CellBlock");
    assert_eq!(s["session"], "127.0.0.1:50000");
}

#[test]
fn nt41_sessions_omit_names_the_session_does_not_hold() {
    let p = OnlinePlayer {
        name: String::new(),
        player_id: None,
        account_name: None,
        zone: String::new(),
        ..player()
    };
    let out = shape(&[p]);
    let s = &out["sessions"][0];
    assert_eq!(s["account_id"], 7, "the id stays without its name");
    for key in [
        "entity_name",
        "player_id",
        "player_name",
        "account_name",
        "world",
    ] {
        assert!(s.get(key).is_none(), "{key} must be absent, got {s}");
    }
}
