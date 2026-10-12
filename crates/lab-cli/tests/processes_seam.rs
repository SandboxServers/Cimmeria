//! The test seam of the process lister: `CIMMERIA_LAB_TEST_SGW_PIDS` replaces
//! the `SGW.exe` list only while `CIMMERIA_LAB_HOME` is set.

mod support;

use cimmeria_lab_cli::processes::sgw_pids;
use support::{env_lock, EnvGuard};

const SEAM: &str = "CIMMERIA_LAB_TEST_SGW_PIDS";

#[test]
fn the_seam_lists_the_given_pids_when_a_lab_home_is_set() {
    let _lock = env_lock();
    let _env = EnvGuard::set(&[
        ("CIMMERIA_LAB_HOME", Some("seam-home")),
        (SEAM, Some("11, 22,x,33")),
    ]);
    assert_eq!(sgw_pids(), vec![11, 22, 33]);
}

#[test]
fn an_empty_seam_lists_no_sgw_at_all() {
    let _lock = env_lock();
    let _env = EnvGuard::set(&[("CIMMERIA_LAB_HOME", Some("seam-home")), (SEAM, Some(""))]);
    assert!(sgw_pids().is_empty());
}

#[test]
fn the_seam_is_ignored_without_a_lab_home() {
    let _lock = env_lock();
    for home in [None, Some("")] {
        let _env = EnvGuard::set(&[("CIMMERIA_LAB_HOME", home), (SEAM, Some("4000000001"))]);
        assert!(
            !sgw_pids().contains(&4_000_000_001),
            "a real shell must not be able to hide or invent clients (home {home:?})"
        );
    }
}
