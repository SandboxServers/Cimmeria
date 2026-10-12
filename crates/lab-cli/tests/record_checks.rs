//! `lab record` argument checks and the state file (ports the removed PowerShell test file,
//! "argument checks" and "state file path").

mod record_support;

use cimmeria_lab_cli::commands::record::checks::{check_arguments, USAGE};
use cimmeria_lab_cli::commands::record::state::{read_state, state_path};
use cimmeria_lab_cli::commands::record::Args;
use record_support::{args, ID};

fn msg(a: &Args) -> String {
    check_arguments(a).expect("an argument error")
}

// Check: 'no action prints the usage'
#[test]
fn no_action_prints_the_usage() {
    let text = msg(&Args::default());
    assert!(text.starts_with("usage: lab record"));
    assert_eq!(text, USAGE);
}

// Check: 'actions are lower case'
#[test]
fn actions_are_lower_case() {
    assert!(msg(&args("Start", Some("first-session"))).starts_with("usage:"));
}

// Check: 'start needs a section'
#[test]
fn start_needs_a_section() {
    assert!(msg(&args("start", None)).starts_with("start needs a section"));
}

// Check: 'a bad section is refused'
#[test]
fn a_bad_section_is_refused() {
    assert!(msg(&args("start", Some("First Session"))).starts_with("section:"));
}

// Check: 'a 49-character section is refused'
#[test]
fn a_49_character_section_is_refused() {
    let long = "a".repeat(49);
    assert!(msg(&args("start", Some(&long))).starts_with("section:"));
}

// Check: 'a good start passes'
#[test]
fn a_good_start_passes() {
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p2".into());
    a.max_minutes = Some("30".into());
    assert_eq!(check_arguments(&a), None);
}

// Check: '-MaxMinutes 0 is refused'
#[test]
fn max_minutes_0_is_refused() {
    let mut a = args("start", Some("fs"));
    a.max_minutes = Some("0".into());
    assert_eq!(msg(&a), "-MaxMinutes must be 1 to 30");
}

// Check: '-MaxMinutes 31 is refused'
#[test]
fn max_minutes_31_is_refused() {
    let mut a = args("start", Some("fs"));
    a.max_minutes = Some("31".into());
    assert_eq!(msg(&a), "-MaxMinutes must be 1 to 30");
}

// Check: '-MaxMinutes abc is refused'
#[test]
fn max_minutes_abc_is_refused() {
    let mut a = args("start", Some("fs"));
    a.max_minutes = Some("abc".into());
    assert_eq!(msg(&a), "-MaxMinutes must be 1 to 30");
}

// Check: 'an empty -MaxMinutes is not given'
#[test]
fn an_empty_max_minutes_is_not_given() {
    let mut a = args("start", Some("fs"));
    a.max_minutes = Some(String::new());
    assert_eq!(check_arguments(&a), None);
}

// Check: 'a path-like instance is refused'
#[test]
fn a_path_like_instance_is_refused() {
    let mut a = args("status", None);
    a.instance = Some(r"..\x".into());
    assert!(msg(&a).starts_with("-Instance:"));
}

// Check: 'a quoting character_expr is refused'
#[test]
fn a_quoting_character_expr_is_refused() {
    let mut a = args("stop", None);
    a.character_expr = Some("x'; --".into());
    assert!(msg(&a).starts_with("-CharacterExpr:"));
}

// Check: 'Px${run_id} passes'
#[test]
fn run_id_expression_passes() {
    let mut a = args("stop", None);
    a.character_expr = Some("Px${run_id}".into());
    assert_eq!(check_arguments(&a), None);
}

// Check: '-NoDraft on start is refused'
#[test]
fn no_draft_on_start_is_refused() {
    let mut a = args("start", Some("fs"));
    a.no_draft = true;
    assert_eq!(msg(&a), "-NoDraft is for lab record stop");
}

// Check: 'status takes no positional argument'
#[test]
fn status_takes_no_positional_argument() {
    assert!(msg(&args("status", Some("x"))).starts_with("status takes no positional"));
}

// Check: 'transform needs a recording'
#[test]
fn transform_needs_a_recording() {
    assert!(msg(&args("transform", None)).starts_with("transform needs a recording"));
}

// Check: 'transform takes no -Instance'
#[test]
fn transform_takes_no_instance() {
    let mut a = args("transform", Some(ID));
    a.instance = Some("p2".into());
    assert!(msg(&a).starts_with("transform runs on no instance"));
}

// Check: 'transform refuses a bad -Section'
#[test]
fn transform_refuses_a_bad_section() {
    let mut a = args("transform", Some(ID));
    a.section = Some("Bad_Section".into());
    assert!(msg(&a).starts_with("-Section:"));
}

// Check: '-SpecsDir on status is refused'
#[test]
fn specs_dir_on_status_is_refused() {
    let mut a = args("status", None);
    a.specs_dir = Some(r"C:\x".into());
    assert!(msg(&a).starts_with("-SpecsDir is for"));
}

// Check: 'the state file is record-<instance>.json in the lab home'
#[test]
fn the_state_file_is_record_instance_json_in_the_lab_home() {
    let home = std::path::Path::new("lab-home");
    assert_eq!(state_path(home, "p2"), Ok(home.join("record-p2.json")));
}

// Check: 'a bad instance never becomes a path'
#[test]
fn a_bad_instance_never_becomes_a_path() {
    let home = std::path::Path::new("lab-home");
    assert!(state_path(home, r"..\evil").is_err());
}

// Check: 'no state file reads as nothing'
#[test]
fn no_state_file_reads_as_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    assert_eq!(read_state(&dir.path().join("record-none.json")), None);
}

// Check: 'an unreadable state file reads as nothing'
#[test]
fn an_unreadable_state_file_reads_as_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("record-junk.json");
    std::fs::write(&path, "not json").expect("write");
    assert_eq!(read_state(&path), None);
}
