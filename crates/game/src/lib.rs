//! # cimmeria-game
//!
//! Game logic for the Cimmeria server emulator. This crate replaces the Python
//! scripting layer with native Rust implementations of player, NPC, mob, combat,
//! inventory, and world systems.
//!
//! Interaction handlers (vendor, lootable, stargate, trainer) live in
//! `cimmeria-services` — see `crates/base-methods/src/base/world_entry/methods/`,
//! `crates/cell-interactions/src/cell/interactions/` and
//! `crates/cell-content/src/cell/ring_transport/`. Vendor,
//! lootable, and stargate stubs that previously lived under
//! `cimmeria-game::interactions::*` were deleted as dead code after audits
//! confirmed zero callers across the workspace. Those stubs were originally
//! created as part of a "real implementation lives here, eventually" pattern
//! that did not pan out — the real handlers landed in `cimmeria-services`
//! instead, and the empty placeholders just produced two parallel surfaces
//! that confused triage. The trainer stub was retired earlier on the same
//! grounds.
//!
//! The `social::*` sketches (`Group`, `Guild`, `Mail`, chat channels) were
//! deleted on the same grounds (#614): nothing imported them, and they read
//! as partial progress on systems whose real code lives elsewhere. Mail and
//! chat are implemented in `cimmeria-services`; groups are not implemented
//! yet.
//!
//! `commands::*` and `missions::*` went the same way (#803). `commands::*`
//! was a pre-#518 slash-command sketch whose handlers never registered
//! anywhere: GM commands are the client's native `/` console (#518) plus the
//! GM-gated `.` console in `crates/cell-console/src/cell/console/` (#523).
//! `missions::*` was a `MissionTracker`/`MissionReward` prototype with
//! `todo!()` persistence: mission state lives in `cimmeria-entity::missions`
//! and `cimmeria-cell-content`'s `cell::missions`, and reward dispatch is
//! #310.

pub mod being;
pub mod combat;
pub mod inventory;
pub mod npc;
pub mod player;
pub mod world;
