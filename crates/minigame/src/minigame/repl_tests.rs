use super::*;

/// A `Send` output with the given fields and `_cmd`.
fn send(cmd: &str, fields: Vec<(&str, SfsValue)>) -> GameOutput {
    let mut vars: HashMap<String, SfsValue> = fields
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    vars.insert("_cmd".to_string(), SfsValue::String(cmd.to_string()));
    GameOutput::Send(vars)
}

#[test]
fn parse_value_keeps_non_finite_words_as_strings() {
    for word in ["inf", "NaN", "infinity", "-inf"] {
        assert_eq!(parse_value(word).as_str(), Some(word));
    }
    assert!(matches!(parse_value("1.5"), SfsValue::Number(_)));
}

#[test]
fn render_sorts_keys_and_nests_objects() {
    let board = SfsValue::list([SfsValue::String("x".into()), SfsValue::Number(0.5)]);
    let out = send(
        "moveresult",
        vec![
            ("timeRemaining", SfsValue::Number(120.0)),
            ("board", board),
            ("ok", SfsValue::Bool(true)),
        ],
    );
    assert_eq!(
        render(&out),
        "< moveresult {board={0=x, 1=0.5}, ok=true, timeRemaining=120}"
    );
}

#[test]
fn render_command_without_fields_has_no_braces() {
    assert_eq!(render(&send("resetgame", vec![])), "< resetgame");
}

#[test]
fn render_terminal_outcomes() {
    assert_eq!(render(&GameOutput::Victory), "< victory");
    assert_eq!(render(&GameOutput::Failure), "< failure");
}

#[test]
fn parse_command_types_each_value() {
    let (cmd, params) = parse_command("processmove current=12 flag=true name=abc").unwrap();
    assert_eq!(cmd, "processmove");
    assert_eq!(params["current"].as_f64(), Some(12.0));
    assert!(matches!(params["flag"], SfsValue::Bool(true)));
    assert_eq!(params["name"].as_str(), Some("abc"));
}

#[test]
fn parse_command_rejects_a_bare_word() {
    assert!(parse_command("processmove current").is_err());
}

#[test]
fn send_unknown_command_gives_no_lines_and_no_error() {
    let mut repl = Repl::new("Livewire", 1, 7).unwrap();
    repl.start();
    assert_eq!(repl.send("noSuchCmd"), Ok(Vec::new()));
}

#[test]
fn send_rejects_a_malformed_parameter() {
    let mut repl = Repl::new("Livewire", 1, 7).unwrap();
    repl.start();
    assert!(repl.send("processmove current").is_err());
}

#[test]
fn send_after_the_game_ends_is_ignored() {
    let mut repl = Repl::new("Livewire", 1, 7).unwrap();
    repl.start();
    assert!(repl.abort().is_empty(), "an abort sends nothing");
    assert_eq!(
        repl.send("processmove current=12 target=13"),
        Ok(Vec::new())
    );
}

#[test]
fn check_script_names_the_line_of_a_mismatch() {
    let script = "game Livewire difficulty 1 seed 7\nstart\n< fullgamestate {wrong=1}\n";
    let err = check_script(script).unwrap_err();
    assert!(err.starts_with("line 3:"), "{err}");
}

#[test]
fn check_script_names_the_line_of_an_unexpected_output() {
    let script = "game Livewire difficulty 1 seed 7\nstart\n";
    let err = check_script(script).unwrap_err();
    assert!(err.starts_with("line 2:"), "{err}");
}

#[test]
fn livewire_golden_checks_clean() {
    let golden = include_str!("../../tests/scripts/livewire/d1_seed7.mgs");
    assert_eq!(check_script(golden), Ok(()));
}

#[test]
fn run_script_echoes_inputs_and_outputs_in_order() {
    let transcript =
        run_script("# header\ngame Livewire difficulty 1 seed 7\nstart\nabort\n").unwrap();
    assert!(transcript
        .starts_with("# header\ngame Livewire difficulty 1 seed 7\nstart\n< fullgamestate {"));
    assert!(
        transcript.ends_with("abort\n"),
        "an abort sends nothing: {transcript}"
    );
}

#[test]
fn script_without_a_game_line_is_an_error() {
    assert!(run_script("start\n").is_err());
}
