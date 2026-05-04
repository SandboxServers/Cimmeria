//! Tests for `LivewireGame` — extracted to a sibling file to keep
//! `livewire/mod.rs` under the 700-line hard cap. The test module
//! is a child of `livewire` so private fields (`difficulty`,
//! `goal_total`, `wires`, etc.) are reachable directly without
//! per-field `#[cfg(test)]` accessors leaking into the production
//! file.

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

/// Distinct seeds must produce distinct wire layouts. Pin so a
/// regression that always seeds StdRng from a constant (or forgets
/// to thread `session.seed` through) can't strip per-room replay
/// variability.
///
/// The test compares wire COUNTS-per-library (a derivable property)
/// rather than the full depth→lib map, so it doesn't tightly couple
/// to the private `wires` HashMap layout. With ~10 distinct library
/// strings drawn from the seeded random pools, two distinct seeds
/// produce different counts with overwhelming probability — and the
/// test fails deterministically (not flakily) if the seed isn't
/// being used at all.
#[test]
fn different_seeds_produce_different_wire_layouts() {
    let mut s1 = make_session(4, 50, 5);
    s1.seed = 1;
    let mut s2 = make_session(4, 50, 5);
    s2.seed = 2;

    let mut g1 = LivewireGame::new(&s1);
    let mut g2 = LivewireGame::new(&s2);
    g1.init_game();
    g2.init_game();

    use std::collections::BTreeMap;
    let counts1: BTreeMap<&str, usize> = g1.wires.values().fold(BTreeMap::new(), |mut acc, w| {
        *acc.entry(w.lib.as_str()).or_insert(0) += 1;
        acc
    });
    let counts2: BTreeMap<&str, usize> = g2.wires.values().fold(BTreeMap::new(), |mut acc, w| {
        *acc.entry(w.lib.as_str()).or_insert(0) += 1;
        acc
    });
    assert_ne!(
        counts1, counts2,
        "different seeds must yield different per-library wire counts"
    );
}

/// `started()` runs init_game and returns one Send carrying the
/// full game-state SfsObject (timer/playfield/wire fields visible
/// to the client). Pin both the return shape AND that the payload
/// has the headline `_cmd=fullupdate` field — without that, the
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
