//! Reload draws from the special-ammo reserve (ammo campaign AM-02, issue
//! #1026). Owned by AM-02; created empty by AM-F so AM-02 never edits this
//! directory's `mod.rs` alongside another packet.
//!
//! AM-02 fills this in: when the active slot's `cur_ammo_type` is special
//! (`cimmeria_entity::ammo_type::is_special`) and `ammo.finite_special` is on
//! (`cimmeria_entity::ammo_feature::finite_special`), a reload asks the base
//! to draw `clip_size - current_ammo` rounds through the base's
//! `inventory::ammo_reserve::draw` and refills to `current_ammo + drawn`
//! (D-AM05).
