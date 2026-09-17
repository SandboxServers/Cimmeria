//! The static [`COMMANDS`] table — the Rust analogue of the legacy
//! `Command.add([...])` table plus the FanMMORPG `path_*` additions.
//!
//! Every name here is reachable from [`crate::cell::console::exec`];
//! `tests::every_spec_is_dispatched` asserts no entry falls through to the
//! "not implemented" arm.
//!
//! # Layout
//!
//! One module per command family, named after the `console/` module that
//! *implements* that family — so the registry row for `.speed` sits beside
//! the rows for the other stat commands, in the file whose name matches the
//! handler. The families are the same category seams the single flat table
//! carried as section comments before it was split.
//!
//! Each module exports a `SPECS` slice; [`COMMANDS`] is their concatenation,
//! flattened at compile time so the exported type stays exactly
//! `&'static [Spec]` and every consumer (`.iter()`, `.len()`, `for spec in
//! COMMANDS`) is untouched by the split.

use super::{spec, Spec, Target};

mod entity_authoring;
mod maintenance;
mod meta;
mod net_debug;
mod patrol;
mod progression;
mod query;
mod spawn;
mod stats;
mod travel;

/// The per-family tables, in the order they appeared in the flat table (and
/// therefore the order `.help` lists them).
const GROUPS: &[&[Spec]] = &[
    meta::SPECS,
    query::SPECS,
    stats::SPECS,
    entity_authoring::SPECS,
    net_debug::SPECS,
    progression::SPECS,
    travel::SPECS,
    maintenance::SPECS,
    spawn::SPECS,
    patrol::SPECS,
];

/// Total registered commands across every family.
const TOTAL: usize = {
    let mut total = 0;
    let mut g = 0;
    while g < GROUPS.len() {
        total += GROUPS[g].len();
        g += 1;
    }
    total
};

/// Fill value for the accumulator in [`flatten`]. Never observable: every
/// slot is overwritten before the array is returned (the loop runs exactly
/// [`TOTAL`] times by construction).
const PLACEHOLDER: Spec = spec("", 0, 0, Target::None, "");

/// Concatenate [`GROUPS`] into one flat array at compile time.
///
/// Splitting the table into per-family modules must not change what the rest
/// of the console sees, and Rust has no way to splice one slice literal into
/// another. Doing the concatenation in a `const fn` keeps [`COMMANDS`] a
/// plain `&'static [Spec]` — no lazy init, no allocation, no call-site churn.
const fn flatten() -> [Spec; TOTAL] {
    let mut out = [PLACEHOLDER; TOTAL];
    let mut written = 0;
    let mut g = 0;
    while g < GROUPS.len() {
        let group = GROUPS[g];
        let mut i = 0;
        while i < group.len() {
            out[written] = group[i];
            written += 1;
            i += 1;
        }
        g += 1;
    }
    out
}

static FLATTENED: [Spec; TOTAL] = flatten();

pub(crate) static COMMANDS: &[Spec] = &FLATTENED;

#[cfg(test)]
mod tests {
    use super::*;

    /// The compile-time concatenation must preserve every family's rows, in
    /// order. A `flatten` that dropped or duplicated a group would otherwise
    /// only surface as a mysteriously missing `.help` entry.
    #[test]
    fn flatten_preserves_every_group_in_order() {
        let mut expected: Vec<&str> = Vec::new();
        for group in GROUPS {
            expected.extend(group.iter().map(|s| s.name));
        }
        let actual: Vec<&str> = COMMANDS.iter().map(|s| s.name).collect();
        assert_eq!(
            actual, expected,
            "COMMANDS must be the per-family tables concatenated in GROUPS order"
        );
        assert_eq!(COMMANDS.len(), TOTAL);
    }

    /// Command names are the dispatch key, so a duplicate would make one of
    /// the two rows unreachable — easy to introduce when the table is spread
    /// over ten files, impossible to spot by reading any one of them.
    #[test]
    fn command_names_are_unique_across_families() {
        let mut seen = std::collections::HashSet::new();
        for spec in COMMANDS {
            assert!(
                seen.insert(spec.name),
                "duplicate console command name across registry families: {}",
                spec.name
            );
        }
    }
}
