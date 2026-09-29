//! Ammo-type switch returns unfired special rounds to the bags (ammo
//! campaign AM-02, issue #1026). Owned by AM-02; created empty by AM-F so
//! AM-02 and AM-03 (`ammo_change.rs`) never edit this directory's `mod.rs`
//! at the same time.
//!
//! AM-02 fills this in: on `requestAmmoChange`, the clip's unfired rounds of
//! the previous type, if special, go back through the base's
//! `inventory::ammo_reserve::return_rounds`; the `remainder` that does not
//! fit stays in the clip (D-AM05). AM-03's handler calls in here.
