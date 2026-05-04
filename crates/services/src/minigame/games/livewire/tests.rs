//! Tests for `LivewireGame` — extracted to a sibling file to keep
//! the production module separate from the tests. (The production
//! `livewire/mod.rs` is still marginally over the 700-line hard cap
//! because the Python port itself runs ~595 lines plus Rust
//! boilerplate; bringing it fully under is a separate refactor PR.
//! Splitting tests out is a strict improvement either way.)
//!
//! The test module is a child of `livewire` so private fields
//! (`difficulty`, `goal_total`, `wires`, etc.) are reachable
//! directly without per-field `#[cfg(test)]` accessors leaking into
//! the production file.

use super::*;

fn make_session(difficulty: u32, tech: u32, level: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        game_name: "livewire".to_string(),
        difficulty,
        tech_competency: tech,
        seed: 0xDEADBEEF,
        abilities_mask: 0,
        intelligence: 0,
        player_level: level,
        ticket: String::new(),
        on_victory_chains: vec![],
        created_at: std::time::Instant::now(),
    }
}

/// `LivewireGame::new` clamps difficulty to [1, 4]. Pin so a
/// future server bug (or malicious client) sending difficulty=0
/// or difficulty=99 can't index out-of-bounds in
/// DIFFICULTY_LEVELS during update_difficulty().
#[test]
fn new_clamps_difficulty_below_one() {
    let g = LivewireGame::new(&make_session(0, 50, 5));
    assert_eq!(g.difficulty, 1);
}

#[test]
fn new_clamps_difficulty_above_four() {
    let g = LivewireGame::new(&make_session(99, 50, 5));
    assert_eq!(g.difficulty, 4);
}

#[test]
fn new_preserves_difficulty_within_range() {
    let g = LivewireGame::new(&make_session(3, 50, 5));
    assert_eq!(g.difficulty, 3);
}

/// `init_game` populates the wire grid via setup_wires. After
/// init the goal_total must equal the difficulty's seeded count
/// (2 / 4 / 4 / 6 for difficulties 1..4) and the wire HashMap
/// must be non-empty.
#[test]
fn init_game_populates_wires_and_goals_per_difficulty() {
    for (difficulty, expected_goals) in [(1u32, 2u32), (2, 4), (3, 4), (4, 6)] {
        let mut g = LivewireGame::new(&make_session(difficulty, 30, 5));
        g.init_game();
        assert_eq!(
            g.goal_total, expected_goals,
            "difficulty {difficulty} should seed goal_total {expected_goals}"
        );
        assert!(!g.wires.is_empty(), "init_game must populate the wire grid");
    }
}

/// `init_game` sets read_out to "<level_prefix><tech_competency>".
/// Pin the prefix so a refactor that drops the per-difficulty
/// label can't silently change what the player sees on the HUD.
#[test]
fn init_game_sets_read_out_prefix_per_difficulty() {
    for (difficulty, prefix) in [(1u32, "I-"), (2, "S-"), (3, "C-"), (4, "E-")] {
        let mut g = LivewireGame::new(&make_session(difficulty, 42, 5));
        g.init_game();
        assert_eq!(
            g.read_out,
            format!("{prefix}42"),
            "difficulty {difficulty} prefix must be {prefix}"
        );
    }
}

/// `session.seed` must thread through into the `StdRng` instance
/// used by the layout generator. Pin via two complementary checks
/// that are BOTH deterministic (no flakiness):
///
/// 1. Same seed → identical layout. If this fails, the RNG isn't
///    deterministic at all.
/// 2. The two specific seeds 1 and 2 produce distinguishable
///    layouts when init_game is run. If a regression always seeds
///    `StdRng` from a constant (ignoring `session.seed`), both
///    layouts collapse to identical and assertion #2 fails. The
///    seeds are hand-picked so the layouts ARE distinct under the
///    current implementation — re-pick if RNG semantics change.
#[test]
fn session_seed_threads_through_into_layout_generator() {
    use std::collections::BTreeMap;

    fn layout_for(seed: u32) -> BTreeMap<u32, String> {
        let mut s = make_session(4, 50, 5);
        s.seed = seed;
        let mut g = LivewireGame::new(&s);
        g.init_game();
        g.wires.iter().map(|(&d, w)| (d, w.lib.clone())).collect()
    }

    // (1) Determinism: identical seed → identical layout.
    assert_eq!(
        layout_for(1),
        layout_for(1),
        "same seed must produce identical layout"
    );

    // (2) Seed actually drives layout: two hand-picked seeds whose
    //     layouts are known to differ under the current RNG.
    assert_ne!(
        layout_for(1),
        layout_for(2),
        "seeds 1 and 2 must produce distinguishable layouts — \
         a regression that ignores session.seed would collapse them"
    );
}

/// `started()` runs init_game and returns one Send carrying the
/// full game-state SfsObject (timer/playfield/wire fields visible
/// to the client). Pin both the return shape AND that the payload
/// has the headline `_cmd=fullgamestate` field — without that, the
/// test would only verify a Send variant, not that it's the
/// initial-state payload.
#[test]
fn started_returns_exactly_one_send_with_full_game_state() {
    let mut g = LivewireGame::new(&make_session(2, 30, 5));
    let outputs = g.started();
    assert_eq!(outputs.len(), 1);
    let GameOutput::Send(payload) = &outputs[0] else {
        panic!("expected GameOutput::Send, got {:?}", outputs[0]);
    };
    let cmd = payload
        .get("_cmd")
        .and_then(|v| v.as_str())
        .expect("started() payload must carry the _cmd field");
    assert_eq!(
        cmd, "fullgamestate",
        "started() must emit the fullgamestate cmd carrying the initial state"
    );
}
