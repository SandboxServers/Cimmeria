//! LX-17b: `lab uat` delegates nothing to the PowerShell script.

use std::fs;
use std::path::Path;

#[test]
fn uat_has_no_powershell_fallback_left() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("commands")
        .join("uat");
    let mut checked = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let file = entry.unwrap().path();
        if file.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&file).unwrap();
            assert!(
                !text.contains("fallback::run"),
                "{} still falls back",
                file.display()
            );
            assert!(
                !text.contains("needs_powershell"),
                "{} still decides on a fallback",
                file.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 10,
        "the uat folder was not scanned ({checked} files)"
    );
}
