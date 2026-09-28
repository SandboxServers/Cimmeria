//! Special-ammo effect scripts, packet AM-04 (ammo campaign, issue #1026):
//! the damage framework: read the shooter's active `cur_ammo_type`,
//! look up `SpaceManager::ammo_catalog.modifier(ammo_type)` and apply it to
//! the shot (D-AM07). Hollow Point and Armor Piercing are its first rows.
//!
//! Created empty by AM-F so each family packet adds its script here and one
//! `match` arm in `registry.rs`, and never edits `effects/mod.rs`.
