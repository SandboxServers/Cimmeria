//! The reload and item-sequence handlers of SGWPlayer's world-interaction
//! cell methods. The `requestReload` dispatch arm itself stays with the rest
//! of the world dispatcher in `cimmeria-cell-methods`.

pub mod item_sequence;
pub mod reload;

// Re-export discipline: keep every cross-module call site's import path
// identical after the split. `reload`/`item_sequence` items are consumed
// from bandolier, base_messages, ticks, and use_ability via
// `cell_methods::player::world::<item>`.
pub use item_sequence::fire_item_sequence;
pub use reload::{handle_reload, maybe_trigger_reload_on_activate, UNHOLSTER_DRAW_DURATION};
