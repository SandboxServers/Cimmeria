//! # cimmeria-cell-cover
//!
//! Server-driven NPC cover: the world-space cover markers loaded from
//! `resources.cover_sets` / `resources.cover_nodes`, the per-world spatial
//! index over them, slot reservation, cover scoring and the per-tick
//! hold / release / seek decision, the peek point an NPC looks from, and the
//! player-side proximity detection that feeds the content engine. See
//! [`cell::cover`] and `docs/architecture/cover-system.md`.
//!
//! Split out of `cimmeria-services` (wave W2a of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its old
//! nesting under `cell`, so `crate::cell::cover::…` and `super::…` paths inside
//! it are unchanged, and `cimmeria-services` re-exports the module at its old
//! path, `cimmeria_services::cell::cover`. One file stays behind:
//! `cell/cover/stance.rs` (the spawn hold, Cover Stance and the shared
//! release) needs the effect-script layer and the `SpaceManager`, which sit
//! above this crate, so it lives in `cimmeria-services` at
//! `cimmeria_services::cell::cover::stance` until the world crate takes it.
//!
//! Tracing: the hand-named `cover.*` targets are unchanged; the module-path
//! rows (the loader's counts, the poisoned-mutex warnings) are now
//! `cimmeria_cell_cover::…`, which `cimmeria-server`'s OTLP filter names.

#![warn(unreachable_pub)]

pub mod cell {
    pub mod cover;
}

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), so the
// moved tests keep importing them from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
