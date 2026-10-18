//! Unit tests for the friction bug cards: the facts, the commented row, the names and the files.

use super::card::render_card;
use super::{run, Args};
use crate::commands::Ctx;
use crate::human::friction::{cluster_friction, FrictionCluster};
use crate::human::rows::parse_jsonl;
use crate::ui::Ui;

/// Three rows of one stuck cluster in praxis, two of a misclick cluster, one lag singleton.
const EXPORT: &str = concat!(
    r#"{"signal":"stuck","reason":"no_path","world":"praxis","tag":"door_a","cimmeria.session_kind":"player","player_id":"1","timestamp":"2026-10-12T14:00:00Z","player_name":"Alice","x":"12.5","z":"-3.0"}"#,
    "\n",
    r#"{"signal":"stuck","reason":"no_path","world":"praxis","tag":"door_a","cimmeria.session_kind":"player","player_id":"2","timestamp":"2026-10-12T14:05:00Z"}"#,
    "\n",
    r#"{"signal":"stuck","reason":"no_path","world":"praxis","tag":"door_a","cimmeria.session_kind":"player","player_id":"1","timestamp":"2026-10-12T14:09:00Z"}"#,
    "\n",
    r#"{"signal":"misclick","reason":"","world":"sgu","tag":"npc_b","cimmeria.session_kind":"player","player_id":"3","timestamp":"2026-10-12T15:00:00Z"}"#,
    "\n",
    r#"{"signal":"misclick","reason":"","world":"sgu","tag":"npc_b","cimmeria.session_kind":"player","player_id":"4","timestamp":"2026-10-12T15:01:00Z"}"#,
    "\n",
    r#"{"signal":"lag","reason":"","world":"sgu","tag":"x","cimmeria.session_kind":"player","player_id":"5","timestamp":"2026-10-12T16:00:00Z"}"#,
    "\n",
);

/// The clusters of the export, most frequent first.
fn clusters(text: &str) -> Vec<FrictionCluster> {
    cluster_friction(&parse_jsonl(text).expect("valid JSONL"))
}

/// The lines between the first ```toml fence and its closing fence.
fn toml_fence(card: &str) -> Vec<&str> {
    let mut inside = false;
    let mut lines = Vec::new();
    for line in card.lines() {
        if line == "```toml" {
            inside = true;
        } else if inside && line == "```" {
            return lines;
        } else if inside {
            lines.push(line);
        }
    }
    panic!("no toml fence in\n{card}");
}

/// A card holds the signal, world, subject and count, and every line of its toml fence is a comment.
#[test]
fn card_has_facts_and_a_commented_row() {
    let stuck = &clusters(EXPORT)[0];
    let card = render_card(stuck);
    assert!(card.contains("# Friction: stuck at door_a (praxis)"));
    assert!(card.contains("- signal: stuck"));
    assert!(card.contains("- world: praxis"));
    assert!(card.contains("- subject: door_a"));
    assert!(card.contains("- count: 3 rows"));
    let fence = toml_fence(&card);
    assert!(!fence.is_empty());
    for line in fence {
        assert!(line.starts_with("# "), "uncommented line: {line:?}");
    }
}

/// Only the listed keys leave a sample: a hand-added player_name never reaches the card.
#[test]
fn card_has_no_personal_names() {
    let mut stuck = clusters(EXPORT).remove(0);
    stuck
        .sample
        .0
        .insert("player_name".to_string(), serde_json::json!("Alice"));
    let card = render_card(&stuck);
    assert!(!card.contains("player_name"), "card:\n{card}");
    assert!(!card.contains("Alice"), "card:\n{card}");
    assert!(card.contains("- x: 12.5"), "card:\n{card}");
}

/// A newline or quote in an exported value must not leave an uncommented line in the toml fence or close it early.
#[test]
fn hostile_values_cannot_escape_the_commented_fence() {
    let mut stuck = clusters(EXPORT).remove(0);
    stuck.subject = "door\n```\nid = \"x\"\n\"quoted\"".to_string();
    stuck.sample.0.insert(
        "template_name".to_string(),
        serde_json::json!("npc\n## Expected"),
    );
    let card = render_card(&stuck);
    for line in toml_fence(&card) {
        assert!(line.starts_with("# "), "uncommented line: {line:?}");
    }
    let fence_lines = card.lines().filter(|l| l.starts_with("```")).count();
    assert_eq!(fence_lines, 2, "card:\n{card}");
    let expected_heads = card.lines().filter(|l| *l == "## Expected").count();
    assert_eq!(expected_heads, 1, "card:\n{card}");
    assert!(card.contains(r#"\"quoted\""#), "card:\n{card}");
}

/// An export that cannot be read exits 1 and writes nothing.
#[test]
fn a_missing_export_exits_one() {
    let dir = tempfile::tempdir().expect("temp dir");
    let out = dir.path().join("drafts");
    let args = Args {
        rows: dir.path().join("absent.jsonl"),
        out: out.clone(),
        top: 20,
        min_count: 2,
        json: false,
    };
    let ctx = Ctx {
        ui: Ui::plain(),
        command: "friction-drafts".into(),
        raw: vec![],
    };
    assert_eq!(run(&args, &ctx), 1);
    assert!(!out.exists(), "nothing is created for an unreadable export");
}

/// `--top` caps the number of cards, most frequent first.
#[test]
fn top_caps_the_cards() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rows = dir.path().join("friction.jsonl");
    std::fs::write(&rows, EXPORT).expect("write export");
    let out = dir.path().join("drafts");
    let args = Args {
        rows,
        out: out.clone(),
        top: 1,
        min_count: 1,
        json: true,
    };
    let ctx = Ctx {
        ui: Ui::plain(),
        command: "friction-drafts".into(),
        raw: vec![],
    };
    assert_eq!(run(&args, &ctx), 0);
    let names: Vec<_> = std::fs::read_dir(&out).expect("drafts folder").collect();
    assert_eq!(names.len(), 1);
}

/// Two clusters of at least two rows give two cards; the singleton gets none.
#[test]
fn writes_one_card_per_cluster() {
    let dir = tempfile::tempdir().expect("temp dir");
    let rows = dir.path().join("friction.jsonl");
    std::fs::write(&rows, EXPORT).expect("write export");
    let out = dir.path().join("drafts");
    let args = Args {
        rows,
        out: out.clone(),
        top: 20,
        min_count: 2,
        json: false,
    };
    let ctx = Ctx {
        ui: Ui::plain(),
        command: "friction-drafts".into(),
        raw: vec![],
    };
    assert_eq!(run(&args, &ctx), 0);
    let mut names: Vec<String> = std::fs::read_dir(&out)
        .expect("drafts folder")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names.len(), 2, "files: {names:?}");
    assert!(names[0].starts_with("bug-01-stuck"), "files: {names:?}");
    assert!(names[1].starts_with("bug-02-misclick"), "files: {names:?}");
}
