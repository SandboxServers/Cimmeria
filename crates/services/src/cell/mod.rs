//! CellApp service.
//!
//! Manages spatial entity simulation, world cells, movement, and Area of
//! Interest calculations. Mirrors the C++ CellApp that partitions the game
//! world into spatial cells and simulates entity interactions within them.
//!
//! Every module of the cell track is in its own crate
//! (`docs/architecture/services-crate-split.md`), and this module re-exports
//! each at its old path:
//!
//! - `cimmeria-cell-world` (wave C1): the world state every cell system
//!   shares (`space_manager`, `arrival`, `cover`, the NPC AI's state
//!   primitives, the ring-transporter state machine, the synchronous effect
//!   scripts, the GM gate, `content_events` and `CellError`).
//! - `cimmeria-cell-combat` (wave C2): `abilities`, `combat`, the effect
//!   pulsing in `effects`, the NPC AI's behaviour, and the bandolier, reload
//!   and item-sequence cell methods.
//! - `cimmeria-cell-content` (wave C3): `content`, `missions`, the ring
//!   dispatcher and entry points in `ring_transport`, and the dialog display.
//! - `cimmeria-cell-interactions` (wave C4): `interactions`, `gate_travel`,
//!   `mail`, the GM space transfer, the respawn fork and the trade session
//!   state.
//! - `cimmeria-cell-console` (wave C5b): `console`, with `chat` (which also
//!   keeps its old `cell::chat` path) and the native `gm*` cell methods.
//! - `cimmeria-cell-methods` (wave C5a): `cell_methods`.
//! - `cimmeria-cell` (wave C6), the top of the track: `CellService` (the cell
//!   loop, the base-message handlers and the ticks) and the cell-method
//!   router, `dispatch`.
//!
//! Nothing else is left here: the module holds only these re-exports.

// The service and the cell-method router, in `cimmeria-cell` (wave C6). The
// facade owns the `CellService` re-export (§2H): the orchestrator builds it.
pub use cimmeria_cell::cell::{dispatch, CellService};

// The client-callable cell methods, in `cimmeria-cell-methods` (wave C5a).
pub use cimmeria_cell_methods::cell::cell_methods;

// The GM surfaces, in `cimmeria-cell-console` (wave C5b): the `.`-console,
// with the chat interceptor and the native `gm*` cell methods under it. Chat
// has been under the console since the services-split preparation for C4-C6
// (§2H) and keeps its old path.
pub use cimmeria_cell_console::cell::console;
pub use console::chat;

// The player interactions, in `cimmeria-cell-interactions` (wave C4).
pub use cimmeria_cell_interactions::cell::{gate_travel, interactions, mail};

// The content layer, in `cimmeria-cell-content` (wave C3).
pub use cimmeria_cell_content::cell::{content, missions, ring_transport};

// Combat, in `cimmeria-cell-combat` (wave C2).
pub use cimmeria_cell_combat::cell::{abilities, combat, effects};

// The world state, in `cimmeria-cell-world` (wave C1), with the seam combat
// raises content events through (§2E) and the cell's error type.
pub use cimmeria_cell_world::cell::{arrival, content_events, cover, space_manager, CellError};

// The spawner's DB loaders, in `cimmeria-cell-catalog` (wave W2b). Populating
// spaces from their records is `space_manager::spawn_npcs_from_records`.
pub use cimmeria_cell_catalog::cell::spawner;

// The Base<->Cell message contract and the server->client method index
// tables, in `cimmeria-wire` (waves W1c and W3a).
pub use cimmeria_wire::cell::{client_methods, messages};

// Not re-exported:
//
// - `space_transfer` (in `cimmeria-cell-interactions`, wave C4):
//   `transfer_player_to_space` is a destructive, unauthenticated entry point
//   (privilege is enforced by the console dispatch layer above it), so it must
//   not be reachable from outside the cell crates, and downstream crates
//   depend only on this one (services-crate-split.md §5.2). Its production
//   callers are the `.goto`/`.summon`/`.gotolocation` console commands in
//   `cell::console::travel` (P46), in `cimmeria-cell-console` since wave C5b.
// - `respawn` and `trade` (`cimmeria-cell-interactions`), `playtest_friction`
//   and `playtest_friction_watch` (`cimmeria-cell-world`), and `kismet` and
//   `player_journal` (`cimmeria-wire`) were re-exported for code here that
//   has since moved to the crates above them; the last of those users, the
//   cell loop and the base-message handlers, went to `cimmeria-cell` in wave
//   C6. `harset_placement_tests` went to `cimmeria-cell-world` then too.
//
// The last two test modules here went to the base crates in wave F:
// `content_tests::mission_701_persistence` (the base's mission query) to
// `cimmeria-base-methods` and `spawner_tests::template_prototype_parity` (the
// GM spawn handler) to `cimmeria-base-session`, both at the same module path.
