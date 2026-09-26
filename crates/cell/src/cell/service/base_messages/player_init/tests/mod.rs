//! Tests for the `InitPlayerState` handler and `send_known_abilities_update`
//! helper in [`super`]. Split out of `player_init/mod.rs` to keep the handler
//! file under the size cap; pure test-code move (no logic changes).

#[cfg(test)]
mod reload_on_activate;
#[cfg(test)]
mod relog_mission_resurrection;
#[cfg(test)]
mod state_field_restore;
#[cfg(test)]
mod system_options_assignment;
