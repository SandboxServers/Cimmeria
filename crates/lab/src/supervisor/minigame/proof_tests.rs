use super::*;
use crate::proof::{EvVerdict, Evidence};

#[test]
fn real_minigame_needs_the_server() {
    // No board read: the server cannot corroborate, so inconclusive.
    let none = Evidence::judge(minigame_checks("Livewire", false, None, true, true));
    assert_eq!(none.verdict, EvVerdict::Inconclusive);
    assert_eq!(none.why, Why::ServerUnreachable);

    // A defeat on the board: denied.
    let defeat = Evidence::judge(minigame_checks(
        "Livewire",
        false,
        Some("defeat"),
        true,
        true,
    ));
    assert_eq!(defeat.verdict, EvVerdict::Denied);
    assert_eq!(defeat.why, Why::NoEffect);

    // A victory on the board: confirmed.
    let victory = Evidence::judge(minigame_checks(
        "Livewire",
        false,
        Some("victory"),
        true,
        true,
    ));
    assert_eq!(victory.verdict, EvVerdict::Confirmed);
}

#[test]
fn placeholder_minigame_is_client_judged() {
    // The window hid and the server is unreachable: confirmed, and the
    // optional server check is still shown to the reader.
    let ev = Evidence::judge(minigame_checks("Hack", true, None, true, true));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let server = ev
        .checks
        .iter()
        .find(|check| check.name == "server_outcome")
        .expect("optional server check present");
    assert!(!server.required);
    assert_eq!(server.ok, None);
}

#[test]
fn placeholder_window_still_open_is_denied() {
    let ev = Evidence::judge(minigame_checks(
        "Converse",
        true,
        Some("victory"),
        false,
        true,
    ));
    assert_eq!(ev.verdict, EvVerdict::Denied);
}

#[test]
fn placeholder_with_a_failed_action_is_denied() {
    // The window hid anyway (closed by the player), but the click sequence failed.
    let ev = Evidence::judge(minigame_checks("Hack", true, None, true, false));
    assert_eq!(ev.verdict, EvVerdict::Denied);
}
