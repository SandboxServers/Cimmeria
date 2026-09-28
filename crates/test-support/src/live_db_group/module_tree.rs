//! Walk a crate's module tree from `src/lib.rs`, following `mod x;`
//! declarations the way rustc does, so each fn gets the module path nextest
//! puts in its test name (`base::vendor::tests::buy_decrements_stack`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::items::{items, FnItem};
use super::lex::lex;

/// A fn with its full test-name path, in a crate's lib target.
#[derive(Debug, Clone)]
pub(crate) struct LibFn {
    pub(crate) file: PathBuf,
    /// `module::path::fn_name`, as nextest names a test in the lib binary.
    pub(crate) test_name: String,
    pub(crate) item: FnItem,
}

#[derive(Debug, Default)]
pub(crate) struct CrateScan {
    pub(crate) fns: Vec<LibFn>,
    /// Every file reached from `src/lib.rs`.
    pub(crate) files: BTreeSet<PathBuf>,
    /// `(file, line)` of gate calls outside any fn body.
    pub(crate) stray_gates: Vec<(PathBuf, usize)>,
}

/// Scan the lib target rooted at `lib_rs`.
pub(crate) fn scan_lib(lib_rs: &Path) -> CrateScan {
    let mut scan = CrateScan::default();
    walk(lib_rs, &[], true, &mut scan);
    scan
}

/// `mod_rs`: the file owns its directory (`lib.rs`, `mod.rs`, or a file
/// loaded through `#[path]`), so its child modules sit next to it rather
/// than in a directory named after it.
fn walk(file: &Path, mod_path: &[String], mod_rs: bool, scan: &mut CrateScan) {
    // Canonical, so a `#[path = "../x.rs"]` file and the same file found by
    // a directory walk compare equal.
    let Ok(file) = file.canonicalize() else {
        return;
    };
    let file = file.as_path();
    if !scan.files.insert(file.to_path_buf()) {
        return;
    }
    let Ok(src) = std::fs::read_to_string(file) else {
        return;
    };
    let found = items(&lex(&src));
    let dir = file.parent().expect("a source file has a parent directory");
    let own_dir = if mod_rs {
        dir.to_path_buf()
    } else {
        dir.join(file.file_stem().expect("a .rs file has a stem"))
    };

    for line in found.stray_gate_lines {
        scan.stray_gates.push((file.to_path_buf(), line));
    }
    for item in found.fns {
        let mut parts: Vec<&str> = mod_path.iter().map(String::as_str).collect();
        parts.extend(item.inline_mods.iter().map(String::as_str));
        parts.push(&item.name);
        scan.fns.push(LibFn {
            file: file.to_path_buf(),
            test_name: parts.join("::"),
            item,
        });
    }
    for m in found.mods {
        let mut child_path = mod_path.to_vec();
        child_path.extend(m.inline_mods.iter().cloned());
        child_path.push(m.name.clone());
        let base = m.inline_mods.iter().fold(own_dir.clone(), |d, s| d.join(s));
        if let Some(p) = &m.path_attr {
            // Outside inline blocks a `#[path]` is relative to the declaring
            // file's directory; inside them, to the inline modules' directory.
            let target = if m.inline_mods.is_empty() {
                dir.join(p)
            } else {
                base.join(p)
            };
            walk(&target, &child_path, true, scan);
        } else if base.join(format!("{}.rs", m.name)).is_file() {
            walk(
                &base.join(format!("{}.rs", m.name)),
                &child_path,
                false,
                scan,
            );
        } else {
            walk(&base.join(&m.name).join("mod.rs"), &child_path, true, scan);
        }
    }
}
