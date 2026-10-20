//! MG-LW1 parity tests: one per changed delta in `spec-livewire-parity.md`.
//! Each test fails against the pre-MG-LW1 value of its delta.

use super::*;

fn make_session(difficulty: u32, tech: u32) -> MinigameSession {
    MinigameSession {
        entity_id: 1,
        player_id: 100,
        session_instance_id: 0,
        game_name: "livewire".to_string(),
        difficulty,
        tech_competency: tech,
        seed: 0xDEADBEEF,
        abilities_mask: 0,
        intelligence: 0,
        player_level: 5,
        ticket: String::new(),
        on_victory_chains: vec![],
        on_failure_chains: vec![],
        player_name: None,
        client_ip: None,
        created_at: tokio::time::Instant::now(),
        connected: false,
        terminal_reported: false,
        cancel: tokio::sync::watch::channel(false).0,
    }
}

/// A game initialised from `seed`, so its board is the one the seed deals.
fn board(seed: u32, difficulty: u32, tech: u32) -> LivewireGame {
    let mut session = make_session(difficulty, tech);
    session.seed = seed;
    let mut g = LivewireGame::new(&session);
    g.init_game();
    g
}

/// The trailing digit of a wire lib (`pGray3` gives 3), which must be 1 to 4.
fn lib_variant(lib: &str) -> u32 {
    let digits = lib.len() - lib.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    assert_eq!(digits, 1, "a lib carries exactly one suffix digit: {lib}");
    lib[lib.len() - 1..].parse().unwrap()
}

/// Playfield and moving wires (`p`, `m`), not the electrodes (`pe`, `me`).
fn is_suffixed_wire(name: &str) -> bool {
    (name.starts_with('p') && !name.starts_with("pe"))
        || (name.starts_with('m') && !name.starts_with("me"))
}

/// Delta 1: every playfield and moving lib ends in a suffix of 1 to 4.
#[test]
fn playfield_and_moving_libs_carry_a_one_to_four_suffix() {
    let mut checked = 0;
    for seed in 0..50 {
        let g = board(seed, 2, 30);
        for w in g.wires.values().filter(|w| is_suffixed_wire(&w.name)) {
            assert!((1..=4).contains(&lib_variant(&w.lib)), "{}", w.lib);
            checked += 1;
        }
    }
    assert!(checked > 0, "the boards must contain suffixed wires");
}

/// Delta 2: goal and obstacle variants reach 4 (the old `1..4` never did).
#[test]
fn goal_and_obstacle_variants_reach_four() {
    let (mut saw_four, mut saw_one) = (false, false);
    for seed in 0..200 {
        let g = board(seed, 4, 30);
        for w in g
            .wires
            .values()
            .filter(|w| w.name.starts_with('g') || w.name.starts_with('o'))
        {
            let v = lib_variant(&w.lib);
            assert!((1..=4).contains(&v), "{}", w.lib);
            saw_four |= v == 4;
            saw_one |= v == 1;
        }
    }
    assert!(saw_four && saw_one, "variants 1 and 4 must both appear");
}

/// Delta 3: goal depths fall in the two bands, 1-25 or 60-85, and both show up.
#[test]
fn goal_depths_fall_in_the_two_bands() {
    let (mut low, mut high) = (false, false);
    for seed in 0..100 {
        let g = board(seed, 4, 30);
        for w in g.wires.values().filter(|w| w.name.starts_with('g')) {
            let in_low = (1..=25).contains(&w.depth);
            let in_high = (60..=85).contains(&w.depth);
            assert!(
                in_low || in_high,
                "goal depth {} is outside both bands",
                w.depth
            );
            low |= in_low;
            high |= in_high;
        }
    }
    assert!(low && high, "both bands must be drawn");
}

/// Delta 4: electrode x and y are whole numbers.
#[test]
fn electrode_positions_are_whole() {
    let mut checked = 0;
    for seed in 0..50 {
        let g = board(seed, 2, 30);
        for w in g
            .wires
            .values()
            .filter(|w| w.name.starts_with("pe") || w.name.starts_with("me"))
        {
            assert_eq!(w.x.fract(), 0.0, "electrode x {}", w.x);
            assert_eq!(w.y.fract(), 0.0, "electrode y {}", w.y);
            checked += 1;
        }
    }
    assert!(checked > 0, "the boards must contain electrodes");
}

/// Delta 5: the obstacle count is `floor(..) * 2`, so it is even and not zero.
#[test]
fn obstacle_total_is_doubled() {
    // (difficulty, tech competency, expected obstacle wires) = 2 * floor(base + tc * coef)
    for (difficulty, tc, expected) in [
        (1, 1, 4),
        (1, 6, 6),
        (2, 1, 4),
        (2, 6, 6),
        (3, 1, 6),
        (3, 6, 8),
        (4, 1, 6),
        (4, 6, 8),
        (4, 30, 20),
    ] {
        let g = board(0, difficulty, tc);
        let n = g.wires.values().filter(|w| w.name.starts_with('o')).count();
        assert_eq!(n, expected, "difficulty {difficulty} tc {tc}");
    }
}

