use super::*;
use crate::test_capture::capture;

fn entry(size: u64, byte: u8) -> (u64, [u8; 32]) {
    (size, [byte; 32])
}

fn tree(entries: &[(&str, u64, u8)]) -> Tree {
    entries
        .iter()
        .map(|&(key, size, byte)| (key.to_owned(), entry(size, byte)))
        .collect()
}

fn field<'a>(fields: &'a [(&'static str, String)], key: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value.as_str())
}

#[test]
fn equal_trees_have_an_empty_diff_and_match() {
    let expected = tree(&[("Working/a.txt", 3, 1), ("Working/b.txt", 4, 2)]);
    let found = diff(&expected, &expected.clone(), &[]);
    assert_eq!(found, TreeDiff::default());
    assert!(found.is_empty());
    assert!(!found.blocking());
    assert_eq!(TreeOutcome::Matched.code(), "matched");
}

#[test]
fn a_case_only_spelling_of_the_same_bytes_is_not_blocking() {
    let expected = tree(&[("Working/SGWGame/a.txt", 3, 1)]);
    let actual = tree(&[("Working/sgwgame/a.txt", 3, 1)]);
    let found = diff(&expected, &actual, &[]);
    assert_eq!(
        found.case_only,
        vec![(
            "Working/SGWGame/a.txt".to_owned(),
            "Working/sgwgame/a.txt".to_owned()
        )]
    );
    assert!(found.missing.is_empty());
    assert!(found.changed.is_empty());
    assert!(found.unexpected.is_empty());
    assert!(!found.blocking());
    assert!(!found.is_empty());
}

#[test]
fn a_size_difference_is_changed_and_blocking() {
    let expected = tree(&[("a.txt", 3, 1)]);
    let actual = tree(&[("a.txt", 4, 1)]);
    let found = diff(&expected, &actual, &[]);
    assert_eq!(found.changed.len(), 1);
    assert_eq!(found.changed[0].path, "a.txt");
    assert_eq!(found.changed[0].expected, (3, [1; 32]));
    assert_eq!(found.changed[0].actual, (4, [1; 32]));
    assert!(found.blocking());
}

#[test]
fn a_digest_difference_with_the_same_size_is_changed() {
    let expected = tree(&[("a.txt", 3, 1)]);
    let actual = tree(&[("a.txt", 3, 9)]);
    let found = diff(&expected, &actual, &[]);
    assert_eq!(found.changed.len(), 1);
    assert_eq!(found.changed[0].actual, (3, [9; 32]));
    assert!(found.blocking());
}

#[test]
fn a_missing_expected_key_blocks_and_an_extra_actual_key_does_not() {
    let expected = tree(&[("a.txt", 3, 1), ("b.txt", 3, 2)]);
    let actual = tree(&[("a.txt", 3, 1)]);
    let found = diff(&expected, &actual, &[]);
    assert_eq!(found.missing, vec![("b.txt".to_owned(), 3, [2; 32])]);
    assert!(found.blocking());

    let extra = diff(&actual, &expected, &[]);
    assert_eq!(extra.unexpected, vec![("b.txt".to_owned(), 3, [2; 32])]);
    assert!(!extra.blocking());
}

#[test]
fn a_presence_only_key_ignores_its_digest_but_not_its_absence() {
    let expected = tree(&[("Working/Launcher.exe", 3, 1), ("b.txt", 3, 2)]);
    let presence = ["working/launcher.exe"];
    let present = tree(&[("Working/Launcher.exe", 3, 9), ("b.txt", 3, 2)]);
    assert!(diff(&expected, &present, &presence).is_empty());

    let absent = tree(&[("b.txt", 3, 2)]);
    let found = diff(&expected, &absent, &presence);
    assert_eq!(found.missing.len(), 1);
    assert!(found.blocking());
}

#[test]
fn a_case_spelling_with_other_bytes_is_changed_under_the_expected_path() {
    let expected = tree(&[("a/B.txt", 3, 1)]);
    let actual = tree(&[("A/b.txt", 5, 2)]);
    let found = diff(&expected, &actual, &[]);
    assert_eq!(found.changed.len(), 1);
    assert_eq!(found.changed[0].path, "a/B.txt");
    assert!(found.case_only.is_empty());
    assert!(found.blocking());
}

#[test]
fn keys_that_fold_together_never_panic_and_the_extra_one_is_missing() {
    let expected = tree(&[("A/x", 1, 1), ("a/x", 2, 2)]);
    let actual = tree(&[("a/X", 1, 1)]);
    let found = diff(&expected, &actual, &[]);
    assert_eq!(found.case_only, vec![("A/x".to_owned(), "a/X".to_owned())]);
    assert_eq!(found.missing, vec![("a/x".to_owned(), 2, [2; 32])]);
}

#[test]
fn five_missing_entries_show_three_samples() {
    let expected = tree(&[
        ("a1", 1, 1),
        ("a2", 1, 1),
        ("a3", 1, 1),
        ("a4", 1, 1),
        ("a5", 1, 1),
    ]);
    let found = diff(&expected, &Tree::new(), &[]);
    let fields = tree_checked_fields("light", TreeOutcome::Mismatch, Some(&found));
    assert_eq!(field(&fields, "missing"), Some("5"));
    let samples = field(&fields, "missing_paths").expect("missing samples");
    assert_eq!(samples.split("; ").count(), 3);
}

#[test]
fn a_changed_sample_shows_both_sizes_and_twelve_hex_digits() {
    let expected = tree(&[("a.txt", 10, 0xab)]);
    let actual = tree(&[("a.txt", 12, 0xcd)]);
    let found = diff(&expected, &actual, &[]);
    let fields = tree_checked_fields("verified", TreeOutcome::Mismatch, Some(&found));
    assert_eq!(
        field(&fields, "changed_paths"),
        Some("a.txt 10>12 abababababab>cdcdcdcdcdcd")
    );
    assert_eq!(field(&fields, "check"), Some("verified"));
    assert_eq!(field(&fields, "outcome"), Some("mismatch"));
}

#[test]
fn every_sample_value_stays_under_230_bytes() {
    let keys: Vec<String> = (0..5).map(|i| format!("{i}{}", "ö".repeat(100))).collect();
    let expected: Tree = keys.iter().map(|key| (key.clone(), entry(1, 1))).collect();
    let found = diff(&expected, &Tree::new(), &[]);
    let fields = tree_checked_fields("light", TreeOutcome::Mismatch, Some(&found));
    for (name, value) in &fields {
        assert!(value.len() <= 230, "{name} is {} bytes", value.len());
    }
    let samples = field(&fields, "missing_paths").expect("missing samples");
    assert_eq!(samples.len(), 1 + 200 + 1 + 1 + 1 + 12);
}

#[test]
fn an_over_long_first_sample_is_cut_on_a_char_boundary() {
    let key = format!("x{}", "ö".repeat(150));
    let expected: Tree = [(key, entry(1, 1))].into_iter().collect();
    let found = diff(&expected, &Tree::new(), &[]);
    let fields = tree_checked_fields("light", TreeOutcome::Mismatch, Some(&found));
    let samples = field(&fields, "missing_paths").expect("missing samples");
    // Byte 230 falls inside a two-byte `ö`, so the cut stops at 229.
    assert_eq!(samples.len(), 229);
    assert!(samples.starts_with('x'));
}

#[test]
fn a_mismatch_logs_an_error_on_the_adoption_target() {
    let expected = tree(&[("a.txt", 3, 1)]);
    let found = diff(&expected, &Tree::new(), &[]);
    let ((), events) = capture(|| log_tree_checked("light", TreeOutcome::Mismatch, Some(&found)));
    let event = events
        .iter()
        .find(|e| e.field("event") == Some("adoption_tree_checked"))
        .expect("adoption_tree_checked event");
    assert_eq!(event.level, Level::ERROR);
    assert_eq!(event.target, "adoption");
    assert_eq!(event.field("outcome"), Some("mismatch"));
    assert_eq!(event.field("missing"), Some("1"));
    assert_eq!(event.field("missing_paths"), Some("a.txt 3 010101010101"));
}

#[test]
fn an_unrecorded_check_warns_without_count_fields() {
    let ((), events) = capture(|| log_tree_checked("light", TreeOutcome::Unrecorded, None));
    let event = events
        .iter()
        .find(|e| e.field("event") == Some("adoption_tree_checked"))
        .expect("adoption_tree_checked event");
    assert_eq!(event.level, Level::WARN);
    assert_eq!(event.target, "adoption");
    assert_eq!(event.field("outcome"), Some("unrecorded"));
    assert_eq!(event.field("missing"), None);
    assert_eq!(event.field("missing_paths"), None);
}
