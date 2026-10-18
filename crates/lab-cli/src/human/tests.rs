//! Unit tests for the JSONL row reader and the friction clustering. Inline JSONL, no files.

use super::friction::{cluster_friction, PERSONAL_KEYS};
use super::rows::{parse_jsonl, Row};

/// Parses one inline JSONL line into a row.
fn row(line: &str) -> Row {
    parse_jsonl(line).expect("valid JSONL").remove(0)
}

/// Numeric fields may arrive as JSON strings; reverting the string parse loses them.
#[test]
fn numbers_may_be_strings() {
    let r = row(r#"{"radius_m":"4.15","n":3}"#);
    assert_eq!(r.f64("radius_m"), Some(4.15));
    assert_eq!(r.i64("n"), Some(3));
}

/// A bad line must name its line number, or a corrupt export is impossible to locate.
#[test]
fn bad_line_names_its_number() {
    let text = "{\"signal\":\"a\"}\n{oops\n";
    let err = parse_jsonl(text).expect_err("second line is not JSON");
    assert!(err.contains("line 2"), "{err}");
}

/// Blank lines between exported rows are skipped, not parsed as errors.
#[test]
fn blank_lines_are_skipped() {
    let text = "{\"signal\":\"a\"}\n\n   \n{\"signal\":\"b\"}\n";
    let rows = parse_jsonl(text).expect("blank lines are fine");
    assert_eq!(rows.len(), 2);
}

/// Windows tools prefix redirected output with a byte-order mark; without stripping it the first line fails to parse.
#[test]
fn a_leading_byte_order_mark_is_ignored() {
    let rows =
        parse_jsonl("\u{feff}{\"signal\":\"a\"}\r\n{\"signal\":\"b\"}\r\n").expect("BOM, CRLF");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].str("signal"), Some("a"));
}

/// A line that is valid JSON but not an object is an error naming its line.
#[test]
fn a_non_object_line_is_an_error() {
    let err = parse_jsonl("{\"signal\":\"a\"}\n[1,2]\n").expect_err("array line");
    assert!(err.contains("line 2"), "{err}");
}

/// Rows without a `player_id` each count as their own player, so two anonymous rows are two players.
#[test]
fn rows_without_a_player_id_count_as_separate_players() {
    let text = concat!(
        r#"{"signal":"s","cimmeria.session_kind":"player"}"#,
        "\n",
        r#"{"signal":"s","cimmeria.session_kind":"player"}"#,
        "\n",
        r#"{"signal":"s","cimmeria.session_kind":"player","player_id":7}"#,
        "\n",
        r#"{"signal":"s","cimmeria.session_kind":"player","player_id":"7"}"#,
        "\n",
    );
    let clusters = cluster_friction(&parse_jsonl(text).expect("valid JSONL"));
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].count, 4);
    assert_eq!(clusters[0].players, 3);
    assert_eq!(clusters[0].subject, "-");
    assert_eq!(clusters[0].world, "-");
}

/// Clusters must key on signal, world and subject, and drop lab rows; reverting any of these changes the count.
#[test]
fn friction_clusters_by_signal_world_subject() {
    let text = concat!(
        r#"{"signal":"repeat_interact_no_effect","reason":"no_effect","world":"Agnos","tag":"Frost","player_id":"p1","cimmeria.session_kind":"player","timestamp":"2026-10-01T10:00:00Z"}"#,
        "\n",
        r#"{"signal":"repeat_interact_no_effect","reason":"no_effect","world":"Agnos","tag":"Frost","player_id":"p1","cimmeria.session_kind":"player","timestamp":"2026-10-01T10:05:00Z"}"#,
        "\n",
        r#"{"signal":"repeat_interact_no_effect","reason":"no_effect","world":"Agnos","tag":"Frost","player_id":"p2","cimmeria.session_kind":"player","timestamp":"2026-10-01T09:00:00Z"}"#,
        "\n",
        r#"{"signal":"step_stalled","reason":"-","world":"Agnos","step_id":77,"player_id":"p1","cimmeria.session_kind":"player","timestamp":"2026-10-01T11:00:00Z"}"#,
        "\n",
        r#"{"signal":"step_stalled","reason":"-","world":"Agnos","step_id":77,"player_id":"p3","cimmeria.session_kind":"player","timestamp":"2026-10-01T11:30:00Z"}"#,
        "\n",
        r#"{"signal":"repeat_interact_no_effect","reason":"no_effect","world":"Agnos","tag":"Frost","player_id":"p9","cimmeria.session_kind":"lab","timestamp":"2026-10-01T12:00:00Z"}"#,
        "\n",
    );
    let clusters = cluster_friction(&parse_jsonl(text).expect("valid JSONL"));
    assert_eq!(clusters.len(), 2);
    let frost = &clusters[0];
    assert_eq!(frost.signal, "repeat_interact_no_effect");
    assert_eq!(frost.subject, "Frost");
    assert_eq!(frost.count, 3);
    assert_eq!(frost.players, 2);
    assert_eq!(frost.first_seen.as_deref(), Some("2026-10-01T09:00:00Z"));
    assert_eq!(frost.last_seen.as_deref(), Some("2026-10-01T10:05:00Z"));
    let stalled = &clusters[1];
    assert_eq!(stalled.subject, "77");
    assert_eq!(stalled.count, 2);
    assert_eq!(stalled.players, 2);
}

/// Personal names must not survive into a cluster sample; keeping them leaks the player into reports.
#[test]
fn sample_drops_personal_names() {
    let text = r#"{"signal":"step_stalled","world":"Agnos","step_id":"5","player_id":"p1","account_name":"acct","player_name":"Bob","entity_name":"Bobby","target_entity_name":"Guard","cimmeria.session_kind":"player","timestamp":"t1"}"#;
    let clusters = cluster_friction(&parse_jsonl(text).expect("valid JSONL"));
    assert_eq!(clusters.len(), 1);
    for key in PERSONAL_KEYS {
        assert!(
            clusters[0].sample.str(key).is_none(),
            "{key} leaked into sample"
        );
        assert!(
            clusters[0].sample.0.get(*key).is_none(),
            "{key} leaked into sample"
        );
    }
    assert_eq!(clusters[0].sample.str("signal"), Some("step_stalled"));
}
