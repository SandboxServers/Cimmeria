//! Process-wide interning of the names log lines carry next to an ID.
//!
//! # Why intern
//!
//! [`PlayerIdentity`](crate::cell_entity::PlayerIdentity) is `Copy` and is
//! stored inside a few dozen other `Copy` structs (hit records, effect plans,
//! metric rows). Giving it an owned `String` name would end `Copy` across all
//! of them. An interned `&'static str` keeps it `Copy`, and it is also the
//! shape `tracing` wants: an `Option<&str>` field is left out when `None`.
//!
//! # Why the leak is bounded
//!
//! Each distinct name is leaked once and reused. The names interned are
//! character names, login names and NPC display / template names, all of
//! which come from database rows (characters, accounts, the seed), not from
//! free client text. The set is capped at [`MAX_INTERNED`] distinct names
//! anyway; past the cap a new name resolves to `None` (the field is left off
//! the line) and one warning is logged.
//!
//! See `docs/architecture/instrumentation-discipline.md` §Rule 6.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock};

/// Most distinct names the process will ever leak. Far above any real
/// server's characters + accounts + seed names.
pub const MAX_INTERNED: usize = 65_536;

static NAMES: LazyLock<RwLock<HashSet<&'static str>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));
static CAP_WARNED: AtomicBool = AtomicBool::new(false);

/// The interned copy of `name`, or `None` when `name` is blank (a name is
/// never `""` on a log line) or the cap is reached.
///
/// A read lock serves names already seen, so the steady state never takes
/// the write lock. A poisoned lock resolves to `None`: an observability
/// helper must never panic the path it describes.
pub fn intern(name: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        return None;
    }
    if let Some(&hit) = NAMES.read().ok()?.get(name) {
        return Some(hit);
    }
    let mut names = NAMES.write().ok()?;
    if let Some(&hit) = names.get(name) {
        return Some(hit);
    }
    if names.len() >= MAX_INTERNED {
        if !CAP_WARNED.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                target: "names",
                event = "names.intern_full",
                reason = "cap_reached",
                cap = MAX_INTERNED,
                "name interner is full: new names are left off log lines"
            );
        }
        return None;
    }
    let leaked: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.insert(leaked);
    Some(leaked)
}

/// [`intern`] over an optional name.
pub fn intern_opt(name: Option<&str>) -> Option<&'static str> {
    name.and_then(intern)
}

#[cfg(test)]
mod tests {
    use super::intern;

    #[test]
    fn same_name_interns_to_the_same_pointer() {
        let a = intern("Teal'c").unwrap();
        let b = intern(&String::from("Teal'c")).unwrap();
        assert!(
            std::ptr::eq(a, b),
            "a name must be leaked once, not per call"
        );
    }

    #[test]
    fn blank_names_are_unresolved() {
        assert_eq!(intern(""), None);
        assert_eq!(intern("   "), None);
    }
}
