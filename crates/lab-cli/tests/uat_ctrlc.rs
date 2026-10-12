use cimmeria_lab_cli::commands::uat::ctrlc::{press, stop_requested};

#[test]
fn the_first_press_sets_stop_and_is_handled_later_ones_are_left_to_the_default() {
    assert!(!stop_requested());
    assert!(press(), "the first press is handled");
    assert!(stop_requested());
    assert!(
        !press(),
        "the second press is not handled: the process ends"
    );
}