/// Delta 6: `obstacle_timer = base + 5 * floor((tc - 1) / 5)`, base 5/6/7/9.
#[test]
fn obstacle_timer_steps_every_five_tech_competency() {
    for (difficulty, base) in [(1, 5.0), (2, 6.0), (3, 7.0), (4, 9.0)] {
        assert_eq!(board(0, difficulty, 1).obstacle_timer, base);
        assert_eq!(board(0, difficulty, 6).obstacle_timer, base + 5.0);
        assert_eq!(board(0, difficulty, 30).obstacle_timer, base + 25.0);
    }
}

/// Delta 7: D4 has no moving-wire divider, so its move timer is 0 and moving wires
/// leave the rate alone.
#[test]
fn d4_move_timer_is_zero() {
    assert_eq!(board(0, 4, 30).move_timer, 0.0);
}

/// Delta 8: timer method 0 is `(param_timer * tc) / 1.7`. Method 1 and 2 are kept.
/// At difficulty 1, tc 30 the param timer is floor(30 + 30 * 0.588235) = 47.
#[test]
fn timer_method_zero_multiplies_the_param_timer() {
    let method_zero = 47.0 * 30.0 / 1.7;
    let method_one = 17.0 + 30.0 * 4.5f64.powf(1.25);
    let mut saw_zero = false;
    for seed in 0..200 {
        let g = board(seed, 1, 30);
        let units = g.time_units;
        if (units - method_zero).abs() < 1e-9 {
            saw_zero = true;
        } else {
            assert!(
                (units - method_one).abs() < 1e-9 || (units - 62.0).abs() < 1e-9,
                "timer {units} matches no method"
            );
        }
    }
    assert!(saw_zero, "some seed must pick method 0 ({method_zero})");
}

/// Delta 11: `activateitemcheat` is refused with WARN and no output.
#[test]
fn item_cheat_is_refused_with_a_warning() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut session = make_session(1, 30);
    session.entity_id = 77;
    session.player_name = Some("Hacker".to_string());
    let mut g = LivewireGame::new(&session);
    g.init_game();
    let outputs = g.message("activateitemcheat", &HashMap::new());
    assert!(outputs.is_empty());
    let event = capture
        .find_message(Level::WARN, "Livewire: item cheats are not available")
        .expect("the item cheat must emit a WARN row");
    assert!(
        event.has_field("reason", "item_cheat_unavailable"),
        "{event:#?}"
    );
    assert!(event.has_field("entity_id", "77"), "{event:#?}");
    assert!(event.has_field("entity_name", "Hacker"), "{event:#?}");
}

/// Delta 11: `activateabilitycheat` is refused with WARN and no output.
#[test]
fn ability_cheat_is_refused_with_a_warning() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut session = make_session(1, 30);
    session.entity_id = 77;
    session.player_name = Some("Hacker".to_string());
    let mut g = LivewireGame::new(&session);
    g.init_game();
    let outputs = g.message("activateabilitycheat", &HashMap::new());
    assert!(outputs.is_empty());
    let event = capture
        .find_message(Level::WARN, "Livewire: ability cheats are not available")
        .expect("the ability cheat must emit a WARN row");
    assert!(
        event.has_field("reason", "ability_cheat_unavailable"),
        "{event:#?}"
    );
    assert!(event.has_field("entity_id", "77"), "{event:#?}");
    assert!(event.has_field("entity_name", "Hacker"), "{event:#?}");
}

/// Delta 14: `aborted()` sends nothing (the client has closed) and cancels the game.
#[test]
fn aborted_sends_nothing_and_cancels() {
    let mut g = board(0, 1, 30);
    g.game_started = true;
    assert!(g.aborted().is_empty(), "an abort must not send a failure");
    assert_eq!(g.outcome, Some("canceled"));
}

/// Delta 16: `timerState` in `fullgamestate` is the numeric live state, 0 at join.
#[test]
fn full_game_state_timer_state_is_a_number() {
    let g = board(0, 1, 30);
    match g.build_full_game_state().get("timerState") {
        Some(SfsValue::Number(n)) => assert_eq!(*n, 0.0),
        other => panic!("timerState must be a Number, got {other:?}"),
    }
}

/// Adjacent fix: a `p` wire the server ignores (playfield inactive) stays uncut.
#[test]
fn ignored_playfield_wire_stays_uncut() {
    let mut g = board(0, 1, 30);
    g.game_started = true;
    g.playfield_active = false;
    let (depth, name) = g
        .wires
        .iter()
        .find(|(_, w)| w.name.starts_with('p'))
        .map(|(d, w)| (*d, w.name.clone()))
        .expect("the board must contain a playfield wire");

    let mut params = HashMap::new();
    params.insert("wirename".to_string(), SfsValue::String(name));
    let outputs = g.message("processmove", &params);
    assert!(outputs.is_empty(), "an inactive playfield wire is ignored");
    assert!(
        !g.wires[&depth].cut,
        "an ignored playfield wire must stay uncut"
    );
}
