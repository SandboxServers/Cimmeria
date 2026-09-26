//! Server-driven NPC cover, at its old path.
//!
//! The cover system lives in the `cimmeria-cell-cover` crate (wave W2a of
//! `docs/architecture/services-crate-split.md`): the loader, the per-world
//! spatial index, reservation, scoring, the per-tick decision, peek points and
//! player detection. This module re-exports all of it, so every
//! `crate::cell::cover::X` path compiles unchanged.
//!
//! [`stance`] stays in this crate: the spawn hold, Cover Stance grant/revoke
//! and the one release every combat-end path calls run through the
//! effect-script layer and read the `SpaceManager`, which sit above the cover
//! crate. The world crate takes this module, re-export and all, when it is
//! extracted (C1).

pub use cimmeria_cell_cover::cell::cover::*;

mod stance;

pub use stance::{
    grant_cover_stance, hold_spawn_cover, hold_spawn_cover_all, release_npc_cover,
    revoke_cover_stance, COVER_STANCE_ABILITY, COVER_STANCE_EFFECT, COVER_STANCE_REMOVE_EFFECT,
};
